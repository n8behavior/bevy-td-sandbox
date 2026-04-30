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

use crate::common::constants::{
    REPAIR_COST_FRAC, REPAIR_RUBBLE_COST_FRAC, TOWER_HP_COST_MULT, TOWER_HP_TIER_MULT,
};

use super::chain_lightning::components::ChainLightning;
use super::components::*;
use super::placement::{SELL_REFUND_PERCENT, SelectedTower, SellText};
use super::targeting::TargetingButton;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const MAX_TIER: u8 = 2;

const UPGRADE_COST_MULT: [f32; 2] = [1.0, 1.5];
const DAMAGE_MULT: [f32; 3] = [1.0, 1.4, 2.0];
const RANGE_MULT: [f32; 3] = [1.0, 1.1, 1.2];
const COOLDOWN_MULT: [f32; 3] = [1.0, 0.85, 0.7];
const AOE_RADIUS_MULT: [f32; 3] = [1.0, 1.15, 1.3];
const SLOW_MULT: [f32; 3] = [1.0, 0.85, 0.7];
const ARC_RANGE_MULT: [f32; 3] = [1.0, 1.333, 1.667];

const TIER_COLOR_BOOST: [f32; 3] = [0.0, 0.15, 0.3];

pub const MAX_MAGNET_TIER: u8 = 3;
const MAGNET_UPGRADE_COSTS: [u32; 3] = [25, 50, 75];
const MAGNET_RANGE_MULT: [f32; 4] = [1.0, 1.5, 2.0, 2.5];

const LABEL_COLOR: Color = Color::srgb(0.95, 0.85, 0.5);
const HINT_COLOR: Color = Color::srgb(0.7, 0.65, 0.5);
const STAT_COLOR: Color = Color::srgb(0.6, 0.6, 0.55);

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

/// Scrap cost to upgrade from `current_tier` to the next tier.
pub(crate) fn upgrade_cost(base_cost: u32, current_tier: u8) -> u32 {
    (base_cost as f32 * UPGRADE_COST_MULT[current_tier as usize]) as u32
}

/// Tower damage at the given tier (base scaled by tier multiplier).
pub(crate) fn damage_at_tier(base_damage: f32, tier: u8) -> f32 {
    base_damage * DAMAGE_MULT[tier as usize]
}

/// Tower range at the given tier.
pub(crate) fn range_at_tier(base_range: f32, tier: u8) -> f32 {
    base_range * RANGE_MULT[tier as usize]
}

/// Fire cooldown at the given tier (lower = faster).
pub(crate) fn cooldown_at_tier(base_cooldown: f32, tier: u8) -> f32 {
    base_cooldown * COOLDOWN_MULT[tier as usize]
}

/// AOE blast radius at the given tier.
pub(crate) fn aoe_radius_at_tier(base_radius: f32, tier: u8) -> f32 {
    base_radius * AOE_RADIUS_MULT[tier as usize]
}

/// Slow factor at the given tier (lower = stronger slow).
pub(crate) fn slow_factor_at_tier(base_factor: f32, tier: u8) -> f32 {
    base_factor * SLOW_MULT[tier as usize]
}

/// Chain lightning arc range at the given tier.
pub(crate) fn arc_range_at_tier(base_arc: f32, tier: u8) -> f32 {
    base_arc * ARC_RANGE_MULT[tier as usize]
}

/// Maximum tower HP at the given tier, derived from base cost.
pub(crate) fn tower_max_hp(base_cost: u32, tier: u8) -> f32 {
    base_cost as f32 * TOWER_HP_COST_MULT * TOWER_HP_TIER_MULT[tier as usize]
}

