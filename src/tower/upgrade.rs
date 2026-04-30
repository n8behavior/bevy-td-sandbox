use std::marker::PhantomData;
use std::time::Duration;

use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;

use crate::audio::{GameSound, PlaySound};
use crate::common::constants::{GridConfig, TILE_SIZE};
use crate::grid::components::GridCell;
use crate::grid::systems::world_to_grid;
use crate::pile::resources::PileScrap;
use crate::shader::{CircleMaterial, CircleMesh};
use crate::stats::resources::RunStats;

use crate::common::constants::{REPAIR_COST_FRAC, REPAIR_RUBBLE_COST_FRAC, TOWER_HP_TIER_MULT};

use super::chain_lightning::components::ChainLightning;
use super::components::*;
use super::placement::{SELL_REFUND_PERCENT, SelectedTower, SellText};
use super::targeting::TargetingButton;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub(crate) const DAMAGE_MULT: [f32; 3] = [1.0, 1.4, 2.0];
pub(crate) const RANGE_MULT: [f32; 3] = [1.0, 1.1, 1.2];
pub(crate) const COOLDOWN_MULT: [f32; 3] = [1.0, 0.85, 0.7];
pub(crate) const AOE_RADIUS_MULT: [f32; 3] = [1.0, 1.15, 1.3];
pub(crate) const SLOW_MULT: [f32; 3] = [1.0, 0.85, 0.7];
pub(crate) const ARC_RANGE_MULT: [f32; 3] = [1.0, 1.333, 1.667];

const TIER_COLOR_BOOST: [f32; 3] = [0.0, 0.15, 0.3];

const MAGNET_RANGE_MULT: [f32; 4] = [1.0, 1.5, 2.0, 2.5];

const LABEL_COLOR: Color = Color::srgb(0.95, 0.85, 0.5);
const HINT_COLOR: Color = Color::srgb(0.7, 0.65, 0.5);
const STAT_COLOR: Color = Color::srgb(0.6, 0.6, 0.55);

// ---------------------------------------------------------------------------
// Generic upgrade-track machinery
//
// Towers can declare any number of independent upgrade dimensions. Each
// dimension is a zero-sized type implementing `UpgradeKind`. The generic
// `apply_track_upgrade::<K>` system handles the input-driven flow (key
// press, scrap debit, tier bump, message emission, floating-text feedback);
// per-capability scaler systems listen on `UpgradeApplied<K>` and apply
// their domain-specific scaling.
// ---------------------------------------------------------------------------

/// Declares an independent upgrade dimension on a tower (e.g. the "Magnet"
/// track that scales scrap-collection range). Each implementor provides a
/// hotkey, a tier ceiling, presentation hints, and a cost function.
pub trait UpgradeKind: Send + Sync + 'static {
    /// Hotkey that gates `apply_track_upgrade::<Self>`.
    const KEY: KeyCode;
    /// Highest tier value the track can reach (inclusive).
    const MAX_TIER: u8;
    /// Human-readable label used by the floating-text feedback.
    const LABEL: &'static str;
    /// Color of the floating-text feedback shown on upgrade.
    const FLOAT_COLOR: Color;

    /// Scrap cost for upgrading FROM `current_tier` to `current_tier + 1`.
    /// Receives the tower's base cost for kinds whose cost scales with the
    /// tower's original price (e.g. the primary tier). Fixed-cost kinds
    /// (e.g. Magnet) ignore `base_cost`.
    fn cost(current_tier: u8, base_cost: u32) -> u32;

    /// Floating-text label shown after an upgrade is applied. The default
    /// renders `"{LABEL} {new_tier}"`. Kinds that prefer a 1-indexed display
    /// (like the primary tier) can override.
    fn float_text(new_tier: u8) -> String {
        format!("{} {}", Self::LABEL, new_tier)
    }
}

/// A typed upgrade-tier counter for the dimension `K`. Towers carry one
/// `UpgradeTrack<K>` per dimension they participate in. The shared
/// `apply_track_upgrade::<K>` system increments the counter and emits
/// `UpgradeApplied<K>` on each upgrade.
#[derive(Component, Debug)]
pub struct UpgradeTrack<K: UpgradeKind> {
    pub tier: u8,
    _marker: PhantomData<K>,
}

impl<K: UpgradeKind> Default for UpgradeTrack<K> {
    fn default() -> Self {
        Self {
            tier: 0,
            _marker: PhantomData,
        }
    }
}

/// Broadcast once per upgrade applied on the `K` track. Per-capability
/// scalers listen on this message and apply domain-specific scaling using
/// the `old_tier` / `new_tier` ratio against their own multiplier table.
///
/// **Idempotency contract:** callers (i.e. `apply_track_upgrade::<K>`)
/// must emit exactly once per tier transition. Per-capability scalers
/// apply ratio math (`live *= MULT[new] / MULT[old]`) and are NOT
/// idempotent — emitting the same message twice double-applies the
/// scaling.
#[derive(Message, Debug)]
pub struct UpgradeApplied<K: UpgradeKind> {
    pub tower: Entity,
    pub old_tier: u8,
    pub new_tier: u8,
    _marker: PhantomData<K>,
}

impl<K: UpgradeKind> UpgradeApplied<K> {
    pub fn new(tower: Entity, old_tier: u8, new_tier: u8) -> Self {
        Self {
            tower,
            old_tier,
            new_tier,
            _marker: PhantomData,
        }
    }
}

