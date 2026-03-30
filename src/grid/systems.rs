use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::common::constants::*;
use crate::states::GameState;

use super::components::*;

const GROUND_COLOR: Color = Color::srgb(0.15, 0.15, 0.12);
const SPAWN_COLOR: Color = Color::srgb(0.2, 0.6, 0.2);
const GOAL_COLOR: Color = Color::srgb(0.8, 0.2, 0.2);

pub fn compute_grid_config(mut commands: Commands, windows: Query<&Window>) {
    let Ok(window) = windows.single() else { return };
    let config = GridConfig::from_window(window.width(), window.height());
    info!(
        "Grid: {}x{} tiles, pixel_scale={}x ({}x{})",
        config.width,
        config.height,
        config.pixel_scale,
        window.width(),
        window.height()
    );
    commands.insert_resource(config);
}

pub fn setup_camera(mut commands: Commands, config: Res<GridConfig>) {
    use bevy::camera::ScalingMode;
    let mut cam = commands.spawn((Camera2d, DespawnOnExit(GameState::Playing)));
    cam.insert(Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::WindowSize,
        scale: 1.0 / config.pixel_scale as f32,
        ..OrthographicProjection::default_2d()
    }));
}

pub fn setup_grid(mut commands: Commands, config: Res<GridConfig>) {
    let settings = GridSettingsBuilder::new_2d(config.width, config.height)
        .chunk_size(CHUNK_SIZE)
        .build();
    let mut grid = CardinalGrid::new(&settings);

    for x in 0..config.width {
        for y in 0..config.height {
            grid.set_nav(UVec3::new(x, y, 0), Nav::Passable(1));
        }
    }
    grid.build();

    commands.spawn((grid, DespawnOnExit(GameState::Playing)));

    let tile_inner = TILE_SIZE - 1.0;
    for x in 0..config.width {
        for y in 0..config.height {
            let world_pos = grid_to_world_cfg(UVec3::new(x, y, 0), &config);
            let is_spawn = UVec3::new(x, y, 0) == config.spawn_pos;
            let is_goal = UVec3::new(x, y, 0) == config.goal_pos;

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

/// Convert grid coord to world position, using GridConfig resource
pub fn grid_to_world_cfg(coord: UVec3, config: &GridConfig) -> Vec2 {
    let offset_x = (config.width as f32 * TILE_SIZE) / 2.0;
    let offset_y = (config.height as f32 * TILE_SIZE) / 2.0;
    Vec2::new(
        coord.x as f32 * TILE_SIZE + TILE_SIZE / 2.0 - offset_x,
        coord.y as f32 * TILE_SIZE + TILE_SIZE / 2.0 - offset_y,
    )
}

/// Convert grid coord to world position, using GridConfig from ECS
pub fn grid_to_world(coord: UVec3, config: &Res<GridConfig>) -> Vec2 {
    grid_to_world_cfg(coord, config)
}

pub fn world_to_grid(pos: Vec2, config: &GridConfig) -> Option<IVec2> {
    let offset_x = (config.width as f32 * TILE_SIZE) / 2.0;
    let offset_y = (config.height as f32 * TILE_SIZE) / 2.0;
    let gx = ((pos.x + offset_x) / TILE_SIZE).floor() as i32;
    let gy = ((pos.y + offset_y) / TILE_SIZE).floor() as i32;
    if gx >= 0 && gx < config.width as i32 && gy >= 0 && gy < config.height as i32 {
        Some(IVec2::new(gx, gy))
    } else {
        None
    }
}
