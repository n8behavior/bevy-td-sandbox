use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::common::constants::*;
use crate::enemy::components::SpawnAnimation;
use crate::grid::components::GridCell;
use crate::grid::systems::{grid_to_world, world_to_grid};
use crate::pathfinding::GridChanged;
use crate::pile::resources::{PileScrap, PileState};
use crate::states::GameState;

use crate::obstacle::components::Obstacle;

use super::components::*;

/// Tracks the currently selected tower blueprint index and placing entity.
#[derive(Resource, Default)]
pub struct SelectedTower {
    pub index: Option<usize>,
    pub entity: Option<Entity>,
}

/// Select a tower type by pressing its registered key. Spawns a placing entity.
pub fn handle_tower_selection(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    mut selected: ResMut<SelectedTower>,
    registry: Res<TowerRegistry>,
) {
    // Check for Escape — deselect.
    if keyboard.just_pressed(KeyCode::Escape) {
        if let Some(entity) = selected.entity.take() {
            commands.entity(entity).despawn();
        }
        selected.index = None;
        return;
    }

    // Check for tower key presses.
    let mut new_index = None;
    for (i, blueprint) in registry.blueprints.iter().enumerate() {
        if keyboard.just_pressed(blueprint.key) {
            new_index = Some(i);
            break;
        }
    }

    let Some(idx) = new_index else { return };

    // If already selecting this type, do nothing.
    if selected.index == Some(idx) {
        return;
    }

    // Despawn old placing entity.
    if let Some(entity) = selected.entity.take() {
        commands.entity(entity).despawn();
    }

    // Spawn new placing tower entity (hidden until cursor positions it).
    let blueprint = &registry.blueprints[idx];
    let mut entity_cmds = commands.spawn((
        Tower,
        Placing,
        PlacementValid(false),
        Visibility::Hidden,
        Sprite::from_color(blueprint.color.with_alpha(0.5), Vec2::splat(TILE_SIZE - 2.0)),
        Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
    ));
    (blueprint.spawn_fn)(&mut entity_cmds);

    selected.index = Some(idx);
    selected.entity = Some(entity_cmds.id());
}

/// Move the placing tower to the cursor and compute placement validity.
pub fn update_placing_tower(
    mut placing: Query<
        (Entity, &mut Transform, &mut PlacementValid, &mut Visibility, Option<&BlocksNav>),
        With<Placing>,
    >,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    existing_towers: Query<&GridCell, (With<Tower>, Without<Placing>)>,
    obstacles: Query<&GridCell, With<Obstacle>>,
    mut grid_query: Query<&mut OrdinalGrid>,
    pile_state: Res<PileState>,
    pile_scrap: Res<PileScrap>,
    selected: Res<SelectedTower>,
    registry: Res<TowerRegistry>,
    config: Res<GridConfig>,
) {
    let Ok((_, mut transform, mut valid, mut vis, blocks_nav)) = placing.single_mut() else {
        return;
    };

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
        valid.0 = false;
        return;
    };

    let cell_uvec = UVec3::new(grid_pos.x as u32, grid_pos.y as u32, 0);
    let snap_pos = grid_to_world(cell_uvec, &config);
    transform.translation = snap_pos.extend(2.0);
    *vis = Visibility::Inherited;

    // Look up cost from registry.
    let cost = selected
        .index
        .and_then(|i| registry.blueprints.get(i))
        .map(|b| b.cost)
        .unwrap_or(u32::MAX);

    let occupied = existing_towers.iter().any(|c| c.coord == grid_pos);
    let is_obstacle = obstacles.iter().any(|c| c.coord == grid_pos);
    let is_pile = pile_state.cells.contains(&cell_uvec);
    let can_afford = pile_scrap.amount >= cost;

    let mut is_valid = !occupied && !is_obstacle && !is_pile && can_afford;

    // Path validation for BlocksNav towers.
    if is_valid
        && blocks_nav.is_some()
        && let Ok(mut grid) = grid_query.single_mut()
    {
        let old_nav = grid.nav(cell_uvec);
        grid.set_nav(cell_uvec, Nav::Impassable);
        grid.build();

        let center = pile_state.center;
        let edge_midpoints = [
            UVec3::new(0, config.height / 2, 0),
            UVec3::new(config.width - 1, config.height / 2, 0),
            UVec3::new(config.width / 2, 0, 0),
            UVec3::new(config.width / 2, config.height - 1, 0),
        ];

        let mut failed_edge = None;
        let all_paths_exist = edge_midpoints.iter().all(|edge| {
            let ok = grid
                .pathfind(&mut PathfindArgs::new(*edge, center).astar())
                .is_some();
            if !ok && failed_edge.is_none() {
                failed_edge = Some(*edge);
            }
            ok
        });

        // Revert tentative change.
        if let Some(old) = old_nav {
            grid.set_nav(cell_uvec, old);
        } else {
            grid.set_nav(cell_uvec, Nav::Passable(1));
        }
        grid.build();

        if !all_paths_exist {
            // Also check without the tentative tower to distinguish
            // "tower blocks a critical path" from "path was already broken".
            let baseline_ok = edge_midpoints.iter().all(|edge| {
                grid.pathfind(&mut PathfindArgs::new(*edge, center).astar())
                    .is_some()
            });
            if !baseline_ok {
                warn!(
                    "Path validation: baseline pathfinding ALREADY broken \
                     (no tower placed). grid={}x{} center={:?} failed_edge={:?}",
                    config.width, config.height, center, failed_edge,
                );
            }
            is_valid = false;
        }
    }

    valid.0 = is_valid;
}