/// Generic system: gates on `K::KEY`, validates the inspected tower, debits
/// scrap, bumps `UpgradeTrack<K>.tier`, and emits `UpgradeApplied<K>`.
/// Spawns a floating-text feedback node using `K::float_text` and
/// `K::FLOAT_COLOR`.
pub fn apply_track_upgrade<K: UpgradeKind>(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    inspected: Res<InspectedTower>,
    mut towers: Query<
        (
            Entity,
            &mut UpgradeTrack<K>,
            &mut TowerCost,
            &BaseCost,
            &Transform,
            &TowerState,
        ),
        With<Tower>,
    >,
    mut pile_scrap: ResMut<PileScrap>,
    mut run_stats: Option<ResMut<RunStats>>,
    mut writer: MessageWriter<UpgradeApplied<K>>,
) {
    if !keyboard.just_pressed(K::KEY) {
        return;
    }
    let Some(entity) = inspected.0 else { return };
    let Ok((tower_entity, mut track, mut cost, base_cost, transform, tower_state)) =
        towers.get_mut(entity)
    else {
        return;
    };

    if !tower_state.is_operational() {
        return;
    }

    if track.tier >= K::MAX_TIER {
        return;
    }

    let ucost = K::cost(track.tier, base_cost.0);
    if pile_scrap.amount < ucost {
        return;
    }

    pile_scrap.amount -= ucost;
    cost.0 += ucost;
    if let Some(run_stats) = run_stats.as_mut() {
        run_stats.scrap_spent += ucost;
    }
    let old_tier = track.tier;
    track.tier += 1;
    let new_tier = track.tier;

    writer.write(UpgradeApplied::<K>::new(tower_entity, old_tier, new_tier));

    let pos = transform.translation.truncate();
    commands.spawn((
        Text2d::new(K::float_text(new_tier)),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(K::FLOAT_COLOR),
        Transform::from_translation(pos.extend(10.0)),
        SellText {
            timer: Timer::from_seconds(0.8, TimerMode::Once),
        },
    ));
}

// ---------------------------------------------------------------------------
// Panel section registry
//
// The upgrade panel is composed of a fixed shared header (tower name + stats)
// followed by a sorted list of `PanelSection`s. Each section owns its own
// queries/lookups via `&mut World` access and renders zero or more text rows
// as children of the panel entity. Per-tower modules can register sections
// for their own interactive controls without touching the shared panel
// system.
// ---------------------------------------------------------------------------

/// One contributor to the upgrade panel. Sections render in ascending order
/// of `order`. The render function receives full world access at sync time
/// (queued via `Commands::queue`) so it can read any components on the
/// `tower` entity and spawn UI rows as children of the `panel` entity.
#[derive(Clone, Copy)]
pub struct PanelSection {
    pub order: i32,
    pub render: fn(world: &mut World, panel: Entity, tower: Entity, pile_scrap: u32),
}

/// Resource-backed list of panel sections. Per-tower (or per-capability)
/// plugins push their `PanelSection` on Startup; the shared
/// `update_upgrade_panel` system iterates the list at render time.
#[derive(Resource, Default)]
pub struct PanelSections {
    sections: Vec<PanelSection>,
}

impl PanelSections {
    /// Add a section, keeping the list sorted by `order` (lower runs first
    /// = appears higher in the panel).
    pub fn add(&mut self, section: PanelSection) {
        self.sections.push(section);
        self.sections.sort_by_key(|s| s.order);
    }

    /// Iterate sections in render order.
    pub fn iter(&self) -> impl Iterator<Item = &PanelSection> {
        self.sections.iter()
    }
}

// ---------------------------------------------------------------------------
// Built-in upgrade kinds
// ---------------------------------------------------------------------------

/// The primary combat-tier upgrade dimension (key U). Cost scales with the
/// tower's base cost via a per-tier multiplier table. Damage, range,
/// cooldown, AOE, slow factor, and HP all scale via per-capability scalers
/// listening on `UpgradeApplied<Primary>`.
#[derive(Debug)]
pub struct Primary;

impl UpgradeKind for Primary {
    const KEY: KeyCode = KeyCode::KeyU;
    const MAX_TIER: u8 = 2;
    const LABEL: &'static str = "Tier";
    const FLOAT_COLOR: Color = Color::srgb(0.9, 0.8, 0.2);

    fn cost(current_tier: u8, base_cost: u32) -> u32 {
        const PRIMARY_UPGRADE_COST_MULT: [f32; 2] = [1.0, 1.5];
        (base_cost as f32 * PRIMARY_UPGRADE_COST_MULT[current_tier as usize]) as u32
    }

    fn float_text(new_tier: u8) -> String {
        // Primary tier is displayed 1-indexed in the panel header, so the
        // floating-text feedback uses the same convention.
        format!("Tier {}", new_tier + 1)
    }
}

/// The scrap-collector upgrade dimension. Towers with `UpgradeTrack<Magnet>`
/// can spend scrap (M key) to expand their collection radius.
#[derive(Debug)]
pub struct Magnet;

impl UpgradeKind for Magnet {
    const KEY: KeyCode = KeyCode::KeyM;
    const MAX_TIER: u8 = 3;
    const LABEL: &'static str = "Magnet";
    const FLOAT_COLOR: Color = Color::srgb(0.4, 0.7, 1.0);

    fn cost(current_tier: u8, _base_cost: u32) -> u32 {
        const MAGNET_UPGRADE_COSTS: [u32; 3] = [25, 50, 75];
        MAGNET_UPGRADE_COSTS[current_tier as usize]
    }
}

/// Per-capability scaler for the Magnet track: scales `ScrapCollector.range`
/// using the magnet-tier ratio and refreshes the collection-aura ring.
pub fn scale_magnet_on_track(
    mut events: MessageReader<UpgradeApplied<Magnet>>,
    mut commands: Commands,
    mut towers: Query<(
        &mut ScrapCollector,
        &Children,
        Option<&CollectionAuraRingConfig>,
    )>,
    magnet_auras: Query<Entity, With<MagnetAura>>,
) {
    for ev in events.read() {
        let Ok((mut collector, children, magnet_aura)) = towers.get_mut(ev.tower) else {
            continue;
        };
        let ratio =
            MAGNET_RANGE_MULT[ev.new_tier as usize] / MAGNET_RANGE_MULT[ev.old_tier as usize];
        collector.range *= ratio;

        for child in children.iter() {
            if magnet_auras.contains(child) {
                commands.entity(child).despawn();
            }
        }

        let color = magnet_aura
            .map(|c| c.color)
            .unwrap_or(crate::common::constants::MAGNET_AURA_COLOR);
        let new_range = collector.range;
        let mut ecmds = commands.entity(ev.tower);
        ecmds.remove::<CollectionAuraRingConfig>();
        ecmds.insert(CollectionAuraRingConfig {
            range: new_range,
            color,
        });
    }
}

// ---------------------------------------------------------------------------
// Resources
// ---------------------------------------------------------------------------

/// Tracks which placed tower the player is currently inspecting.
#[derive(Resource, Default)]
pub struct InspectedTower(pub Option<Entity>);

