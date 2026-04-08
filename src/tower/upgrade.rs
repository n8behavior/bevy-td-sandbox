use std::time::Duration;

use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;

use crate::audio::resources::SoundAssets;
use crate::audio::systems::play_sound;
use crate::common::constants::{GridConfig, TILE_SIZE};
use crate::grid::components::GridCell;
use crate::grid::systems::world_to_grid;
use crate::pile::resources::PileScrap;
use crate::shader::{CircleMaterial, CircleMesh};
use crate::states::GameState;
use crate::stats::resources::RunStats;
use crate::ui::tower_menu::WavePreviewPanel;

use crate::common::constants::{
    REPAIR_COST_FRAC, REPAIR_RUBBLE_COST_FRAC, TOWER_HP_COST_MULT, TOWER_HP_TIER_MULT,
};

use super::components::*;
use super::placement::{SELL_REFUND_PERCENT, SelectedTower, SellText};
use super::targeting::RadialMenuState;
use super::types::scrap_magnet::ScrapMagnet;

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

fn upgrade_cost(base_cost: u32, current_tier: u8) -> u32 {
    (base_cost as f32 * UPGRADE_COST_MULT[current_tier as usize]) as u32
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Left-click a placed tower (when not in placement mode) to inspect it.
/// Escape clears the inspection.
pub fn inspect_tower(
    mouse: Res<ButtonInput<MouseButton>>,
    keyboard: Res<ButtonInput<KeyCode>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    towers: Query<(Entity, &GridCell, Option<&TargetingMode>, &TowerState), With<Tower>>,
    selected: Res<SelectedTower>,
    mut inspected: ResMut<InspectedTower>,
    mut radial_menu: ResMut<RadialMenuState>,
    config: Res<GridConfig>,
) {
    // Escape: close radial menu first, then clear inspection.
    if keyboard.just_pressed(KeyCode::Escape) && selected.index.is_none() {
        if radial_menu.tower.is_some() {
            radial_menu.tower = None;
        } else {
            inspected.0 = None;
        }
        return;
    }

    // Entering placement mode clears inspection and radial menu.
    if selected.is_changed() && selected.index.is_some() {
        inspected.0 = None;
        radial_menu.tower = None;
    }

    // If radial menu was just closed by handle_radial_click, skip click processing.
    if radial_menu.is_changed() {
        return;
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
        radial_menu.tower = None;
        inspected.0 = None;
        return;
    };

    if let Some((entity, _, targeting, _)) = towers
        .iter()
        .find(|(_, gc, _, s)| s.is_placed() && gc.coord == grid_pos)
    {
        if inspected.0 == Some(entity) {
            // Re-click the already-inspected tower: toggle radial menu.
            if radial_menu.tower.is_some() {
                radial_menu.tower = None;
            } else if targeting.is_some() {
                radial_menu.tower = Some(entity);
            }
        } else {
            // New tower: inspect it, close any open radial menu.
            radial_menu.tower = None;
            inspected.0 = Some(entity);
        }
    } else {
        radial_menu.tower = None;
        inspected.0 = None;
    }
}

/// Press U to upgrade the inspected tower.
pub fn apply_upgrade(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    inspected: Res<InspectedTower>,
    mut towers: Query<
        (
            &mut TowerTier,
            &BaseStats,
            &mut TowerStats,
            &mut TowerCost,
            &mut Sprite,
            &Transform,
            &Children,
            Option<&mut TurretState>,
            Option<&mut AoEOnHit>,
            Option<&mut SlowOnHit>,
            (
                Option<&RangeRingConfig>,
                Option<&AuraRingConfig>,
                Option<&mut ChainLightning>,
                Option<&BaseArcRange>,
                Option<&mut ChainCooldown>,
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
        mut stats,
        mut cost,
        mut sprite,
        transform,
        children,
        turret,
        aoe,
        slow,
        (range_ring, aura_ring, chain_lightning, base_arc_range, chain_cooldown, tower_health),
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
    let t = (tier.0 as usize).min(MAX_TIER as usize);

    // Scale tower HP with tier (preserve damage fraction).
    if let Some(mut health) = tower_health {
        let old_frac = health.fraction();
        let new_max = base.cost as f32 * TOWER_HP_COST_MULT * TOWER_HP_TIER_MULT[t];
        health.max = new_max;
        health.current = new_max * old_frac;
    }

    // Apply stat multipliers from base.
    stats.damage = base.damage * DAMAGE_MULT[t];
    stats.range = base.range * RANGE_MULT[t];

    if let Some(mut turret) = turret {
        let new_dur = base.cooldown_secs * COOLDOWN_MULT[t];
        turret
            .cooldown
            .set_duration(Duration::from_secs_f32(new_dur));
    }
    if let Some(mut aoe) = aoe {
        aoe.radius = base.aoe_radius * AOE_RADIUS_MULT[t];
        aoe.damage = base.aoe_damage * DAMAGE_MULT[t];
    }
    if let Some(mut slow) = slow {
        slow.factor = base.slow_factor * SLOW_MULT[t];
    }
    if let Some(mut chain) = chain_lightning
        && let Some(base_arc) = base_arc_range
    {
        chain.arc_range = base_arc.0 * ARC_RANGE_MULT[t];
    }
    if let Some(mut cc) = chain_cooldown {
        let new_dur = base.cooldown_secs * COOLDOWN_MULT[t];
        cc.timer.set_duration(Duration::from_secs_f32(new_dur));
    }

    // Despawn old ring children before re-inserting configs.
    for child in children.iter() {
        if range_rings.contains(child) || aura_visuals.contains(child) {
            commands.entity(child).despawn();
        }
    }

    // Re-insert ring configs to trigger the Added<> reactive systems.
    let mut ecmds = commands.entity(entity);
    if let Some(rr) = range_ring {
        let new_rr = RangeRingConfig {
            range: stats.range,
            color: rr.color,
        };
        ecmds.remove::<RangeRingConfig>();
        ecmds.insert(new_rr);
    }
    if let Some(ar) = aura_ring {
        // Use updated stats.range so the aura visual scales with upgrades
        // (affects TarPit slow aura and ScrapMagnet pull aura).
        let new_ar = AuraRingConfig {
            range: stats.range,
            color: ar.color,
        };
        ecmds.remove::<AuraRingConfig>();
        ecmds.insert(new_ar);
    }

    // Visual feedback: white flash.
    let new_color = tier_color(base.color, tier.0);
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
/// upgrade, sync its `ScrapCollector.range` to the new `TowerStats.range`.
/// Towers WITH `MagnetTier` manage collection range via the magnet system.
pub fn sync_collector_on_upgrade(
    mut towers: Query<
        (&TowerStats, &mut ScrapCollector, &TowerState),
        (With<Tower>, Without<MagnetTier>, Changed<TowerTier>),
    >,
) {
    for (stats, mut collector, tower_state) in &mut towers {
        if !tower_state.is_placed() {
            continue;
        }
        collector.range = stats.range;
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
            Option<&MagnetAuraConfig>,
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
    let t = tier.0 as usize;

    // Update collection range.
    collector.range = base_range.0 * MAGNET_RANGE_MULT[t];

    // Despawn old magnet aura children before re-inserting config.
    for child in children.iter() {
        if magnet_auras.contains(child) {
            commands.entity(child).despawn();
        }
    }

    // Re-insert MagnetAuraConfig to trigger the Added<> reactive system.
    let color = magnet_aura
        .map(|c| c.color)
        .unwrap_or(crate::common::constants::MAGNET_AURA_COLOR);
    let mut ecmds = commands.entity(entity);
    ecmds.remove::<MagnetAuraConfig>();
    ecmds.insert(MagnetAuraConfig {
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
pub fn apply_repair(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    inspected: Res<InspectedTower>,
    mut towers: Query<
        (
            Entity,
            &mut TowerHealth,
            &BaseStats,
            &TowerTier,
            &mut Sprite,
            &Transform,
            &Children,
            &mut TowerState,
            Option<&RangeRingConfig>,
            Option<&AuraRingConfig>,
            Option<&MagnetAuraConfig>,
        ),
        With<Tower>,
    >,
    range_rings: Query<Entity, With<RangeRing>>,
    aura_visuals: Query<Entity, With<AuraVisual>>,
    magnet_auras: Query<Entity, With<MagnetAura>>,
    mut pile_scrap: ResMut<PileScrap>,
    mut run_stats: Option<ResMut<RunStats>>,
    sounds: Res<SoundAssets>,
) {
    if !keyboard.just_pressed(KeyCode::KeyR) {
        return;
    }
    let Some(entity) = inspected.0 else { return };
    let Ok((
        entity,
        mut health,
        base,
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
    let fraction = if is_rubble {
        REPAIR_RUBBLE_COST_FRAC
    } else {
        REPAIR_COST_FRAC
    };
    let repair_cost = (base.cost as f32 * fraction) as u32;

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
            let ar_new = AuraRingConfig {
                range: ar.range,
                color: ar.color,
            };
            ecmds.remove::<AuraRingConfig>();
            ecmds.insert(ar_new);
        }
        if let Some(ma) = magnet_aura {
            let ma_new = MagnetAuraConfig {
                range: ma.range,
                color: ma.color,
            };
            ecmds.remove::<MagnetAuraConfig>();
            ecmds.insert(ma_new);
        }
    }

    // Visual: white flash restoring to healthy tier color.
    let healthy_color = tier_color(base.color, tier.0);
    sprite.color = Color::WHITE;
    commands.entity(entity).insert(UpgradeFlash {
        timer: Timer::from_seconds(0.15, TimerMode::Once),
        target_color: healthy_color,
    });

    play_sound(&mut commands, &sounds.tower_repaired, 0.4);

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
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(10.0),
            right: Val::Px(10.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            padding: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.12, 0.1, 0.08, 0.85)),
        Visibility::Hidden,
        UpgradePanel,
        DespawnOnExit(GameState::Playing),
    ));
}

/// Rebuild the upgrade panel contents when the inspected tower changes.
pub fn update_upgrade_panel(
    mut commands: Commands,
    inspected: Res<InspectedTower>,
    radial_menu: Res<RadialMenuState>,
    towers: Query<
        (
            &TowerName,
            &TowerTier,
            &TowerStats,
            &TowerCost,
            &BaseStats,
            Option<&TurretState>,
            Option<&AoEOnHit>,
            Option<&SlowOnHit>,
            (
                Option<&ChainLightning>,
                Option<&ChainCooldown>,
                Option<&BaseArcRange>,
                Option<&TargetingMode>,
                Option<&MagnetTier>,
                Option<&ScrapCollector>,
                Option<&ScrapMagnet>,
                Option<&TowerHealth>,
            ),
            &TowerState,
        ),
        With<Tower>,
    >,
    tower_health_check: Query<&TowerHealth, (With<Tower>, Changed<TowerHealth>)>,
    mut panel_query: Query<(Entity, &mut Visibility), With<UpgradePanel>>,
    mut wave_preview: Query<&mut Visibility, (With<WavePreviewPanel>, Without<UpgradePanel>)>,
    pile_scrap: Res<PileScrap>,
) {
    let Ok((panel_entity, mut vis)) = panel_query.single_mut() else {
        return;
    };

    // Rebuild when something relevant changes (including tower health during combat).
    let health_changed = inspected.0.is_some_and(|e| tower_health_check.contains(e));
    if !health_changed
        && !inspected.is_changed()
        && !pile_scrap.is_changed()
        && !radial_menu.is_changed()
    {
        return;
    }

    let Some(entity) = inspected.0 else {
        *vis = Visibility::Hidden;
        // Restore wave preview visibility (its own system handles actual show/hide).
        if let Ok(mut wv) = wave_preview.single_mut() {
            *wv = Visibility::Inherited;
        }
        return;
    };

    let Ok((
        name,
        tier,
        stats,
        cost,
        base,
        turret,
        aoe,
        slow,
        (
            chain,
            chain_cd,
            base_arc,
            targeting_mode,
            magnet_tier,
            collector,
            is_scrap_magnet,
            tower_health,
        ),
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

    // Hide wave preview while upgrade panel is open.
    if let Ok(mut wv) = wave_preview.single_mut() {
        *wv = Visibility::Hidden;
    }

    commands.entity(panel_entity).despawn_related::<Children>();

    commands.entity(panel_entity).with_children(|parent| {
        // Header
        parent.spawn((
            Text::new(format!(
                "== {} (Tier {}/{}) ==",
                name.0,
                tier.0 + 1,
                MAX_TIER + 1
            )),
            TextColor(LABEL_COLOR),
            TextFont {
                font_size: 15.0,
                ..default()
            },
        ));

        // Tower HP (only shown for towers with health)
        if let Some(health) = tower_health {
            let is_rubble = *tower_state == TowerState::Rubble;
            let hp_text = if is_rubble {
                "HP: RUBBLE".to_string()
            } else {
                format!("HP: {:.0}/{:.0}", health.current, health.max)
            };
            let hp_color = if is_rubble {
                Color::srgb(0.5, 0.2, 0.2)
            } else if health.fraction() > 0.5 {
                Color::srgb(0.3, 0.8, 0.3)
            } else if health.fraction() > 0.25 {
                Color::srgb(0.9, 0.8, 0.2)
            } else {
                Color::srgb(0.9, 0.3, 0.3)
            };
            parent.spawn((
                Text::new(hp_text),
                TextColor(hp_color),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        }

        // Current stats
        parent.spawn((
            Text::new(format!("DMG: {:.0}  RNG: {:.0}", stats.damage, stats.range)),
            TextColor(STAT_COLOR),
            TextFont {
                font_size: 13.0,
                ..default()
            },
        ));

        if let Some(turret) = turret {
            parent.spawn((
                Text::new(format!(
                    "FIRE RATE: {:.2}s",
                    turret.cooldown.duration().as_secs_f32()
                )),
                TextColor(STAT_COLOR),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        }

        if let Some(aoe) = aoe {
            parent.spawn((
                Text::new(format!("AOE: {:.0} radius", aoe.radius)),
                TextColor(STAT_COLOR),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        }

        if let Some(slow) = slow {
            parent.spawn((
                Text::new(format!("SLOW: {:.0}%", (1.0 - slow.factor) * 100.0)),
                TextColor(STAT_COLOR),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        }

        if let Some(chain) = chain {
            parent.spawn((
                Text::new(format!("ARC: {:.0}", chain.arc_range)),
                TextColor(STAT_COLOR),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        }

        if let Some(cc) = chain_cd {
            parent.spawn((
                Text::new(format!(
                    "FIRE RATE: {:.2}s",
                    cc.timer.duration().as_secs_f32()
                )),
                TextColor(STAT_COLOR),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        }

        if let Some(mode) = targeting_mode {
            parent.spawn((
                Text::new(format!("TARGET: {}", mode.name())),
                TextColor(STAT_COLOR),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        }

        // Upgrade or max tier (hidden when rubble — must repair first)
        if *tower_state == TowerState::Rubble {
            parent.spawn((
                Text::new("\nREPAIR REQUIRED"),
                TextColor(Color::srgb(0.9, 0.3, 0.3)),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        } else if tier.0 < MAX_TIER {
            let ucost = upgrade_cost(base.cost, tier.0);
            let t = (tier.0 as usize + 1).min(MAX_TIER as usize);
            let next_dmg = base.damage * DAMAGE_MULT[t];
            let next_rng = base.range * RANGE_MULT[t];

            let mut next_text = format!("\nNext: DMG {:.0}  RNG {:.0}", next_dmg, next_rng);
            if let Some(ba) = base_arc {
                let next_arc = ba.0 * ARC_RANGE_MULT[t];
                next_text.push_str(&format!("  ARC {:.0}", next_arc));
            }

            parent.spawn((
                Text::new(next_text),
                TextColor(Color::srgb(0.8, 0.75, 0.4)),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));

            let can_afford = pile_scrap.amount >= ucost;
            let cost_color = if can_afford {
                LABEL_COLOR
            } else {
                Color::srgb(0.9, 0.3, 0.3)
            };
            parent.spawn((
                Text::new(format!("[U] Upgrade: ${ucost}")),
                TextColor(cost_color),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        } else {
            parent.spawn((
                Text::new("\nMAX TIER"),
                TextColor(Color::srgb(0.4, 0.9, 0.4)),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        }

        // Magnet upgrade section
        if let Some(col) = collector {
            parent.spawn((
                Text::new(format!("COLLECT: {:.0}", col.range)),
                TextColor(STAT_COLOR),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        }

        if let Some(mt) = magnet_tier {
            if mt.0 < MAX_MAGNET_TIER {
                let mcost = MAGNET_UPGRADE_COSTS[mt.0 as usize];
                let can_afford_m = pile_scrap.amount >= mcost;
                let mcost_color = if can_afford_m {
                    LABEL_COLOR
                } else {
                    Color::srgb(0.9, 0.3, 0.3)
                };
                parent.spawn((
                    Text::new(format!("[M] Magnet: ${mcost}")),
                    TextColor(mcost_color),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                ));
            } else {
                parent.spawn((
                    Text::new("MAGNET: MAX"),
                    TextColor(Color::srgb(0.4, 0.7, 1.0)),
                    TextFont {
                        font_size: 13.0,
                        ..default()
                    },
                ));
            }
        } else if is_scrap_magnet.is_some() {
            parent.spawn((
                Text::new("MAGNET: MAX"),
                TextColor(Color::srgb(0.4, 0.7, 1.0)),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        }

        // Repair hint (only when damaged)
        if let Some(health) = tower_health
            && health.current < health.max
        {
            let is_rubble = *tower_state == TowerState::Rubble;
            let frac = if is_rubble {
                REPAIR_RUBBLE_COST_FRAC
            } else {
                REPAIR_COST_FRAC
            };
            let repair_cost = (base.cost as f32 * frac) as u32;
            let can_afford = pile_scrap.amount >= repair_cost;
            let repair_color = if can_afford {
                LABEL_COLOR
            } else {
                Color::srgb(0.9, 0.3, 0.3)
            };
            parent.spawn((
                Text::new(format!("[R] Repair: ${repair_cost}")),
                TextColor(repair_color),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        }

        // Sell hint
        let sell_refund = cost.0 * SELL_REFUND_PERCENT / 100;
        parent.spawn((
            Text::new(format!("[RMB] Sell: +${sell_refund}")),
            TextColor(HINT_COLOR),
            TextFont {
                font_size: 11.0,
                ..default()
            },
        ));

        if targeting_mode.is_some() {
            parent.spawn((
                Text::new("[Click tower] Targeting"),
                TextColor(HINT_COLOR),
                TextFont {
                    font_size: 11.0,
                    ..default()
                },
            ));
        }

        parent.spawn((
            Text::new("[ESC] Close"),
            TextColor(HINT_COLOR),
            TextFont {
                font_size: 11.0,
                ..default()
            },
        ));
    });
}
