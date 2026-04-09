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

/// Spawn the 2D camera with pixel-scale-adjusted orthographic projection.
/// Camera2d auto-requires a default projection; we override it with a custom
/// scale derived from GridConfig::pixel_scale.
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

/// Create the pathfinding navigation grid with all cells passable.
pub fn spawn_nav_grid(mut commands: Commands, config: Res<GridConfig>) {
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
}

/// Spawn the visual grid: ground plane, cell sprites, and major grid lines.
pub fn spawn_grid_visuals(mut commands: Commands, config: Res<GridConfig>) {
    // Ground plane — sits behind cells (z=-1) so its color shows as fine grid lines.
    let grid_w = config.width as f32 * TILE_SIZE;
    let grid_h = config.height as f32 * TILE_SIZE;
    commands.spawn((
        Sprite::from_color(GRID_LINE_COLOR, Vec2::new(grid_w, grid_h)),
        Transform::from_translation(Vec3::new(0.0, 0.0, -1.0)),
        DespawnOnExit(GameState::Playing),
    ));

    // Cell sprites are smaller than TILE_SIZE by GRID_LINE_WIDTH so the ground
    // plane (colored GRID_LINE_COLOR) shows through the gap as fine grid lines.
    let cell_size = TILE_SIZE - GRID_LINE_WIDTH;
    for x in 0..config.width {
        for y in 0..config.height {
            let world_pos = grid_to_world_cfg(UVec3::new(x, y, 0), &config);
            let is_edge = config.is_edge(x, y);

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

    // Major grid lines — drawn above cells (z=0.1) every MAJOR_GRID_INTERVAL cells.
    for x in major_line_positions(config.width) {
        commands.spawn((
            Sprite::from_color(MAJOR_LINE_COLOR, Vec2::new(MAJOR_LINE_WIDTH, grid_h)),
            Transform::from_translation(Vec3::new(x, 0.0, 0.1)),
            DespawnOnExit(GameState::Playing),
        ));
    }

    for y in major_line_positions(config.height) {
        commands.spawn((
            Sprite::from_color(MAJOR_LINE_COLOR, Vec2::new(grid_w, MAJOR_LINE_WIDTH)),
            Transform::from_translation(Vec3::new(0.0, y, 0.1)),
            DespawnOnExit(GameState::Playing),
        ));
    }
}

/// Compute world-space positions for major grid lines along one axis.
fn major_line_positions(extent: u32) -> Vec<f32> {
    let total = extent as f32 * TILE_SIZE;
    let half = total / 2.0;
    (0..=extent)
        .step_by(MAJOR_GRID_INTERVAL as usize)
        .map(|c| c as f32 * TILE_SIZE - half)
        .collect()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> GridConfig {
        GridConfig {
            width: 40,
            height: 32,
            pixel_scale: 1,
        }
    }

    #[test]
    fn grid_world_round_trip() {
        let config = cfg();
        for x in [0, 1, 10, 20, 39] {
            for y in [0, 1, 10, 16, 31] {
                let coord = UVec3::new(x, y, 0);
                let world = grid_to_world_cfg(coord, &config);
                let back = world_to_grid(world, &config).expect("should be in bounds");
                assert_eq!(
                    back,
                    IVec2::new(x as i32, y as i32),
                    "round trip failed for ({x}, {y})"
                );
            }
        }
    }

    #[test]
    fn world_to_grid_out_of_bounds_returns_none() {
        let config = cfg();
        // Far outside the grid
        assert!(world_to_grid(Vec2::new(10000.0, 10000.0), &config).is_none());
        assert!(world_to_grid(Vec2::new(-10000.0, -10000.0), &config).is_none());
    }

    #[test]
    fn grid_to_world_corner_cells() {
        let config = cfg();
        let origin = grid_to_world_cfg(UVec3::new(0, 0, 0), &config);
        let far = grid_to_world_cfg(UVec3::new(39, 31, 0), &config);
        // Origin cell should be in negative quadrant
        assert!(origin.x < 0.0);
        assert!(origin.y < 0.0);
        // Far corner should be in positive quadrant
        assert!(far.x > 0.0);
        assert!(far.y > 0.0);
    }

    #[test]
    fn major_line_positions_count() {
        // 40-wide grid: lines at 0, 5, 10, 15, 20, 25, 30, 35, 40 = 9 lines
        let positions = major_line_positions(40);
        assert_eq!(positions.len(), 9);
    }

    #[test]
    fn major_line_positions_boundaries() {
        let positions = major_line_positions(40);
        let half = 40.0 * TILE_SIZE / 2.0;
        // First line at left edge
        assert!((positions[0] - (-half)).abs() < f32::EPSILON);
        // Last line at right edge
        assert!((positions[positions.len() - 1] - half).abs() < f32::EPSILON);
    }

    #[test]
    fn major_line_positions_small_grid() {
        // 8-wide grid (single chunk): lines at 0, 5 = 2 lines
        // (step_by(5) from 0..=8 yields 0 and 5; 10 > 8 so no third)
        let positions = major_line_positions(8);
        assert_eq!(positions.len(), 2);
    }
}
