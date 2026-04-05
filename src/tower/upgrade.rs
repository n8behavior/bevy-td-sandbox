use std::time::Duration;

use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;

use crate::common::constants::{GridConfig, TILE_SIZE};
use crate::grid::components::GridCell;
use crate::grid::systems::world_to_grid;
use crate::pile::resources::PileScrap;
use crate::shader::{CircleMaterial, CircleMesh};
use crate::states::GameState;
use crate::stats::resources::RunStats;
use crate::ui::tower_menu::WavePreviewPanel;

use super::components::*;
use super::placement::{SelectedTower, SellText};
use super::targeting::RadialMenuState;

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

fn tier_color(base: Color, tier: u8) -> Color {
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
    towers: Query<(Entity, &GridCell, Option<&TargetingMode>), (With<Tower>, Without<Placing>)>,
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

    if let Some((entity, _, targeting)) = towers.iter().find(|(_, gc, _)| gc.coord == grid_pos) {
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
            Option<&RangeRingConfig>,
            Option<&AuraRingConfig>,
            Option<&mut ChainLightning>,
            Option<&BaseArcRange>,
            Option<&mut ChainCooldown>,
        ),
        (With<Tower>, Without<Placing>),
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
        range_ring,
        aura_ring,
        chain_lightning,
        base_arc_range,
        chain_cooldown,
    )) = towers.get_mut(entity)
    else {
        return;
    };

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
    let t = tier.0 as usize;

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
        let new_ar = AuraRingConfig {
            range: ar.range,
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
            Option<&ChainLightning>,
            Option<&ChainCooldown>,
            Option<&BaseArcRange>,
            Option<&TargetingMode>,
        ),
        (With<Tower>, Without<Placing>),
    >,
    mut panel_query: Query<(Entity, &mut Visibility), With<UpgradePanel>>,
    mut wave_preview: Query<&mut Visibility, (With<WavePreviewPanel>, Without<UpgradePanel>)>,
    pile_scrap: Res<PileScrap>,
) {
    let Ok((panel_entity, mut vis)) = panel_query.single_mut() else {
        return;
    };

    // Only rebuild when something relevant changes.
    if !inspected.is_changed() && !pile_scrap.is_changed() && !radial_menu.is_changed() {
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
        chain,
        chain_cd,
        base_arc,
        targeting_mode,
    )) = towers.get(entity)
    else {
        *vis = Visibility::Hidden;
        return;
    };

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

        // Upgrade or max tier
        if tier.0 < MAX_TIER {
            let ucost = upgrade_cost(base.cost, tier.0);
            let t = tier.0 as usize + 1;
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

        // Sell hint
        let sell_refund = cost.0 * 60 / 100;
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
