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

    if scrap.0 < tower_type.cost() {
        return;
    }

    let is_occupied = existing_towers
        .iter()
        .any(|cell| cell.coord == grid_pos);
    if is_occupied {
        return;
    }

    let is_special = spawn_cells.iter().any(|c| c.coord == grid_pos)
        || goal_cells.iter().any(|c| c.coord == grid_pos);
    if is_special {
        return;
    }

    let cell_uvec = UVec3::new(grid_pos.x as u32, grid_pos.y as u32, 0);

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

    // Path validation: only needed for impassable towers
    if !is_tarpit {
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
            if let Some(old) = old_nav {
                grid.set_nav(cell_uvec, old);
            } else {
                grid.set_nav(cell_uvec, Nav::Passable(1));
            }
            grid.build();
            return;
        }
    }

    scrap.0 -= tower_type.cost();

    let tower_world = grid_to_world(cell_uvec, &config);
    let stats = tower_type.stats();
    let fire_rate = stats.fire_rate;

    let mut entity_cmds = commands.spawn((
        Tower { tower_type },
        stats.clone(),
        AttackCooldown {
            timer: Timer::from_seconds(1.0 / fire_rate, TimerMode::Repeating),
        },
        GridCell { coord: grid_pos },
        Sprite::from_color(tower_type.color(), Vec2::splat(TILE_SIZE - 2.0)),
        Transform::from_translation(tower_world.extend(0.5)),
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
                let alpha = 0.35 * (1.0 - frac * 0.7); // 0.35 inner → ~0.10 outer
                entity_cmds.with_child((
                    AuraVisual,
                    Sprite::from_color(
                        Color::srgba(0.15, 0.1, 0.0, alpha),
                        Vec2::splat(size),
                    ),
                    Transform::from_translation(Vec3::new(0.0, 0.0, -0.2)),
                ));
            }
        }
        TowerType::Explosive => {
            entity_cmds.insert(AoEOnHit {
                radius: 2.5 * TILE_SIZE,
                damage: 15.0,
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
    spawn_cells: Query<&GridCell, With<SpawnPoint>>,
    goal_cells: Query<&GridCell, With<GoalPoint>>,
    scrap: Res<PlayerScrap>,
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
    let is_special = spawn_cells.iter().any(|c| c.coord == grid_pos)
        || goal_cells.iter().any(|c| c.coord == grid_pos);
    let can_afford = scrap.0 >= tower_type.cost();

    let valid = !occupied && !is_special && can_afford;

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
        TowerType::TarPit => Color::srgba(0.15, 0.1, 0.0, 0.2),
        TowerType::Explosive => Color::srgba(0.9, 0.3, 0.1, 0.1),
        _ => Color::srgba(0.8, 0.8, 0.3, 0.08),
    };
    preview.with_child((
        Sprite::from_color(ring_color, Vec2::splat(range_px * 2.0)),
        Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)),
    ));
}