// ---------------------------------------------------------------------------
// UI marker
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct UpgradePanel;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub fn tier_color(base: Color, tier: u8) -> Color {
    let Srgba {
        red,
        green,
        blue,
        alpha,
    } = Srgba::from(base);
    let boost = TIER_COLOR_BOOST[tier as usize];
    Color::srgba(
        (red + boost).min(1.0),
        (green + boost).min(1.0),
        (blue + boost).min(1.0),
        alpha,
    )
}

/// Compute tower sprite color based on health degradation + tier.
pub fn degradation_color(base: Color, tier: u8, health: &TowerHealth) -> Color {
    use crate::common::constants::RUBBLE_TOWER_COLOR;
    let eff = health.effectiveness();
    let tc = tier_color(base, tier);
    if eff >= 1.0 {
        tc
    } else if eff >= 0.75 {
        darken(tc, 0.15)
    } else if eff > 0.0 {
        darken(tc, 0.35)
    } else {
        RUBBLE_TOWER_COLOR
    }
}

fn darken(color: Color, amount: f32) -> Color {
    let Srgba {
        red,
        green,
        blue,
        alpha,
    } = Srgba::from(color);
    Color::srgba(
        (red - amount).max(0.0),
        (green - amount).max(0.0),
        (blue - amount).max(0.0),
        alpha,
    )
}

/// Scrap refund when selling a tower (60% of total investment).
pub(crate) fn sell_refund(total_cost: u32) -> u32 {
    total_cost * SELL_REFUND_PERCENT / 100
}

/// Scrap cost to fully repair a tower. Rubble towers cost more.
pub(crate) fn repair_cost(base_cost: u32, is_rubble: bool) -> u32 {
    let frac = if is_rubble {
        REPAIR_RUBBLE_COST_FRAC
    } else {
        REPAIR_COST_FRAC
    };
    (base_cost as f32 * frac) as u32
}

// ---------------------------------------------------------------------------
// Per-capability tier scalers
//
// Each scaler reads `UpgradeApplied<Primary>` and mutates only the capability it owns,
// using ratio math against its own multiplier table. Towers without the
// capability are absent from the per-scaler query and skipped automatically.
// ---------------------------------------------------------------------------

/// Compute the ratio between two tiers in the same multiplier table.
fn tier_ratio(mults: &[f32], old_tier: u8, new_tier: u8) -> f32 {
    mults[new_tier as usize] / mults[old_tier as usize]
}

/// Scale a `Turret`'s damage, range, and cooldown for the new tier.
pub fn scale_turret_on_tier(
    mut events: MessageReader<UpgradeApplied<Primary>>,
    mut towers: Query<&mut Turret>,
) {
    for ev in events.read() {
        let Ok(mut turret) = towers.get_mut(ev.tower) else {
            continue;
        };
        turret.damage.0 *= tier_ratio(&DAMAGE_MULT, ev.old_tier, ev.new_tier);
        turret.range.0 *= tier_ratio(&RANGE_MULT, ev.old_tier, ev.new_tier);
        let cur_secs = turret.cooldown.0.duration().as_secs_f32();
        let new_secs = cur_secs * tier_ratio(&COOLDOWN_MULT, ev.old_tier, ev.new_tier);
        turret
            .cooldown
            .0
            .set_duration(Duration::from_secs_f32(new_secs));
    }
}

/// Scale an `AoEOnHit`'s blast radius and explosion damage for the new tier.
pub fn scale_aoe_on_tier(
    mut events: MessageReader<UpgradeApplied<Primary>>,
    mut towers: Query<&mut AoEOnHit>,
) {
    for ev in events.read() {
        let Ok(mut aoe) = towers.get_mut(ev.tower) else {
            continue;
        };
        aoe.radius *= tier_ratio(&AOE_RADIUS_MULT, ev.old_tier, ev.new_tier);
        aoe.damage *= tier_ratio(&DAMAGE_MULT, ev.old_tier, ev.new_tier);
    }
}

/// Scale a `SlowOnHit`'s aura range and slow factor for the new tier.
pub fn scale_slow_on_tier(
    mut events: MessageReader<UpgradeApplied<Primary>>,
    mut towers: Query<&mut SlowOnHit>,
) {
    for ev in events.read() {
        let Ok(mut slow) = towers.get_mut(ev.tower) else {
            continue;
        };
        slow.range.0 *= tier_ratio(&RANGE_MULT, ev.old_tier, ev.new_tier);
        slow.factor *= tier_ratio(&SLOW_MULT, ev.old_tier, ev.new_tier);
    }
}

/// Scale `TowerHealth.max` for the new tier while preserving the current
/// damage fraction (a tower at 50% HP stays at 50% HP after upgrading).
pub fn scale_health_on_tier(
    mut events: MessageReader<UpgradeApplied<Primary>>,
    mut towers: Query<&mut TowerHealth>,
) {
    for ev in events.read() {
        let Ok(mut health) = towers.get_mut(ev.tower) else {
            continue;
        };
        let frac = health.fraction();
        health.max *= tier_ratio(&TOWER_HP_TIER_MULT, ev.old_tier, ev.new_tier);
        health.current = health.max * frac;
    }
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Left-click a placed tower (when not in placement mode) to inspect it.
/// Escape clears the inspection. Targeting mode is changed via inline buttons
/// in the upgrade panel — no world-space radial menu.
pub fn inspect_tower(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    towers: Query<(Entity, &GridCell, &TowerState), With<Tower>>,
    selected: Res<SelectedTower>,
    mut inspected: ResMut<InspectedTower>,
    config: Res<GridConfig>,
) {
    if keyboard.just_pressed(KeyCode::Escape) && selected.index.is_none() {
        inspected.0 = None;
        return;
    }

    // Entering placement mode clears inspection.
    if selected.is_changed() && selected.index.is_some() {
        inspected.0 = None;
    }

    // Left-click when no tower selected for placement.
    if !mouse.just_pressed(MouseButton::Left) || selected.index.is_some() {
        return;
    }

    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_transform)) = cameras.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else {
        return;
    };
    let Some(grid_pos) = world_to_grid(world_pos, &config) else {
        inspected.0 = None;
        return;
    };

    if let Some((entity, _, _)) = towers
        .iter()
        .find(|(_, gc, s)| s.is_placed() && gc.coord == grid_pos)
    {
        inspected.0 = Some(entity);
    } else {
        inspected.0 = None;
    }
}