/// Scrap collection range at the given magnet tier.
pub(crate) fn magnet_range_at_tier(base_range: f32, tier: u8) -> f32 {
    base_range * MAGNET_RANGE_MULT[tier as usize]
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

/// Press U to upgrade the inspected tower.
#[allow(clippy::type_complexity)]
pub fn apply_upgrade(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    inspected: Res<InspectedTower>,
    mut towers: Query<
        (
            &mut TowerTier,
            &BaseStats,
            &TowerColor,
            &mut TowerCost,
            &mut Sprite,
            &Transform,
            &Children,
            Option<&mut Turret>,
            Option<&mut AoEOnHit>,
            Option<&mut SlowOnHit>,
            Option<&mut ChainLightning>,
            (
                Option<&RangeRingConfig>,
                Option<&SlowAuraRingConfig>,
                Option<&mut TowerHealth>,
            ),
            &TowerState,
        ),
        With<Tower>,
    >,
    range_rings: Query<Entity, With<RangeRing>>,
    aura_visuals: Query<Entity, With<AuraVisual>>,
    mut pile_scrap: ResMut<PileScrap>,
    mut run_stats: Option<ResMut<RunStats>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyU) {
        return;
    }
    let Some(entity) = inspected.0 else { return };
    let Ok((
        mut tier,
        base,
        tower_color,
        mut cost,
        mut sprite,
        transform,
        children,
        turret,
        aoe,
        slow,
        chain,
        (range_ring, aura_ring, tower_health),
        tower_state,
    )) = towers.get_mut(entity)
    else {
        return;
    };

    if !tower_state.is_operational() {
        return;
    }

    if tier.0 >= MAX_TIER {
        return;
    }

    let ucost = upgrade_cost(base.cost, tier.0);
    if pile_scrap.amount < ucost {
        return;
    }

    // Deduct and track.
    pile_scrap.amount -= ucost;
    cost.0 += ucost;

    if let Some(run_stats) = run_stats.as_mut() {
        run_stats.scrap_spent += ucost;
    }
    tier.0 += 1;

    // Scale tower HP with tier (preserve damage fraction).
    if let Some(mut health) = tower_health {
        let old_frac = health.fraction();
        let new_max = tower_max_hp(base.cost, tier.0);
        health.max = new_max;
        health.current = new_max * old_frac;
    }

    // Per-capability tier scaling. Each capability owns the slice of base
    // stats it cares about; non-applicable capabilities are absent from the
    // entity and skipped.
    if let Some(mut turret) = turret {
        turret.damage = Damage(damage_at_tier(base.damage, tier.0));
        turret.range = Range(range_at_tier(base.range, tier.0));
        let new_dur = cooldown_at_tier(base.cooldown_secs, tier.0);
        turret
            .cooldown
            .0
            .set_duration(Duration::from_secs_f32(new_dur));
    }
    if let Some(mut aoe) = aoe {
        aoe.radius = aoe_radius_at_tier(base.aoe_radius, tier.0);
        aoe.damage = damage_at_tier(base.aoe_damage, tier.0);
    }
    if let Some(mut slow) = slow {
        slow.range = Range(range_at_tier(base.range, tier.0));
        slow.factor = slow_factor_at_tier(base.slow_factor, tier.0);
    }
    if let Some(mut chain) = chain {
        chain.damage = Damage(damage_at_tier(base.damage, tier.0));
        chain.primary_range = Range(range_at_tier(base.range, tier.0));
        // Chain Lightning's per-tower scaling (arc_range, ChainCooldown) is
        // handled reactively by `chain_lightning::systems::scale_chain_on_tier_change`.
    }

    // Despawn old ring children before re-inserting configs.
    for child in children.iter() {
        if range_rings.contains(child) || aura_visuals.contains(child) {
            commands.entity(child).despawn();
        }
    }

    // Re-insert ring configs to trigger the Added<> reactive systems. The
    // visual range is recomputed from the per-tower base stat so it tracks
    // whichever capability owns it (the legacy unified TowerStats.range is
    // gone).
    let new_visual_range = range_at_tier(base.range, tier.0);
    let mut ecmds = commands.entity(entity);
    if let Some(rr) = range_ring {
        let new_rr = RangeRingConfig {
            range: new_visual_range,
            color: rr.color,
        };
        ecmds.remove::<RangeRingConfig>();
        ecmds.insert(new_rr);
    }
    if let Some(ar) = aura_ring {
        // Aura ring matches the new aura range (TarPit slow aura, ScrapMagnet
        // pull aura). Both scale via base.range.
        let new_ar = SlowAuraRingConfig {
            range: new_visual_range,
            color: ar.color,
        };
        ecmds.remove::<SlowAuraRingConfig>();
        ecmds.insert(new_ar);
    }

    // Visual feedback: white flash.
    let new_color = tier_color(tower_color.0, tier.0);
    sprite.color = Color::WHITE;
    ecmds.insert(UpgradeFlash {
        timer: Timer::from_seconds(0.15, TimerMode::Once),
        target_color: new_color,
    });

    // Floating text.
    let pos = transform.translation.truncate();
    commands.spawn((
        Text2d::new(format!("Tier {}", tier.0 + 1)),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.8, 0.2)),
        Transform::from_translation(pos.extend(10.0)),
        SellText {
            timer: Timer::from_seconds(0.8, TimerMode::Once),
        },
    ));
}

