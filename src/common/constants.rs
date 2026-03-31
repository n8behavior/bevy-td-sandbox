use bevy::prelude::*;

pub const TILE_SIZE: f32 = 20.0;
pub const MIN_GRID_HEIGHT: u32 = 30;
pub const CHUNK_SIZE: u32 = 8;

pub const STARTING_SCRAP: u32 = 10_000;

pub const SCRAP_DROP_LIFETIME: f32 = 10.0;

/// Tuning constant: how many scrap per tile of pile area.
/// Higher = smaller pile for same scrap amount.
pub const SCRAP_PER_TILE: f32 = 2000.0;

pub const WINDOWED_WIDTH: f32 = 1280.0;
pub const WINDOWED_HEIGHT: f32 = 720.0;

/// Base ground color used for non-pile cells.
pub const GROUND_COLOR: Color = Color::srgb(0.18, 0.2, 0.13);

/// Junk pile cell color.
pub const PILE_COLOR: Color = Color::srgb(0.7, 0.55, 0.2);

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
        let width = (screen_w / tile_px).floor() as u32;
        let height = (screen_h / tile_px).floor() as u32;

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