/// Per-capability scaler that handles the visual side of a primary-tier
/// upgrade: refreshes the range/aura ring config (so visuals scale with
/// the new range) and triggers the white-flash sprite tween into the new
/// tier color. Stat scaling lives in the other `scale_*_on_tier` systems.
#[allow(clippy::type_complexity)]
pub fn scale_primary_visuals_on_upgrade(
    mut events: MessageReader<UpgradeApplied<Primary>>,
    mut commands: Commands,
    mut towers: Query<(
        &TowerColor,
        &Children,
        &mut Sprite,
        Option<&RangeRingConfig>,
        Option<&SlowAuraRingConfig>,
    )>,
    range_rings: Query<Entity, With<RangeRing>>,
    aura_visuals: Query<Entity, With<AuraVisual>>,
) {
    for ev in events.read() {
        let Ok((tower_color, children, mut sprite, range_ring, aura_ring)) =
            towers.get_mut(ev.tower)
        else {
            continue;
        };

        // Despawn old ring children before re-inserting configs.
        for child in children.iter() {
            if range_rings.contains(child) || aura_visuals.contains(child) {
                commands.entity(child).despawn();
            }
        }

        let range_ratio = tier_ratio(&RANGE_MULT, ev.old_tier, ev.new_tier);
        let mut ecmds = commands.entity(ev.tower);
        if let Some(rr) = range_ring {
            ecmds.remove::<RangeRingConfig>();
            ecmds.insert(RangeRingConfig {
                range: rr.range * range_ratio,
                color: rr.color,
            });
        }
        if let Some(ar) = aura_ring {
            ecmds.remove::<SlowAuraRingConfig>();
            ecmds.insert(SlowAuraRingConfig {
                range: ar.range * range_ratio,
                color: ar.color,
            });
        }

        // Visual feedback: white flash tweening into the new tier color.
        let new_color = tier_color(tower_color.0, ev.new_tier);
        sprite.color = Color::WHITE;
        ecmds.insert(UpgradeFlash {
            timer: Timer::from_seconds(0.15, TimerMode::Once),
            target_color: new_color,
        });
    }
}

/// When a tower WITHOUT `UpgradeTrack<Magnet>` (i.e. ScrapMagnet) gets a
/// primary-tier upgrade, sync its `ScrapCollector.range` to the new aura
/// range stored on `SlowOnHit.range` — ScrapMagnet's collector and slow
/// aura share a radius. Towers WITH `UpgradeTrack<Magnet>` manage collection
/// range via `scale_magnet_on_track`.
pub fn sync_collector_on_upgrade(
    mut towers: Query<
        (&SlowOnHit, &mut ScrapCollector, &TowerState),
        (
            With<Tower>,
            Without<UpgradeTrack<Magnet>>,
            Changed<UpgradeTrack<Primary>>,
        ),
    >,
) {
    for (slow, mut collector, tower_state) in &mut towers {
        if !tower_state.is_placed() {
            continue;
        }
        collector.range = slow.range.0;
    }
}

/// Press R to repair the inspected damaged tower.
#[allow(clippy::type_complexity)]
pub fn apply_repair(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    inspected: Res<InspectedTower>,
    mut towers: Query<
        (
            Entity,
            &mut TowerHealth,
            &BaseCost,
            &TowerColor,
            &UpgradeTrack<Primary>,
            &mut Sprite,
            &Transform,
            &Children,
            &mut TowerState,
            Option<&RangeRingConfig>,
            Option<&SlowAuraRingConfig>,
            Option<&CollectionAuraRingConfig>,
        ),
        With<Tower>,
    >,
    range_rings: Query<Entity, With<RangeRing>>,
    aura_visuals: Query<Entity, With<AuraVisual>>,
    magnet_auras: Query<Entity, With<MagnetAura>>,
    mut pile_scrap: ResMut<PileScrap>,
    mut run_stats: Option<ResMut<RunStats>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyR) {
        return;
    }
    let Some(entity) = inspected.0 else { return };
    let Ok((
        entity,
        mut health,
        base_cost,
        tower_color,
        tier,
        mut sprite,
        transform,
        children,
        mut tower_state,
        range_ring,
        aura_ring,
        magnet_aura,
    )) = towers.get_mut(entity)
    else {
        return;
    };

    if !tower_state.is_placed() {
        return;
    }

    // No-op if already at full HP.
    if health.current >= health.max {
        return;
    }

    let is_rubble = *tower_state == TowerState::Rubble;
    let repair_cost = repair_cost(base_cost.0, is_rubble);

    if pile_scrap.amount < repair_cost {
        return;
    }

    pile_scrap.amount -= repair_cost;
    if let Some(run_stats) = run_stats.as_mut() {
        run_stats.scrap_spent += repair_cost;
    }

    health.current = health.max;

    if is_rubble {
        *tower_state = TowerState::Active;

        // Despawn any stale ring children that survived.
        for child in children.iter() {
            if range_rings.contains(child)
                || aura_visuals.contains(child)
                || magnet_auras.contains(child)
            {
                commands.entity(child).despawn();
            }
        }

        // Re-insert ring configs to trigger Added<> reactive spawning.
        let mut ecmds = commands.entity(entity);
        if let Some(rr) = range_ring {
            let rr_new = RangeRingConfig {
                range: rr.range,
                color: rr.color,
            };
            ecmds.remove::<RangeRingConfig>();
            ecmds.insert(rr_new);
        }
        if let Some(ar) = aura_ring {
            let ar_new = SlowAuraRingConfig {
                range: ar.range,
                color: ar.color,
            };
            ecmds.remove::<SlowAuraRingConfig>();
            ecmds.insert(ar_new);
        }
        if let Some(ma) = magnet_aura {
            let ma_new = CollectionAuraRingConfig {
                range: ma.range,
                color: ma.color,
            };
            ecmds.remove::<CollectionAuraRingConfig>();
            ecmds.insert(ma_new);
        }
    }

    // Visual: white flash restoring to healthy tier color.
    let healthy_color = tier_color(tower_color.0, tier.tier);
    sprite.color = Color::WHITE;
    commands.entity(entity).insert(UpgradeFlash {
        timer: Timer::from_seconds(0.15, TimerMode::Once),
        target_color: healthy_color,
    });

    commands.trigger(PlaySound(GameSound::TowerRepaired));

    let pos = transform.translation.truncate();
    commands.spawn((
        Text2d::new("Repaired!"),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.3, 0.9, 0.3)),
        Transform::from_translation(pos.extend(10.0)),
        SellText {
            timer: Timer::from_seconds(0.8, TimerMode::Once),
        },
    ));
}