/// When a tower WITHOUT `MagnetTier` (i.e. ScrapMagnet) gets a stat
/// upgrade, sync its `ScrapCollector.range` to the new aura range stored on
/// `SlowOnHit.range` (ScrapMagnet's collector and slow aura share a radius).
/// Towers WITH `MagnetTier` manage collection range via the magnet system.
pub fn sync_collector_on_upgrade(
    mut towers: Query<
        (&SlowOnHit, &mut ScrapCollector, &TowerState),
        (With<Tower>, Without<MagnetTier>, Changed<TowerTier>),
    >,
) {
    for (slow, mut collector, tower_state) in &mut towers {
        if !tower_state.is_placed() {
            continue;
        }
        collector.range = slow.range.0;
    }
}

/// Press M to upgrade the magnet tier of the inspected tower.
pub fn apply_magnet_upgrade(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    inspected: Res<InspectedTower>,
    mut towers: Query<
        (
            &mut MagnetTier,
            &BaseMagnetRange,
            &mut ScrapCollector,
            &mut TowerCost,
            &Transform,
            &Children,
            Option<&CollectionAuraRingConfig>,
            &TowerState,
        ),
        With<Tower>,
    >,
    magnet_auras: Query<Entity, With<MagnetAura>>,
    mut pile_scrap: ResMut<PileScrap>,
) {
    if !keyboard.just_pressed(KeyCode::KeyM) {
        return;
    }
    let Some(entity) = inspected.0 else { return };
    let Ok((
        mut tier,
        base_range,
        mut collector,
        mut cost,
        transform,
        children,
        magnet_aura,
        tower_state,
    )) = towers.get_mut(entity)
    else {
        return;
    };

    if !tower_state.is_operational() {
        return;
    }

    if tier.0 >= MAX_MAGNET_TIER {
        return;
    }

    let ucost = MAGNET_UPGRADE_COSTS[tier.0 as usize];
    if pile_scrap.amount < ucost {
        return;
    }

    // Deduct and track.
    pile_scrap.amount -= ucost;
    cost.0 += ucost;
    tier.0 += 1;

    // Update collection range.
    collector.range = magnet_range_at_tier(base_range.0, tier.0);

    // Despawn old magnet aura children before re-inserting config.
    for child in children.iter() {
        if magnet_auras.contains(child) {
            commands.entity(child).despawn();
        }
    }

    // Re-insert CollectionAuraRingConfig to trigger the Added<> reactive system.
    let color = magnet_aura
        .map(|c| c.color)
        .unwrap_or(crate::common::constants::MAGNET_AURA_COLOR);
    let mut ecmds = commands.entity(entity);
    ecmds.remove::<CollectionAuraRingConfig>();
    ecmds.insert(CollectionAuraRingConfig {
        range: collector.range,
        color,
    });

    // Floating text feedback.
    let pos = transform.translation.truncate();
    commands.spawn((
        Text2d::new(format!("Magnet {}", tier.0)),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.4, 0.7, 1.0)),
        Transform::from_translation(pos.extend(10.0)),
        SellText {
            timer: Timer::from_seconds(0.8, TimerMode::Once),
        },
    ));
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
            &BaseStats,
            &TowerColor,
            &TowerTier,
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
        base,
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
    let repair_cost = repair_cost(base.cost, is_rubble);

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
    let healthy_color = tier_color(tower_color.0, tier.0);
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
            &TowerTier,
            &BaseStats,
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
                Changed<TowerTier>,
                Changed<TowerHealth>,
                Changed<AoEOnHit>,
                Changed<SlowOnHit>,
                Changed<ScrapCollector>,
                Added<PanelStats>,
            )>,
        ),
    >,
) {
    for (mut panel, tier, base, turret, chain, aoe, slow, health, collector, tower_state) in
        &mut towers
    {
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

        if tier.0 < MAX_TIER {
            let next = tier.0 + 1;
            // Only preview DMG for towers that actually deal damage.
            if turret.is_some() || chain.is_some() {
                panel.next_tier_common.push(StatLine {
                    label: "DMG",
                    value: format!("{:.0}", damage_at_tier(base.damage, next)),
                    color: STAT_COLOR,
                });
            }
            // RNG preview applies to any tower with a range capability.
            if turret.is_some() || chain.is_some() || slow.is_some() {
                panel.next_tier_common.push(StatLine {
                    label: "RNG",
                    value: format!("{:.0}", range_at_tier(base.range, next)),
                    color: STAT_COLOR,
                });
            }
            if aoe.is_some() && base.aoe_radius > 0.0 {
                panel.next_tier_common.push(StatLine {
                    label: "AOE",
                    value: format!("{:.0}", aoe_radius_at_tier(base.aoe_radius, next)),
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

/// Rebuild the upgrade panel contents when the inspected tower changes.
///
/// Renders only from `PanelStats` (current + next-tier stats) plus tower-agnostic
/// interactive buttons (Upgrade, Magnet, Repair) whose color depends on
/// `pile_scrap`. No tower-type-specific component queries.
#[allow(clippy::type_complexity)]
pub fn update_upgrade_panel(
    mut commands: Commands,
    inspected: Res<InspectedTower>,
    towers: Query<
        (
            &TowerName,
            &TowerTier,
            &TowerCost,
            &BaseStats,
            &PanelStats,
            Option<&TargetingMode>,
            Option<&MagnetTier>,
            Option<&ScrapCollector>,
            Option<&TowerHealth>,
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

    let Ok((
        name,
        tier,
        cost,
        base,
        panel,
        targeting_mode,
        magnet_tier,
        collector,
        tower_health,
        tower_state,
    )) = towers.get(entity)
    else {
        *vis = Visibility::Hidden;
        return;
    };

    if !tower_state.is_placed() {
        *vis = Visibility::Hidden;
        return;
    }

    *vis = Visibility::Inherited;

    commands.entity(panel_entity).despawn_related::<Children>();

    commands.entity(panel_entity).with_children(|parent| {
        // Header
        spawn_text_line(
            parent,
            format!("== {} (Tier {}/{}) ==", name.0, tier.0 + 1, MAX_TIER + 1),
            LABEL_COLOR,
            15.0,
        );

        // Current stats: common (shared) + extra (per-tower).
        for line in panel.common.iter().chain(panel.extra.iter()) {
            spawn_text_line(parent, format_stat(line), line.color, 13.0);
        }

        // Targeting mode label + inline buttons (replaces world-space radial menu).
        if let Some(mode) = targeting_mode {
            spawn_text_line(parent, format!("TARGET: {}", mode.name()), STAT_COLOR, 13.0);
            spawn_targeting_buttons(parent, entity, *mode);
        }

        // Upgrade button / next-tier preview / max-tier banner.
        if *tower_state == TowerState::Rubble {
            spawn_text_line(
                parent,
                "\nREPAIR REQUIRED",
                Color::srgb(0.9, 0.3, 0.3),
                13.0,
            );
        } else if tier.0 < MAX_TIER {
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

            let ucost = upgrade_cost(base.cost, tier.0);
            let cost_color = if pile_scrap.amount >= ucost {
                LABEL_COLOR
            } else {
                Color::srgb(0.9, 0.3, 0.3)
            };
            spawn_text_line(parent, format!("[U] Upgrade: ${ucost}"), cost_color, 13.0);
        } else {
            spawn_text_line(parent, "\nMAX TIER", Color::srgb(0.4, 0.9, 0.4), 13.0);
        }

        // Magnet upgrade button: any tower with MagnetTier shows the upgrade
        // button; any tower with a ScrapCollector but no MagnetTier shows MAX.
        if let Some(mt) = magnet_tier {
            if mt.0 < MAX_MAGNET_TIER {
                let mcost = MAGNET_UPGRADE_COSTS[mt.0 as usize];
                let mcost_color = if pile_scrap.amount >= mcost {
                    LABEL_COLOR
                } else {
                    Color::srgb(0.9, 0.3, 0.3)
                };
                spawn_text_line(parent, format!("[M] Magnet: ${mcost}"), mcost_color, 13.0);
            } else {
                spawn_text_line(parent, "MAGNET: MAX", Color::srgb(0.4, 0.7, 1.0), 13.0);
            }
        } else if collector.is_some() {
            spawn_text_line(parent, "MAGNET: MAX", Color::srgb(0.4, 0.7, 1.0), 13.0);
        }

        // Repair button (only when damaged)
        if let Some(health) = tower_health
            && health.current < health.max
        {
            let is_rubble = *tower_state == TowerState::Rubble;
            let rcost = repair_cost(base.cost, is_rubble);
            let repair_color = if pile_scrap.amount >= rcost {
                LABEL_COLOR
            } else {
                Color::srgb(0.9, 0.3, 0.3)
            };
            spawn_text_line(parent, format!("[R] Repair: ${rcost}"), repair_color, 13.0);
        }

        // Sell hint
        spawn_text_line(
            parent,
            format!("[RMB] Sell: +${}", sell_refund(cost.0)),
            HINT_COLOR,
            11.0,
        );

        spawn_text_line(parent, "[ESC] Close", HINT_COLOR, 11.0);
    });
}

/// Spawn one row of 4 targeting-mode buttons (C/H/L/F) inside the panel.
/// Each button carries a `TargetingButton { tower, mode }` so
/// `handle_targeting_button` can apply the chosen mode on click.
fn spawn_targeting_buttons(
    parent: &mut ChildSpawnerCommands,
    tower: Entity,
    current_mode: TargetingMode,
) {
    parent
        .spawn(Node {
            flex_direction: FlexDirection::Row,
            column_gap: Val::Px(4.0),
            margin: UiRect::top(Val::Px(2.0)),
            ..default()
        })
        .with_children(|row| {
            for &mode in &TargetingMode::ALL {
                let bg = if mode == current_mode {
                    Color::srgba(0.3, 0.7, 0.3, 0.8)
                } else {
                    Color::srgba(0.2, 0.2, 0.2, 0.7)
                };
                row.spawn((
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
                .with_children(|btn| {
                    btn.spawn((
                        Text::new(mode.label()),
                        TextColor(Color::WHITE),
                        TextFont {
                            font_size: 13.0,
                            ..default()
                        },
                    ));
                });
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- upgrade_cost --

    #[test]
    fn upgrade_cost_tier_0() {
        // Tier 0→1: base_cost * 1.0
        assert_eq!(upgrade_cost(100, 0), 100);
    }

    #[test]
    fn upgrade_cost_tier_1() {
        // Tier 1→2: base_cost * 1.5
        assert_eq!(upgrade_cost(100, 1), 150);
    }

    // -- damage_at_tier --

    #[test]
    fn damage_progression() {
        let base = 10.0;
        assert_eq!(damage_at_tier(base, 0), 10.0);
        assert_eq!(damage_at_tier(base, 1), 14.0);
        assert_eq!(damage_at_tier(base, 2), 20.0);
    }

    // -- range_at_tier --

    #[test]
    fn range_progression() {
        let base = 100.0;
        assert_eq!(range_at_tier(base, 0), 100.0);
        assert!((range_at_tier(base, 1) - 110.0).abs() < 0.01);
        assert!((range_at_tier(base, 2) - 120.0).abs() < 0.01);
    }

    // -- cooldown_at_tier --

    #[test]
    fn cooldown_decreases_with_tier() {
        let base = 1.0;
        assert_eq!(cooldown_at_tier(base, 0), 1.0);
        assert_eq!(cooldown_at_tier(base, 1), 0.85);
        assert_eq!(cooldown_at_tier(base, 2), 0.7);
    }

    // -- aoe / slow / arc --

    #[test]
    fn aoe_radius_progression() {
        let base = 50.0;
        assert_eq!(aoe_radius_at_tier(base, 0), 50.0);
        assert_eq!(aoe_radius_at_tier(base, 2), 65.0);
    }

    #[test]
    fn slow_factor_decreases_with_tier() {
        let base = 0.5;
        assert_eq!(slow_factor_at_tier(base, 0), 0.5);
        assert!((slow_factor_at_tier(base, 2) - 0.35).abs() < 0.001);
    }

    #[test]
    fn arc_range_progression() {
        let base = 60.0;
        assert_eq!(arc_range_at_tier(base, 0), 60.0);
        assert!((arc_range_at_tier(base, 2) - 100.02).abs() < 0.1);
    }

    // -- tower_max_hp --

    #[test]
    fn tower_max_hp_progression() {
        // cost=50, TOWER_HP_COST_MULT=3.0, TOWER_HP_TIER_MULT=[1.0, 1.4, 2.0]
        assert_eq!(tower_max_hp(50, 0), 150.0);
        assert_eq!(tower_max_hp(50, 1), 210.0);
        assert_eq!(tower_max_hp(50, 2), 300.0);
    }

    // -- magnet_range_at_tier --

    #[test]
    fn magnet_range_progression() {
        let base = 30.0;
        assert_eq!(magnet_range_at_tier(base, 0), 30.0);
        assert_eq!(magnet_range_at_tier(base, 1), 45.0);
        assert_eq!(magnet_range_at_tier(base, 2), 60.0);
        assert_eq!(magnet_range_at_tier(base, 3), 75.0);
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
