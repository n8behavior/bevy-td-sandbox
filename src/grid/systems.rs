use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::Rng;

use crate::camera::components::CameraController;
use crate::common::constants::*;
use crate::states::GameState;

use super::components::*;

/// Per-cell color jitter range (+/- applied to each RGB channel).
const COLOR_JITTER: f32 = 0.03;

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
    let scale = 1.0 / config.pixel_scale as f32;
    commands.spawn((
        Camera2d,
        DespawnOnExit(GameState::Playing),
        Projection::Orthographic(OrthographicProjection {
            scaling_mode: ScalingMode::WindowSize,
            scale,
            ..OrthographicProjection::default_2d()
        }),
        CameraController {
            min_scale: scale * 0.25,
            max_scale: scale * 3.0,
            zoom_step: 1.15,
            home_translation: Vec3::ZERO,
            home_scale: scale,
        },
    ));
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

    // Solid ground plane behind all cells to eliminate black gaps.
    let grid_w = config.width as f32 * TILE_SIZE;
    let grid_h = config.height as f32 * TILE_SIZE;
    commands.spawn((
        Sprite::from_color(GROUND_COLOR, Vec2::new(grid_w, grid_h)),
        Transform::from_translation(Vec3::new(0.0, 0.0, -1.0)),
        DespawnOnExit(GameState::Playing),
    ));

    let mut rng = rand::rng();

    for x in 0..config.width {
        for y in 0..config.height {
            let world_pos = grid_to_world_cfg(UVec3::new(x, y, 0), &config);
            let is_edge = x == 0 || x == config.width - 1 || y == 0 || y == config.height - 1;

            let color = jitter_color(&mut rng, GROUND_COLOR);

            let mut entity = commands.spawn((
                Sprite::from_color(color, Vec2::splat(TILE_SIZE)),
                Transform::from_translation(world_pos.extend(0.0)),
                GridCell {
                    coord: IVec2::new(x as i32, y as i32),
                },
                DespawnOnExit(GameState::Playing),
            ));

            if is_edge {
                entity.insert(EdgeCell);
            }
        }
    }
}

/// Apply small random RGB perturbation to a color.
fn jitter_color(rng: &mut impl Rng, color: Color) -> Color {
    let Srgba { red, green, blue, alpha } = Srgba::from(color);
    Color::srgba(
        (red + rng.random_range(-COLOR_JITTER..COLOR_JITTER)).clamp(0.0, 1.0),
        (green + rng.random_range(-COLOR_JITTER..COLOR_JITTER)).clamp(0.0, 1.0),
        (blue + rng.random_range(-COLOR_JITTER..COLOR_JITTER)).clamp(0.0, 1.0),
        alpha,
    )
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
