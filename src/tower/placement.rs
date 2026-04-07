use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::common::constants::*;
use crate::enemy::components::SpawnAnimation;
use crate::grid::components::GridCell;
use crate::grid::systems::{grid_to_world, world_to_grid};
use crate::pathfinding::GridChanged;
use crate::pile::resources::{PileScrap, PileState};
use crate::states::GameState;
use crate::stats::resources::RunStats;

use crate::terrain::components::Terrain;

use super::components::*;
use super::upgrade::InspectedTower;

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
    mut inspected: ResMut<InspectedTower>,
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

    // Clear inspection when entering placement mode.
    inspected.0 = None;

    // Despawn old placing entity.
    if let Some(entity) = selected.entity.take() {
        commands.entity(entity).despawn();
    }

    // Spawn new placing tower entity (hidden until cursor positions it).
    let blueprint = &registry.blueprints[idx];
    let mut entity_cmds = commands.spawn((
        Tower,
        TowerState::Placing,
        PlacementValid(false),
        Visibility::Hidden,
        Sprite::from_color(
            blueprint.color.with_alpha(0.5),
            Vec2::splat(TILE_SIZE - 2.0),
        ),
        Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
    ));
    (blueprint.spawn_fn)(&mut entity_cmds);

    selected.index = Some(idx);
    selected.entity = Some(entity_cmds.id());
}

/// Move the placing tower to the cursor and compute placement validity.
pub fn update_placing_tower(
    mut placing: Query<
        (
            Entity,
            &mut Transform,
            &mut PlacementValid,
            &mut Visibility,
            Option<&BlocksNav>,
            &TowerState,
        ),
        With<Tower>,
    >,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    existing_towers: Query<(&GridCell, &TowerState), With<Tower>>,
    terrain_cells: Query<&GridCell, With<Terrain>>,
    mut grid_query: Query<&mut OrdinalGrid>,
    pile_state: Res<PileState>,
    pile_scrap: Res<PileScrap>,
    selected: Res<SelectedTower>,
    registry: Res<TowerRegistry>,
    config: Res<GridConfig>,
) {
    let Ok((_, mut transform, mut valid, mut vis, blocks_nav, tower_state)) = placing.single_mut()
    else {
        return;
    };

    if *tower_state != TowerState::Placing {
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

    let occupied = existing_towers
        .iter()
        .any(|(c, s)| s.is_placed() && c.coord == grid_pos);
    let is_terrain = terrain_cells.iter().any(|c| c.coord == grid_pos);
    let is_pile = pile_state.cells.contains(&cell_uvec);
    let can_afford = pile_scrap.amount >= cost;

    let mut is_valid = !occupied && !is_terrain && !is_pile && can_afford;

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

        let all_paths_exist = edge_midpoints.iter().all(|edge| {
            grid.pathfind(&mut PathfindArgs::new(*edge, center).astar())
                .is_some()
        });

        // Revert tentative change.
        if let Some(old) = old_nav {
            grid.set_nav(cell_uvec, old);
        } else {
            grid.set_nav(cell_uvec, Nav::Passable(1));
        }
        grid.build();

        if !all_paths_exist {
            is_valid = false;
        }
    }

    valid.0 = is_valid;
}