/// Tint the placing tower green (valid) or red (invalid).
pub fn tint_placing_tower(
    mut placing: Query<(&mut Sprite, &PlacementValid), With<Placing>>,
    selected: Res<SelectedTower>,
    registry: Res<TowerRegistry>,
) {
    let Ok((mut sprite, valid)) = placing.single_mut() else {
        return;
    };

    let base_color = selected
        .index
        .and_then(|i| registry.blueprints.get(i))
        .map(|b| b.color)
        .unwrap_or(Color::WHITE);

    if valid.0 {
        sprite.color = base_color.with_alpha(0.6);
    } else {
        sprite.color = Color::srgba(0.9, 0.3, 0.3, 0.5);
    }
}

/// On left-click, commit the placing tower to the grid.
pub fn confirm_tower_placement(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    mut placing: Query<
        (Entity, &Transform, &PlacementValid, Option<&BlocksNav>, &Children),
        With<Placing>,
    >,
    range_rings: Query<Entity, With<RangeRing>>,
    mut grid_query: Query<&mut OrdinalGrid>,
    mut pile_scrap: ResMut<PileScrap>,
    mut selected: ResMut<SelectedTower>,
    registry: Res<TowerRegistry>,
    config: Res<GridConfig>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok((entity, transform, valid, blocks_nav, children)) = placing.single_mut() else {
        return;
    };

    if !valid.0 {
        return;
    }

    let Some(idx) = selected.index else { return };
    let Some(blueprint) = registry.blueprints.get(idx) else {
        return;
    };

    let world_pos = transform.translation.truncate();
    let Some(grid_pos) = world_to_grid(world_pos, &config) else {
        return;
    };

    let cell_uvec = UVec3::new(grid_pos.x as u32, grid_pos.y as u32, 0);

    // Update nav grid.
    if blocks_nav.is_some()
        && let Ok(mut grid) = grid_query.single_mut()
    {
        grid.set_nav(cell_uvec, Nav::Impassable);
        grid.build();
    }

    // Deduct cost.
    pile_scrap.amount -= blueprint.cost;

    // Transition from Placing to placed.
    let snap_pos = grid_to_world(cell_uvec, &config);
    commands.entity(entity).remove::<Placing>();
    commands.entity(entity).remove::<PlacementValid>();
    // Despawn range ring children.
    for child in children.iter() {
        if range_rings.contains(child) {
            commands.entity(child).despawn();
        }
    }
    commands.entity(entity).insert((
        GridCell { coord: grid_pos },
        Sprite::from_color(blueprint.color, Vec2::splat(TILE_SIZE - 2.0)),
        Transform::from_translation(snap_pos.extend(0.5)).with_scale(Vec3::ZERO),
        SpawnAnimation {
            timer: Timer::from_seconds(0.2, TimerMode::Once),
        },
        DespawnOnExit(GameState::Playing),
    ));

    commands.trigger(GridChanged);

    // Spawn a new placing tower for continued placement (hidden until cursor positions it).
    let mut new_cmds = commands.spawn((
        Tower,
        Placing,
        PlacementValid(false),
        Visibility::Hidden,
        Sprite::from_color(blueprint.color.with_alpha(0.5), Vec2::splat(TILE_SIZE - 2.0)),
        Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
    ));
    (blueprint.spawn_fn)(&mut new_cmds);
    selected.entity = Some(new_cmds.id());
}

