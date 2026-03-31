use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::common::constants::*;
use crate::grid::components::GridCell;
use crate::grid::systems::{grid_to_world, world_to_grid};
use crate::pathfinding::GridChanged;
use crate::pile::resources::{PileScrap, PileState};
use crate::states::GameState;

use super::components::*;

#[derive(Resource, Default)]
pub struct SelectedTower(pub Option<TowerType>);

#[derive(Component)]
pub struct PlacementPreview;

pub fn handle_tower_selection(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedTower>,
) {
    if keyboard.just_pressed(KeyCode::Digit1) {
        selected.0 = Some(TowerType::ScrapGun);
    } else if keyboard.just_pressed(KeyCode::Digit2) {
        selected.0 = Some(TowerType::TarPit);
    } else if keyboard.just_pressed(KeyCode::Digit3) {
        selected.0 = Some(TowerType::Explosive);
    } else if keyboard.just_pressed(KeyCode::Digit4) {
        selected.0 = Some(TowerType::Railgun);
    } else if keyboard.just_pressed(KeyCode::Escape) {
        selected.0 = None;
    }
}

pub fn handle_tower_placement(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    selected: Res<SelectedTower>,
    mut pile_scrap: ResMut<PileScrap>,
    mut grid_query: Query<&mut CardinalGrid>,
    existing_towers: Query<&GridCell, With<Tower>>,
    pile_state: Res<PileState>,
    config: Res<GridConfig>,
) {
    let Some(tower_type) = selected.0 else { return };
    if !mouse.just_pressed(MouseButton::Left) {
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
        return;
    };

    if pile_scrap.amount < tower_type.cost() {
        return;
    }

    let is_occupied = existing_towers
        .iter()
        .any(|cell| cell.coord == grid_pos);
    if is_occupied {
        return;
    }

    // Cannot build on pile cells.
    let cell_uvec = UVec3::new(grid_pos.x as u32, grid_pos.y as u32, 0);
    if pile_state.cells.contains(&cell_uvec) {
        return;
    }

    let Ok(mut grid) = grid_query.single_mut() else {
        return;
    };

    // TarPit is passable (high cost), other towers are impassable
    let is_tarpit = tower_type == TowerType::TarPit;
    let nav = if is_tarpit {
        Nav::Passable(1) // Same cost as open ground -- enemies walk right through
    } else {
        Nav::Impassable
    };

    let old_nav = grid.nav(cell_uvec);
    grid.set_nav(cell_uvec, nav);
    grid.build();

    // Path validation: ensure enemies from edges can still reach the pile.
    if !is_tarpit {
        let center = pile_state.center;
        let edge_midpoints = [
            UVec3::new(0, config.height / 2, 0),
            UVec3::new(config.width - 1, config.height / 2, 0),
            UVec3::new(config.width / 2, 0, 0),
            UVec3::new(config.width / 2, config.height - 1, 0),
        ];

        let any_path_exists = edge_midpoints.iter().any(|edge| {
            grid.pathfind(&mut PathfindArgs::new(*edge, center).astar())
                .is_some()
        });

        if !any_path_exists {
            if let Some(old) = old_nav {
                grid.set_nav(cell_uvec, old);
            } else {
                grid.set_nav(cell_uvec, Nav::Passable(1));
            }
            grid.build();
            return;
        }
    }

    pile_scrap.amount -= tower_type.cost();

    let tower_world = grid_to_world(cell_uvec, &config);
    let stats = tower_type.stats();

    let mut entity_cmds = commands.spawn((
        Tower { tower_type },
        stats.clone(),
        TurretState::new(tower_type),
        GridCell { coord: grid_pos },
        Sprite::from_color(tower_type.color(), Vec2::splat(TILE_SIZE - 2.0)),
        Transform::from_translation(tower_world.extend(0.5))
            .with_scale(Vec3::ZERO),
        crate::enemy::components::SpawnAnimation {
            timer: Timer::from_seconds(0.2, TimerMode::Once),
        },
        DespawnOnExit(GameState::Playing),
    ));

    match tower_type {
        TowerType::TarPit => {
            entity_cmds.insert(SlowOnHit {
                factor: 0.4,
                duration: 0.5, // Short duration -- refreshed by aura while in range
            });
            // Spawn gradient aura rings as children (innermost = darkest)
            let range_px = stats.range * TILE_SIZE;
            let rings = 5;
            for i in 0..rings {
                let frac = (i + 1) as f32 / rings as f32;
                let size = range_px * 2.0 * frac;
                let alpha = 0.45 * (1.0 - frac * 0.6); // 0.45 inner → ~0.18 outer
                entity_cmds.with_child((
                    AuraVisual,
                    Sprite::from_color(
                        Color::srgba(0.3, 0.1, 0.35, alpha),
                        Vec2::splat(size),
                    ),
                    Transform::from_translation(Vec3::new(0.0, 0.0, -0.2)),
                ));
            }
        }
        TowerType::Explosive => {
            entity_cmds.insert(AoEOnHit {
                radius: 3.5 * TILE_SIZE,
                damage: 25.0,
            });
        }
        _ => {}
    }

    commands.trigger(GridChanged);
}

pub fn tower_placement_preview(
    mut commands: Commands,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    selected: Res<SelectedTower>,
    existing_previews: Query<Entity, With<PlacementPreview>>,
    existing_towers: Query<&GridCell, With<Tower>>,
    pile_state: Res<PileState>,
    pile_scrap: Res<PileScrap>,
    config: Res<GridConfig>,
) {
    for entity in &existing_previews {
        commands.entity(entity).despawn();
    }

    let Some(tower_type) = selected.0 else { return };

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
        return;
    };

    let cell_uvec = UVec3::new(grid_pos.x as u32, grid_pos.y as u32, 0);
    let snap_pos = grid_to_world(cell_uvec, &config);

    let occupied = existing_towers.iter().any(|c| c.coord == grid_pos);
    let is_pile = pile_state.cells.contains(&cell_uvec);
    let can_afford = pile_scrap.amount >= tower_type.cost();

    let valid = !occupied && !is_pile && can_afford;

    let color = if valid {
        Color::srgba(0.3, 0.9, 0.3, 0.5)
    } else {
        Color::srgba(0.9, 0.3, 0.3, 0.5)
    };

    // Tower sprite preview
    let mut preview = commands.spawn((
        Sprite::from_color(color, Vec2::splat(TILE_SIZE - 2.0)),
        Transform::from_translation(snap_pos.extend(2.0)),
        PlacementPreview,
    ));

    // Range ring preview
    let stats = tower_type.stats();
    let range_px = stats.range * TILE_SIZE;
    let ring_color = match tower_type {
        TowerType::TarPit => Color::srgba(0.3, 0.1, 0.35, 0.25),
        TowerType::Explosive => Color::srgba(0.9, 0.3, 0.1, 0.1),
        _ => Color::srgba(0.8, 0.8, 0.3, 0.08),
    };
    preview.with_child((
        Sprite::from_color(ring_color, Vec2::splat(range_px * 2.0)),
        Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)),
    ));
}
