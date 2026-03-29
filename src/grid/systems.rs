use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::common::constants::*;
use crate::states::GameState;

use super::components::*;

const GROUND_COLOR: Color = Color::srgb(0.15, 0.15, 0.12);
const GRID_LINE_COLOR: Color = Color::srgb(0.22, 0.22, 0.18);
const SPAWN_COLOR: Color = Color::srgb(0.2, 0.6, 0.2);
const GOAL_COLOR: Color = Color::srgb(0.8, 0.2, 0.2);

pub fn setup_camera(mut commands: Commands) {
    commands.spawn((Camera2d, DespawnOnExit(GameState::Playing)));
}

pub fn setup_grid(mut commands: Commands) {
    let settings = GridSettingsBuilder::new_2d(GRID_WIDTH, GRID_HEIGHT)
        .chunk_size(CHUNK_SIZE)
        .build();
    let mut grid = CardinalGrid::new(&settings);

    // All cells passable by default
    for x in 0..GRID_WIDTH {
        for y in 0..GRID_HEIGHT {
            grid.set_nav(UVec3::new(x, y, 0), Nav::Passable(1));
        }
    }
    grid.build();

    commands.spawn((grid, DespawnOnExit(GameState::Playing)));

    // Render ground tiles
    let tile_inner = TILE_SIZE - 1.0; // 1px gap for grid lines
    for x in 0..GRID_WIDTH {
        for y in 0..GRID_HEIGHT {
            let world_pos = grid_to_world(UVec3::new(x, y, 0));
            let is_spawn = x == 0 && y == GRID_HEIGHT / 2;
            let is_goal = x == GRID_WIDTH - 1 && y == GRID_HEIGHT / 2;

            let color = if is_spawn {
                SPAWN_COLOR
            } else if is_goal {
                GOAL_COLOR
            } else {
                GROUND_COLOR
            };

            let mut entity = commands.spawn((
                Sprite::from_color(color, Vec2::splat(tile_inner)),
                Transform::from_translation(world_pos.extend(0.0)),
                GridCell {
                    coord: IVec2::new(x as i32, y as i32),
                },
                DespawnOnExit(GameState::Playing),
            ));

            if is_spawn {
                entity.insert(SpawnPoint);
            }
            if is_goal {
                entity.insert(GoalPoint);
            }
        }
    }
}

pub fn grid_to_world(coord: UVec3) -> Vec2 {
    let offset_x = (GRID_WIDTH as f32 * TILE_SIZE) / 2.0;
    let offset_y = (GRID_HEIGHT as f32 * TILE_SIZE) / 2.0;
    Vec2::new(
        coord.x as f32 * TILE_SIZE + TILE_SIZE / 2.0 - offset_x,
        coord.y as f32 * TILE_SIZE + TILE_SIZE / 2.0 - offset_y,
    )
}

pub fn world_to_grid(pos: Vec2) -> Option<IVec2> {
    let offset_x = (GRID_WIDTH as f32 * TILE_SIZE) / 2.0;
    let offset_y = (GRID_HEIGHT as f32 * TILE_SIZE) / 2.0;
    let gx = ((pos.x + offset_x) / TILE_SIZE).floor() as i32;
    let gy = ((pos.y + offset_y) / TILE_SIZE).floor() as i32;
    if gx >= 0 && gx < GRID_WIDTH as i32 && gy >= 0 && gy < GRID_HEIGHT as i32 {
        Some(IVec2::new(gx, gy))
    } else {
        None
    }
}