/// Animate the upgrade flash: hold white, then restore target color.
pub fn animate_upgrade_flash(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Sprite, &mut UpgradeFlash)>,
) {
    for (entity, mut sprite, mut flash) in &mut query {
        flash.timer.tick(time.delta());
        if flash.timer.is_finished() {
            sprite.color = flash.target_color;
            commands.entity(entity).remove::<UpgradeFlash>();
        }
    }
}

// ---------------------------------------------------------------------------
// Selection ring
// ---------------------------------------------------------------------------

/// Spawn / despawn the selection ring when InspectedTower changes.
pub fn manage_selection_ring(
    mut commands: Commands,
    inspected: Res<InspectedTower>,
    existing_rings: Query<Entity, With<SelectionRing>>,
    circle_mesh: Res<CircleMesh>,
    mut materials: ResMut<Assets<CircleMaterial>>,
) {
    if !inspected.is_changed() {
        return;
    }

    // Despawn old ring(s).
    for ring in &existing_rings {
        commands.entity(ring).despawn();
    }

    // Spawn new ring on inspected tower.
    let Some(entity) = inspected.0 else { return };
    let mat = materials.add(CircleMaterial {
        color: Color::srgba(1.0, 1.0, 1.0, 0.35),
        softness: 0.15,
        fill_fade: 0.0,
        ripple_speed: 0.6,
        time: 0.0,
    });
    commands.entity(entity).with_child((
        SelectionRing,
        Mesh2d(circle_mesh.0.clone()),
        MeshMaterial2d(mat),
        Transform::from_translation(Vec3::new(0.0, 0.0, -0.05))
            .with_scale(Vec3::splat(TILE_SIZE + 6.0)),
    ));
}

// ---------------------------------------------------------------------------
// Upgrade panel UI
// ---------------------------------------------------------------------------

pub fn setup_upgrade_panel(mut commands: Commands) {
    commands.spawn((
        crate::ui::tower_menu::panel_node(crate::ui::tower_menu::PanelAnchor::BottomRight),
        Visibility::Hidden,
        UpgradePanel,
    ));
}

// ---------------------------------------------------------------------------
// PanelStats writers
// ---------------------------------------------------------------------------

/// HP color thresholds: green > 50%, yellow > 25%, red otherwise; dark red for rubble.
fn hp_color(state: &TowerState, health: &TowerHealth) -> Color {
    if *state == TowerState::Rubble {
        Color::srgb(0.5, 0.2, 0.2)
    } else if health.fraction() > 0.5 {
        Color::srgb(0.3, 0.8, 0.3)
    } else if health.fraction() > 0.25 {
        Color::srgb(0.9, 0.8, 0.2)
    } else {
        Color::srgb(0.9, 0.3, 0.3)
    }
}

