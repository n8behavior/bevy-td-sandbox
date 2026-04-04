use bevy::prelude::*;

pub const TILE_SIZE: f32 = 20.0;
pub const MIN_GRID_HEIGHT: u32 = 30;
pub const CHUNK_SIZE: u32 = 8;

pub const STARTING_SCRAP: u32 = 200;

pub const SCRAP_DROP_LIFETIME: f32 = 10.0;

/// Scrap collection: pull speed for scrap drops (world units/sec).
pub const SCRAP_PULL_SPEED: f32 = 60.0;
/// Scrap Magnet tower: pull speed for enemies (world units/sec at field center).
pub const ENEMY_PULL_SPEED: f32 = 15.0;
/// Distance at which a pulled scrap drop is auto-collected.
pub const MAGNET_COLLECT_RADIUS: f32 = 5.0;
/// Aura color shared by all scrap collectors (blue electromagnetic).
pub const MAGNET_AURA_COLOR: Color = Color::srgba(0.15, 0.35, 0.7, 0.55);

/// Tuning constant: how many scrap per tile of pile area.
/// Higher = smaller pile for same scrap amount.
pub const SCRAP_PER_TILE: f32 = 2000.0;

pub const WINDOWED_WIDTH: f32 = 1280.0;
pub const WINDOWED_HEIGHT: f32 = 720.0;

/// Cell fill — warm sepia paper.
pub const PAPER_COLOR: Color = Color::srgb(0.82, 0.76, 0.66);

/// Fine grid line color (shows through gaps between cells).
pub const GRID_LINE_COLOR: Color = Color::srgb(0.68, 0.62, 0.52);

/// Major grid line color, drawn every MAJOR_GRID_INTERVAL cells.
pub const MAJOR_LINE_COLOR: Color = Color::srgb(0.55, 0.48, 0.38);

/// Major grid line interval in cells.
pub const MAJOR_GRID_INTERVAL: u32 = 5;

/// Gap between cells (fine grid line width in world units).
pub const GRID_LINE_WIDTH: f32 = 1.0;

/// Major grid line thickness in world units.
pub const MAJOR_LINE_WIDTH: f32 = 1.5;

/// Junk pile cell color.
pub const PILE_COLOR: Color = Color::srgb(0.7, 0.55, 0.2);

/// Obstacle (rubble/ruin) base color.
pub const OBSTACLE_COLOR: Color = Color::srgb(0.42, 0.38, 0.32);

/// Fraction of eligible cells covered by obstacles.
pub const OBSTACLE_COVERAGE: f32 = 0.08;

/// Cluster size range for obstacle generation.
pub const OBSTACLE_MIN_CLUSTER: u32 = 5;
pub const OBSTACLE_MAX_CLUSTER: u32 = 25;

#[derive(Resource)]
pub struct GridConfig {
    pub width: u32,
    pub height: u32,
    pub pixel_scale: u32,
}

impl GridConfig {
    pub fn from_window(screen_w: f32, screen_h: f32) -> Self {
        let pixel_scale = ((screen_h / (MIN_GRID_HEIGHT as f32 * TILE_SIZE)).floor() as u32).max(1);
        let tile_px = TILE_SIZE * pixel_scale as f32;
        // Round down to chunk_size so every cell belongs to a full chunk
        // (bevy_northstar truncates: cells beyond y_chunks*chunk_size are unreachable).
        let width = (screen_w / tile_px).floor() as u32 / CHUNK_SIZE * CHUNK_SIZE;
        let height = (screen_h / tile_px).floor() as u32 / CHUNK_SIZE * CHUNK_SIZE;

        Self {
            width,
            height,
            pixel_scale,
        }
    }

    /// Grid center coordinate.
    pub fn center(&self) -> UVec3 {
        UVec3::new(self.width / 2, self.height / 2, 0)
    }
}