/// Tint the placing tower green (valid) or red (invalid).
pub fn tint_placing_tower(
    mut placing: Query<(&mut Sprite, &PlacementValid, &TowerState), With<Tower>>,
    selected: Res<SelectedTower>,
    registry: Res<TowerRegistry>,
) {
    let Ok((mut sprite, valid, tower_state)) = placing.single_mut() else {
        return;
    };

    if *tower_state != TowerState::Placing {
        return;
    }

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
        (
            Entity,
            &Transform,
            &PlacementValid,
            Option<&BlocksNav>,
            &Children,
            &mut TowerState,
        ),
        With<Tower>,
    >,
    range_rings: Query<Entity, With<RangeRing>>,
    mut grid_query: Query<&mut OrdinalGrid>,
    mut pile_scrap: ResMut<PileScrap>,
    mut selected: ResMut<SelectedTower>,
    registry: Res<TowerRegistry>,
    config: Res<GridConfig>,
    mut stats: Option<ResMut<RunStats>>,
) {
    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok((entity, transform, valid, blocks_nav, children, mut tower_state)) =
        placing.single_mut()
    else {
        return;
    };

    if *tower_state != TowerState::Placing {
        return;
    }

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

    if let Some(stats) = stats.as_mut() {
        stats.towers_placed += 1;
        stats.scrap_spent += blueprint.cost;
    }

    // Transition from Placing to Active.
    let snap_pos = grid_to_world(cell_uvec, &config);
    *tower_state = TowerState::Active;
    commands.entity(entity).remove::<PlacementValid>();
    // Despawn range ring children.
    for child in children.iter() {
        if range_rings.contains(child) {
            commands.entity(child).despawn();
        }
    }
    commands.entity(entity).insert((
        GridCell { coord: grid_pos },
        TowerCost(blueprint.cost),
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
        TowerState::Placing,
        PlacementValid(false),
        Visibility::Hidden,
        Sprite::from_color(
            blueprint.color.with_alpha(0.5),
            Vec2::splat(TILE_SIZE - 2.0),
        ),
        Transform::from_translation(Vec3::new(0.0, 0.0, 2.0)),
    ));
    (blueprint.spawn_fn)(&mut new_cmds);
    selected.entity = Some(new_cmds.id());
}

// ---------------------------------------------------------------------------
// Sell towers
// ---------------------------------------------------------------------------

const SELL_REFUND_PERCENT: u32 = 60;

/// Floating text that rises and fades after selling or upgrading a tower.
#[derive(Component)]
pub struct SellText {
    pub timer: Timer,
}

/// Right-click a placed tower to sell it for 60% of its original cost.
pub fn sell_tower(
    mut commands: Commands,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    towers: Query<
        (
            Entity,
            &GridCell,
            &TowerCost,
            Option<&BlocksNav>,
            &TowerState,
        ),
        With<Tower>,
    >,
    mut grid_query: Query<&mut OrdinalGrid>,
    mut pile_scrap: ResMut<PileScrap>,
    config: Res<GridConfig>,
    mut inspected: ResMut<InspectedTower>,
    mut stats: Option<ResMut<RunStats>>,
) {
    if !mouse.just_pressed(MouseButton::Right) {
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

    // Find a placed tower at this grid cell.
    let Some((entity, _, cost, blocks_nav, _)) = towers
        .iter()
        .find(|(_, gc, _, _, s)| s.is_placed() && gc.coord == grid_pos)
    else {
        return;
    };

    let refund = cost.0 * SELL_REFUND_PERCENT / 100;
    pile_scrap.amount += refund;

    if let Some(stats) = stats.as_mut() {
        stats.towers_sold += 1;
    }

    let cell_uvec = UVec3::new(grid_pos.x as u32, grid_pos.y as u32, 0);
    let tower_world_pos = grid_to_world(cell_uvec, &config);

    // Restore nav grid if this tower blocked pathing.
    if blocks_nav.is_some()
        && let Ok(mut grid) = grid_query.single_mut()
    {
        grid.set_nav(cell_uvec, Nav::Passable(1));
        grid.build();
    }

    // Clear inspection if this was the inspected tower.
    if inspected.0 == Some(entity) {
        inspected.0 = None;
    }

    commands.entity(entity).despawn();
    commands.trigger(GridChanged);

    // Spawn floating refund text.
    commands.spawn((
        Text2d::new(format!("+{refund}")),
        TextFont {
            font_size: 14.0,
            ..default()
        },
        TextColor(Color::srgb(0.9, 0.8, 0.2)),
        Transform::from_translation(tower_world_pos.extend(10.0)),
        SellText {
            timer: Timer::from_seconds(0.8, TimerMode::Once),
        },
    ));
}

/// Animate sell text: rise upward, fade out, then despawn.
pub fn animate_sell_text(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut Transform, &mut TextColor, &mut SellText)>,
) {
    for (entity, mut transform, mut color, mut sell) in &mut query {
        sell.timer.tick(time.delta());
        let progress = sell.timer.elapsed_secs() / sell.timer.duration().as_secs_f32();

        // Rise upward.
        transform.translation.y += 30.0 * time.delta_secs();

        // Fade out.
        color.0 = Color::srgba(0.9, 0.8, 0.2, 1.0 - progress);

        if sell.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
