use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::camera::components::CameraController;
use crate::common::constants::*;
use crate::states::GameState;

use super::components::*;

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
    let mut grid = OrdinalGrid::new(&settings);

    for x in 0..config.width {
        for y in 0..config.height {
            grid.set_nav(UVec3::new(x, y, 0), Nav::Passable(1));
        }
    }
    grid.build();

    commands.spawn((grid, DespawnOnExit(GameState::Playing)));

    // Ground plane — fine grid line color fills gaps between cells.
    let grid_w = config.width as f32 * TILE_SIZE;
    let grid_h = config.height as f32 * TILE_SIZE;
    commands.spawn((
        Sprite::from_color(GRID_LINE_COLOR, Vec2::new(grid_w, grid_h)),
        Transform::from_translation(Vec3::new(0.0, 0.0, -1.0)),
        DespawnOnExit(GameState::Playing),
    ));

    // Cell sprites — slightly smaller than TILE_SIZE to reveal fine grid lines.
    let cell_size = TILE_SIZE - GRID_LINE_WIDTH;
    for x in 0..config.width {
        for y in 0..config.height {
            let world_pos = grid_to_world_cfg(UVec3::new(x, y, 0), &config);
            let is_edge = x == 0 || x == config.width - 1 || y == 0 || y == config.height - 1;

            let mut entity = commands.spawn((
                Sprite::from_color(PAPER_COLOR, Vec2::splat(cell_size)),
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

    // Major grid lines — darker lines every MAJOR_GRID_INTERVAL cells.
    let half_w = grid_w / 2.0;
    let half_h = grid_h / 2.0;

    for c in (0..=config.width).step_by(MAJOR_GRID_INTERVAL as usize) {
        let x = c as f32 * TILE_SIZE - half_w;
        commands.spawn((
            Sprite::from_color(MAJOR_LINE_COLOR, Vec2::new(MAJOR_LINE_WIDTH, grid_h)),
            Transform::from_translation(Vec3::new(x, 0.0, 0.1)),
            DespawnOnExit(GameState::Playing),
        ));
    }

    for r in (0..=config.height).step_by(MAJOR_GRID_INTERVAL as usize) {
        let y = r as f32 * TILE_SIZE - half_h;
        commands.spawn((
            Sprite::from_color(MAJOR_LINE_COLOR, Vec2::new(grid_w, MAJOR_LINE_WIDTH)),
            Transform::from_translation(Vec3::new(0.0, y, 0.1)),
            DespawnOnExit(GameState::Playing),
        ));
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