/// Reactively repopulate `PanelStats.common` and `PanelStats.next_tier` for any
/// tower whose stats changed. Tower-type-agnostic: handles HP, DMG, RNG, FIRE
/// RATE (turret), AOE, SLOW, COLLECT — everything driven by shared components.
/// Per-tower extras (e.g. Chain Lightning's ARC) are written by per-tower
/// systems into `PanelStats.extra`.
///
/// DMG and RNG are sourced from whichever capability owns them: `Turret` for
/// projectile turrets, `ChainLightning` for chain towers, or `SlowOnHit` for
/// pure aura towers (which only have a range).
#[allow(clippy::type_complexity)]
pub fn rebuild_common_stats(
    mut towers: Query<
        (
            &mut PanelStats,
            &UpgradeTrack<Primary>,
            Option<&Turret>,
            Option<&ChainLightning>,
            Option<&AoEOnHit>,
            Option<&SlowOnHit>,
            Option<&TowerHealth>,
            Option<&ScrapCollector>,
            &TowerState,
        ),
        (
            With<Tower>,
            Or<(
                Changed<Turret>,
                Changed<ChainLightning>,
                Changed<UpgradeTrack<Primary>>,
                Changed<TowerHealth>,
                Changed<AoEOnHit>,
                Changed<SlowOnHit>,
                Changed<ScrapCollector>,
                Added<PanelStats>,
            )>,
        ),
    >,
) {
    for (mut panel, tier, turret, chain, aoe, slow, health, collector, tower_state) in &mut towers {
        panel.common.clear();
        panel.next_tier_common.clear();

        if let Some(h) = health {
            let value = if *tower_state == TowerState::Rubble {
                "RUBBLE".to_string()
            } else {
                format!("{:.0}/{:.0}", h.current, h.max)
            };
            panel.common.push(StatLine {
                label: "HP",
                value,
                color: hp_color(tower_state, h),
            });
        }

        // DMG comes from whichever firing capability is present.
        if let Some(t) = turret {
            panel.common.push(StatLine {
                label: "DMG",
                value: format!("{:.0}", t.damage.0),
                color: STAT_COLOR,
            });
        } else if let Some(c) = chain {
            panel.common.push(StatLine {
                label: "DMG",
                value: format!("{:.0}", c.damage.0),
                color: STAT_COLOR,
            });
        }

        // RNG comes from the primary-range field of whichever capability owns
        // it. Pure aura towers (TarPit, ScrapMagnet) advertise their slow
        // aura's range as RNG.
        if let Some(t) = turret {
            panel.common.push(StatLine {
                label: "RNG",
                value: format!("{:.0}", t.range.0),
                color: STAT_COLOR,
            });
        } else if let Some(c) = chain {
            panel.common.push(StatLine {
                label: "RNG",
                value: format!("{:.0}", c.primary_range.0),
                color: STAT_COLOR,
            });
        } else if let Some(s) = slow {
            panel.common.push(StatLine {
                label: "RNG",
                value: format!("{:.0}", s.range.0),
                color: STAT_COLOR,
            });
        }

        if let Some(t) = turret {
            panel.common.push(StatLine {
                label: "FIRE RATE",
                value: format!("{:.2}s", t.cooldown.0.duration().as_secs_f32()),
                color: STAT_COLOR,
            });
        }

        if let Some(a) = aoe {
            panel.common.push(StatLine {
                label: "AOE",
                value: format!("{:.0} radius", a.radius),
                color: STAT_COLOR,
            });
        }

        if let Some(s) = slow {
            panel.common.push(StatLine {
                label: "SLOW",
                value: format!("{:.0}%", (1.0 - s.factor) * 100.0),
                color: STAT_COLOR,
            });
        }

        if let Some(c) = collector {
            panel.common.push(StatLine {
                label: "COLLECT",
                value: format!("{:.0}", c.range),
                color: STAT_COLOR,
            });
        }

        if tier.tier < Primary::MAX_TIER {
            let cur = tier.tier;
            let next = cur + 1;
            let dmg_ratio = tier_ratio(&DAMAGE_MULT, cur, next);
            let rng_ratio = tier_ratio(&RANGE_MULT, cur, next);
            let aoe_ratio = tier_ratio(&AOE_RADIUS_MULT, cur, next);

            // Only preview DMG for towers that actually deal damage. Compute
            // the next-tier value as a ratio of the current capability value.
            if let Some(t) = turret {
                panel.next_tier_common.push(StatLine {
                    label: "DMG",
                    value: format!("{:.0}", t.damage.0 * dmg_ratio),
                    color: STAT_COLOR,
                });
            } else if let Some(c) = chain {
                panel.next_tier_common.push(StatLine {
                    label: "DMG",
                    value: format!("{:.0}", c.damage.0 * dmg_ratio),
                    color: STAT_COLOR,
                });
            }
            // RNG preview applies to any tower with a range capability.
            if let Some(t) = turret {
                panel.next_tier_common.push(StatLine {
                    label: "RNG",
                    value: format!("{:.0}", t.range.0 * rng_ratio),
                    color: STAT_COLOR,
                });
            } else if let Some(c) = chain {
                panel.next_tier_common.push(StatLine {
                    label: "RNG",
                    value: format!("{:.0}", c.primary_range.0 * rng_ratio),
                    color: STAT_COLOR,
                });
            } else if let Some(s) = slow {
                panel.next_tier_common.push(StatLine {
                    label: "RNG",
                    value: format!("{:.0}", s.range.0 * rng_ratio),
                    color: STAT_COLOR,
                });
            }
            if let Some(a) = aoe
                && a.radius > 0.0
            {
                panel.next_tier_common.push(StatLine {
                    label: "AOE",
                    value: format!("{:.0}", a.radius * aoe_ratio),
                    color: STAT_COLOR,
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Upgrade panel render
// ---------------------------------------------------------------------------

/// Spawn a single text line as a panel child.
fn spawn_text_line(
    parent: &mut ChildSpawnerCommands,
    text: impl Into<String>,
    color: Color,
    font_size: f32,
) {
    parent.spawn((
        Text::new(text.into()),
        TextColor(color),
        TextFont {
            font_size,
            ..default()
        },
    ));
}

/// Format a stat line as `LABEL: value`.
fn format_stat(line: &StatLine) -> String {
    format!("{}: {}", line.label, line.value)
}

// ---------------------------------------------------------------------------
// Default panel sections
//
// Sections receive `&mut World` access (via `Commands::queue`) so they can
// look up arbitrary components on the inspected tower without forcing the
// shared panel system to query them. Each section spawns its UI as direct
// child entities of the panel, using the world helpers below.
// ---------------------------------------------------------------------------

/// Spawn a text node and attach it as a child of `panel`. World-based
/// equivalent of `spawn_text_line` for use inside `Commands::queue` closures.
fn spawn_text_line_world(
    world: &mut World,
    panel: Entity,
    text: impl Into<String>,
    color: Color,
    font_size: f32,
) {
    let child = world
        .spawn((
            Text::new(text.into()),
            TextColor(color),
            TextFont {
                font_size,
                ..default()
            },
        ))
        .id();
    world.entity_mut(panel).add_child(child);
}

/// Spawn the targeting-button row (C/H/L/F) as a child of `panel`.
/// World-based equivalent of `spawn_targeting_buttons`.
fn spawn_targeting_buttons_world(
    world: &mut World,
    panel: Entity,
    tower: Entity,
    current_mode: TargetingMode,
) {
    let row = world
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(4.0),
            margin: UiRect::top(Val::Px(2.0)),
            ..default()
        })
        .id();

    for &mode in &TargetingMode::ALL {
        let bg = if mode == current_mode {
            Color::srgba(0.3, 0.7, 0.3, 0.8)
        } else {
            Color::srgba(0.2, 0.2, 0.2, 0.7)
        };
        let btn = world
            .spawn((
                Button,
                Node {
                    width: Val::Px(28.0),
                    height: Val::Px(22.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
                BackgroundColor(bg),
                TargetingButton { tower, mode },
            ))
            .id();
        let label = world
            .spawn((
                Text::new(mode.label()),
                TextColor(Color::WHITE),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ))
            .id();
        world.entity_mut(btn).add_child(label);
        world.entity_mut(row).add_child(btn);
    }
    world.entity_mut(panel).add_child(row);
}

/// Targeting-mode label + button row. Renders only on towers with a
/// `TargetingMode` component.
fn render_targeting_section(world: &mut World, panel: Entity, tower: Entity, _pile_scrap: u32) {
    let Some(mode) = world.entity(tower).get::<TargetingMode>().copied() else {
        return;
    };
    spawn_text_line_world(
        world,
        panel,
        format!("TARGET: {}", mode.name()),
        STAT_COLOR,
        13.0,
    );
    spawn_targeting_buttons_world(world, panel, tower, mode);
}

/// Magnet upgrade button. Towers with `UpgradeTrack<Magnet>` show the next
/// upgrade cost; towers with a `ScrapCollector` but no track show MAX.
fn render_magnet_section(world: &mut World, panel: Entity, tower: Entity, pile_scrap: u32) {
    let entity_ref = world.entity(tower);
    let track_tier = entity_ref.get::<UpgradeTrack<Magnet>>().map(|t| t.tier);
    let has_collector = entity_ref.contains::<ScrapCollector>();
    let base_cost = entity_ref.get::<BaseCost>().map(|c| c.0).unwrap_or(0);

    let line = if let Some(tier) = track_tier {
        if tier < Magnet::MAX_TIER {
            let mcost = Magnet::cost(tier, base_cost);
            let mcost_color = if pile_scrap >= mcost {
                LABEL_COLOR
            } else {
                Color::srgb(0.9, 0.3, 0.3)
            };
            Some((format!("[M] Magnet: ${mcost}"), mcost_color))
        } else {
            Some(("MAGNET: MAX".to_string(), Color::srgb(0.4, 0.7, 1.0)))
        }
    } else if has_collector {
        Some(("MAGNET: MAX".to_string(), Color::srgb(0.4, 0.7, 1.0)))
    } else {
        None
    };

    if let Some((text, color)) = line {
        spawn_text_line_world(world, panel, text, color, 13.0);
    }
}

/// Repair button. Renders only on towers with a `TowerHealth` that is below
/// max. Cost depends on whether the tower is rubble or merely damaged.
fn render_repair_section(world: &mut World, panel: Entity, tower: Entity, pile_scrap: u32) {
    let entity_ref = world.entity(tower);
    let Some(health) = entity_ref.get::<TowerHealth>() else {
        return;
    };
    if health.current >= health.max {
        return;
    }
    let Some(base_cost) = entity_ref.get::<BaseCost>().map(|c| c.0) else {
        return;
    };
    let Some(tower_state) = entity_ref.get::<TowerState>().copied() else {
        return;
    };

    let is_rubble = tower_state == TowerState::Rubble;
    let rcost = repair_cost(base_cost, is_rubble);
    let repair_color = if pile_scrap >= rcost {
        LABEL_COLOR
    } else {
        Color::srgb(0.9, 0.3, 0.3)
    };
    spawn_text_line_world(
        world,
        panel,
        format!("[R] Repair: ${rcost}"),
        repair_color,
        13.0,
    );
}

/// Sell hint, shown on every placed tower. Reads `TowerCost` for the refund
/// amount.
fn render_sell_section(world: &mut World, panel: Entity, tower: Entity, _pile_scrap: u32) {
    let Some(cost) = world.entity(tower).get::<TowerCost>().map(|c| c.0) else {
        return;
    };
    let refund = sell_refund(cost);
    spawn_text_line_world(
        world,
        panel,
        format!("[RMB] Sell: +${refund}"),
        HINT_COLOR,
        11.0,
    );
}

/// Register the four built-in panel sections (targeting, magnet, repair,
/// sell). Step 7 of issue #81 keeps these in shared code; future steps can
/// move them into per-tower plugins.
pub fn register_default_panel_sections(mut sections: ResMut<PanelSections>) {
    sections.add(PanelSection {
        order: 100,
        render: render_targeting_section,
    });
    sections.add(PanelSection {
        order: 200,
        render: render_magnet_section,
    });
    sections.add(PanelSection {
        order: 300,
        render: render_repair_section,
    });
    sections.add(PanelSection {
        order: 400,
        render: render_sell_section,
    });
}

/// Rebuild the upgrade panel contents when the inspected tower changes.
///
/// Renders the shared header + `PanelStats` (common + extra stat lines) plus
/// the hardcoded primary-tier upgrade button. All other interactive controls
/// are contributed via `PanelSections`; sections render at sync time with
/// full world access so they can query their own components.
#[allow(clippy::type_complexity)]
pub fn update_upgrade_panel(
    mut commands: Commands,
    inspected: Res<InspectedTower>,
    towers: Query<
        (
            &TowerName,
            &UpgradeTrack<Primary>,
            &BaseCost,
            &PanelStats,
            &TowerState,
        ),
        With<Tower>,
    >,
    panel_stats_check: Query<(), (With<Tower>, Changed<PanelStats>)>,
    mut panel_query: Query<(Entity, &mut Visibility), With<UpgradePanel>>,
    pile_scrap: Res<PileScrap>,
) {
    let Ok((panel_entity, mut vis)) = panel_query.single_mut() else {
        return;
    };

    // Rebuild when something relevant changes.
    let panel_stats_changed = inspected.0.is_some_and(|e| panel_stats_check.contains(e));
    if !panel_stats_changed && !inspected.is_changed() && !pile_scrap.is_changed() {
        return;
    }

    let Some(entity) = inspected.0 else {
        *vis = Visibility::Hidden;
        return;
    };

    let Ok((name, tier, base_cost, panel, tower_state)) = towers.get(entity) else {
        *vis = Visibility::Hidden;
        return;
    };

    if !tower_state.is_placed() {
        *vis = Visibility::Hidden;
        return;
    }

    *vis = Visibility::Inherited;

    commands.entity(panel_entity).despawn_related::<Children>();

    // Capture the data the deferred section pass will need.
    let pile_amount = pile_scrap.amount;
    let tower = entity;

    commands.entity(panel_entity).with_children(|parent| {
        // Header
        spawn_text_line(
            parent,
            format!(
                "== {} (Tier {}/{}) ==",
                name.0,
                tier.tier + 1,
                Primary::MAX_TIER + 1
            ),
            LABEL_COLOR,
            15.0,
        );

        // Current stats: common (shared) + extra (per-tower).
        for line in panel.common.iter().chain(panel.extra.iter()) {
            spawn_text_line(parent, format_stat(line), line.color, 13.0);
        }

        // Upgrade button / next-tier preview / max-tier banner. Stays
        // hardcoded for now; step 9 will migrate it to a panel section.
        if *tower_state == TowerState::Rubble {
            spawn_text_line(
                parent,
                "\nREPAIR REQUIRED",
                Color::srgb(0.9, 0.3, 0.3),
                13.0,
            );
        } else if tier.tier < Primary::MAX_TIER {
            let preview = panel
                .next_tier_common
                .iter()
                .chain(panel.next_tier_extra.iter())
                .map(|l| format!("{} {}", l.label, l.value))
                .collect::<Vec<_>>()
                .join("  ");
            if !preview.is_empty() {
                spawn_text_line(
                    parent,
                    format!("\nNext: {preview}"),
                    Color::srgb(0.8, 0.75, 0.4),
                    13.0,
                );
            }

            let ucost = Primary::cost(tier.tier, base_cost.0);
            let cost_color = if pile_amount >= ucost {
                LABEL_COLOR
            } else {
                Color::srgb(0.9, 0.3, 0.3)
            };
            spawn_text_line(parent, format!("[U] Upgrade: ${ucost}"), cost_color, 13.0);
        } else {
            spawn_text_line(parent, "\nMAX TIER", Color::srgb(0.4, 0.9, 0.4), 13.0);
        }
    });

    // Run registered panel sections at the next sync point. Sections see
    // full world access so they can query their own components without
    // forcing the shared system to do so. The ESC hint is appended last.
    commands.queue(move |world: &mut World| {
        let sections: Vec<PanelSection> = world.resource::<PanelSections>().sections.clone();
        for section in &sections {
            (section.render)(world, panel_entity, tower, pile_amount);
        }
        spawn_text_line_world(world, panel_entity, "[ESC] Close", HINT_COLOR, 11.0);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Primary::cost --

    #[test]
    fn primary_cost_tier_0() {
        // Tier 0→1: base_cost * 1.0
        assert_eq!(Primary::cost(0, 100), 100);
    }

    #[test]
    fn primary_cost_tier_1() {
        // Tier 1→2: base_cost * 1.5
        assert_eq!(Primary::cost(1, 100), 150);
    }

    // -- per-capability scalers (UpgradeApplied<Primary>) --

    fn scaler_test_app() -> App {
        let mut app = App::new();
        app.add_message::<UpgradeApplied<Primary>>();
        app
    }

    #[test]
    fn scale_turret_applies_tier_ratio() {
        let mut app = scaler_test_app();
        app.add_systems(Update, scale_turret_on_tier);

        let entity = app
            .world_mut()
            .spawn(Turret::new(Damage(10.0), Range(100.0), 1.0, 0.1))
            .id();

        app.world_mut()
            .write_message(UpgradeApplied::<Primary>::new(entity, 0, 1));
        app.update();

        let turret = app.world().get::<Turret>(entity).unwrap();
        // Tier 0→1: damage *1.4, range *1.1, cooldown *0.85.
        assert!((turret.damage.0 - 14.0).abs() < 1e-4);
        assert!((turret.range.0 - 110.0).abs() < 1e-4);
        assert!(
            (turret.cooldown.0.duration().as_secs_f32() - 0.85).abs() < 1e-4,
            "cooldown was {:?}",
            turret.cooldown.0.duration()
        );
    }

    #[test]
    fn scalers_are_not_idempotent() {
        // The scalers apply ratio math (live *= MULT[new]/MULT[old]) and are
        // intentionally NOT idempotent. Emitting the same `UpgradeApplied`
        // twice double-applies the scaling. This is a contract: callers
        // (i.e. `apply_track_upgrade`) must emit exactly once per tier
        // transition.
        let mut app = scaler_test_app();
        app.add_systems(Update, scale_turret_on_tier);

        let entity = app
            .world_mut()
            .spawn(Turret::new(Damage(10.0), Range(100.0), 1.0, 0.1))
            .id();

        // Two identical messages → ratio applied twice.
        app.world_mut()
            .write_message(UpgradeApplied::<Primary>::new(entity, 0, 1));
        app.world_mut()
            .write_message(UpgradeApplied::<Primary>::new(entity, 0, 1));
        app.update();

        let turret = app.world().get::<Turret>(entity).unwrap();
        // 10.0 * 1.4 * 1.4 = 19.6, NOT 14.0 (the idempotent answer).
        assert!(
            (turret.damage.0 - 19.6).abs() < 1e-3,
            "expected double-application 19.6, got {}",
            turret.damage.0
        );
    }

    #[test]
    fn scale_health_preserves_fraction() {
        let mut app = scaler_test_app();
        app.add_systems(Update, scale_health_on_tier);

        // Tower at 50% HP before upgrade.
        let entity = app
            .world_mut()
            .spawn(TowerHealth {
                current: 50.0,
                max: 100.0,
            })
            .id();

        app.world_mut()
            .write_message(UpgradeApplied::<Primary>::new(entity, 0, 1));
        app.update();

        let h = app.world().get::<TowerHealth>(entity).unwrap();
        // Tier 0→1: max *1.4 = 140.0; fraction preserved at 50% → current = 70.0.
        assert!((h.max - 140.0).abs() < 1e-4);
        assert!((h.current - 70.0).abs() < 1e-4);
        assert!((h.fraction() - 0.5).abs() < 1e-4);
    }

    // -- sell_refund --

    #[test]
    fn sell_refund_sixty_percent() {
        assert_eq!(sell_refund(100), 60);
        assert_eq!(sell_refund(150), 90);
    }

    #[test]
    fn sell_refund_zero_cost() {
        assert_eq!(sell_refund(0), 0);
    }

    // -- repair_cost --

    #[test]
    fn repair_cost_damaged() {
        // REPAIR_COST_FRAC = 0.3
        assert_eq!(repair_cost(100, false), 30);
    }

    #[test]
    fn repair_cost_rubble() {
        // REPAIR_RUBBLE_COST_FRAC = 0.5
        assert_eq!(repair_cost(100, true), 50);
    }

    // -- tier_color --

    #[test]
    fn tier_color_base_unchanged() {
        let base = Color::srgb(0.5, 0.5, 0.5);
        let result = tier_color(base, 0);
        let Srgba {
            red, green, blue, ..
        } = Srgba::from(result);
        assert!((red - 0.5).abs() < 0.001);
        assert!((green - 0.5).abs() < 0.001);
        assert!((blue - 0.5).abs() < 0.001);
    }

    #[test]
    fn tier_color_clamped_at_one() {
        let bright = Color::srgb(0.95, 0.95, 0.95);
        let result = tier_color(bright, 2); // boost = 0.3
        let Srgba {
            red, green, blue, ..
        } = Srgba::from(result);
        assert_eq!(red, 1.0);
        assert_eq!(green, 1.0);
        assert_eq!(blue, 1.0);
    }

    // -- degradation_color --

    #[test]
    fn degradation_full_health_returns_tier_color() {
        let base = Color::srgb(0.5, 0.5, 0.5);
        let health = TowerHealth {
            current: 100.0,
            max: 100.0,
        };
        let result = degradation_color(base, 0, &health);
        let expected = tier_color(base, 0);
        assert_eq!(result, expected);
    }

    #[test]
    fn degradation_zero_health_returns_rubble() {
        use crate::common::constants::RUBBLE_TOWER_COLOR;
        let health = TowerHealth {
            current: 0.0,
            max: 100.0,
        };
        let result = degradation_color(Color::WHITE, 0, &health);
        assert_eq!(result, RUBBLE_TOWER_COLOR);
    }
}
