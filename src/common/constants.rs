use bevy::prelude::*;

pub const TILE_SIZE: f32 = 20.0;
pub const MIN_GRID_HEIGHT: u32 = 30;
pub const CHUNK_SIZE: u32 = 8;

pub const STARTING_SCRAP: u32 = 200;
pub const STARTING_LIVES: u32 = 20;

pub const SCRAP_DROP_LIFETIME: f32 = 10.0;

pub const WINDOWED_WIDTH: f32 = 1280.0;
pub const WINDOWED_HEIGHT: f32 = 720.0;

#[derive(Resource)]
pub struct GridConfig {
    pub width: u32,
    pub height: u32,
    pub pixel_scale: u32,
    pub spawn_pos: UVec3,
    pub goal_pos: UVec3,
}

impl GridConfig {
    pub fn from_window(screen_w: f32, screen_h: f32) -> Self {
        let pixel_scale = ((screen_h / (MIN_GRID_HEIGHT as f32 * TILE_SIZE)).floor() as u32).max(1);
        let tile_px = TILE_SIZE * pixel_scale as f32;
        let width = (screen_w / tile_px).floor() as u32;
        let height = (screen_h / tile_px).floor() as u32;

        let spawn_pos = UVec3::new(0, height / 2, 0);
        let goal_pos = UVec3::new(width - 1, height / 2, 0);

        Self {
            width,
            height,
            pixel_scale,
            spawn_pos,
            goal_pos,
        }
    }
}
