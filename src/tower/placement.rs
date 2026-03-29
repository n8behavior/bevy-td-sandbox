use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::common::constants::*;
use crate::economy::resources::PlayerScrap;
use crate::grid::components::{GridCell, SpawnPoint, GoalPoint};
use crate::grid::systems::{grid_to_world, world_to_grid};
use crate::pathfinding::GridChanged;
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
    mut scrap: ResMut<PlayerScrap>,
    mut grid_query: Query<&mut CardinalGrid>,
    existing_towers: Query<&GridCell, With<Tower>>,
    spawn_cells: Query<&GridCell, With<SpawnPoint>>,
    goal_cells: Query<&GridCell, With<GoalPoint>>,
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

    let Some(grid_pos) = world_to_grid(world_pos) else {
        return;
    };

    // Check affordability
    if scrap.0 < tower_type.cost() {
        return;
    }

    // Check cell not already occupied
    let is_occupied = existing_towers
        .iter()
        .any(|cell| cell.coord == grid_pos);
    if is_occupied {
        return;
    }

    // Don't place on spawn or goal
    let is_special = spawn_cells.iter().any(|c| c.coord == grid_pos)
        || goal_cells.iter().any(|c| c.coord == grid_pos);
    if is_special {
        return;
    }

    let cell_uvec = UVec3::new(grid_pos.x as u32, grid_pos.y as u32, 0);

    // Path validation: temporarily block, check if path still exists
    let Ok(mut grid) = grid_query.single_mut() else {
        return;
    };

    let old_nav = grid.nav(cell_uvec);
    grid.set_nav(cell_uvec, Nav::Impassable);
    grid.build();

    // Get spawn and goal positions
    let Ok(spawn_cell) = spawn_cells.single() else {
        return;
    };
    let Ok(goal_cell) = goal_cells.single() else {
        return;
    };
    let spawn_uvec = UVec3::new(spawn_cell.coord.x as u32, spawn_cell.coord.y as u32, 0);
    let goal_uvec = UVec3::new(goal_cell.coord.x as u32, goal_cell.coord.y as u32, 0);

    let path_exists = grid
        .pathfind(&mut PathfindArgs::new(spawn_uvec, goal_uvec).astar())
        .is_some();

    if !path_exists {
        // Revert
        if let Some(old) = old_nav {
            grid.set_nav(cell_uvec, old);
        } else {
            grid.set_nav(cell_uvec, Nav::Passable(1));
        }
        grid.build();
        return;
    }

    // Valid placement! Deduct cost and spawn tower
    scrap.0 -= tower_type.cost();

    let world_pos = grid_to_world(cell_uvec);
    let stats = tower_type.stats();
    let fire_rate = stats.fire_rate;

    let mut entity_cmds = commands.spawn((
        Tower,
        stats,
        AttackCooldown {
            timer: Timer::from_seconds(1.0 / fire_rate, TimerMode::Repeating),
        },
        GridCell { coord: grid_pos },
        Sprite::from_color(tower_type.color(), Vec2::splat(TILE_SIZE - 2.0)),
        Transform::from_translation(world_pos.extend(0.5)),
        DespawnOnExit(GameState::Playing),
    ));

    // Add special components based on tower type
    match tower_type {
        TowerType::TarPit => {
            entity_cmds.insert(SlowOnHit {
                factor: 0.4,
                duration: 2.0,
            });
        }
        TowerType::Explosive => {
            entity_cmds.insert(AoEOnHit {
                radius: 2.5 * TILE_SIZE,
                damage: 15.0,
            });
        }
        _ => {}
    }

    // Trigger enemy re-pathing
    commands.trigger(GridChanged);
}

pub fn tower_placement_preview(
    mut commands: Commands,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    selected: Res<SelectedTower>,
    existing_previews: Query<Entity, With<PlacementPreview>>,
    existing_towers: Query<&GridCell, With<Tower>>,
    spawn_cells: Query<&GridCell, With<SpawnPoint>>,
    goal_cells: Query<&GridCell, With<GoalPoint>>,
    scrap: Res<PlayerScrap>,
) {
    // Despawn old preview
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
    let Some(grid_pos) = world_to_grid(world_pos) else {
        return;
    };

    let cell_uvec = UVec3::new(grid_pos.x as u32, grid_pos.y as u32, 0);
    let snap_pos = grid_to_world(cell_uvec);

    // Determine if placement is valid
    let occupied = existing_towers.iter().any(|c| c.coord == grid_pos);
    let is_special = spawn_cells.iter().any(|c| c.coord == grid_pos)
        || goal_cells.iter().any(|c| c.coord == grid_pos);
    let can_afford = scrap.0 >= tower_type.cost();

    let valid = !occupied && !is_special && can_afford;

    let color = if valid {
        Color::srgba(0.3, 0.9, 0.3, 0.5)
    } else {
        Color::srgba(0.9, 0.3, 0.3, 0.5)
    };

    commands.spawn((
        Sprite::from_color(color, Vec2::splat(TILE_SIZE - 2.0)),
        Transform::from_translation(snap_pos.extend(2.0)),
        PlacementPreview,
    ));
}
