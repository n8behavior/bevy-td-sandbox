use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::Rng;

use crate::camera::components::CameraController;
use crate::common::constants::*;
use crate::states::GameState;

use super::components::*;

const GROUND_COLOR: Color = Color::srgb(0.18, 0.2, 0.13);
const SPAWN_COLOR: Color = Color::srgb(0.2, 0.6, 0.2);
const GOAL_COLOR: Color = Color::srgb(0.8, 0.2, 0.2);

/// Per-cell color jitter range (+/- applied to each RGB channel).
const COLOR_JITTER: f32 = 0.03;
/// Size of the glow sprite relative to tile size.
const GLOW_SCALE: f32 = 2.5;

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
            let is_spawn = UVec3::new(x, y, 0) == config.spawn_pos;
            let is_goal = UVec3::new(x, y, 0) == config.goal_pos;

            let base_color = if is_spawn {
                SPAWN_COLOR
            } else if is_goal {
                GOAL_COLOR
            } else {
                GROUND_COLOR
            };

            // Per-cell color jitter for organic feel.
            let color = jitter_color(&mut rng, base_color);

            let mut entity = commands.spawn((
                Sprite::from_color(color, Vec2::splat(TILE_SIZE)),
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

            // Pulsing glow behind spawn/goal cells.
            if is_spawn || is_goal {
                entity.with_child((
                    SpecialCellGlow,
                    Sprite::from_color(
                        base_color.with_alpha(0.25),
                        Vec2::splat(TILE_SIZE * GLOW_SCALE),
                    ),
                    Transform::from_translation(Vec3::new(0.0, 0.0, -0.1)),
                ));
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

/// Pulse the alpha of spawn/goal glow sprites.
pub fn animate_special_cell_glow(
    mut glows: Query<&mut Sprite, With<SpecialCellGlow>>,
    time: Res<Time>,
) {
    let t = time.elapsed_secs();
    // Oscillate alpha between 0.10 and 0.30.
    let alpha = 0.20 + 0.10 * (t * 2.5).sin();
    for mut sprite in &mut glows {
        sprite.color = sprite.color.with_alpha(alpha);
    }
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
