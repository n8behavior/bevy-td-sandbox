use bevy::prelude::*;

pub const TILE_SIZE: f32 = 20.0;
pub const MIN_GRID_HEIGHT: u32 = 30;
pub const CHUNK_SIZE: u32 = 8;

pub const STARTING_SCRAP: u32 = 250;
pub const ENDLESS_STARTING_SCRAP: u32 = 400;

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

#[cfg(not(target_arch = "wasm32"))]
pub const WINDOWED_WIDTH: f32 = 1280.0;
#[cfg(not(target_arch = "wasm32"))]
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

/// Total terrain coverage fraction (~8% of eligible cells).
pub const TERRAIN_COVERAGE: f32 = 0.08;

/// Cluster size range for terrain generation.
pub const TERRAIN_MIN_CLUSTER: u32 = 5;
pub const TERRAIN_MAX_CLUSTER: u32 = 25;

/// Coverage weight per terrain type (sum to 1.0).
pub const RUBBLE_WEIGHT: f32 = 0.45;
pub const PUDDLE_WEIGHT: f32 = 0.30;
pub const RADIOACTIVE_WEIGHT: f32 = 0.25;

/// Terrain base colors.
pub const RUBBLE_COLOR: Color = Color::srgb(0.42, 0.38, 0.32);
pub const PUDDLE_COLOR: Color = Color::srgb(0.25, 0.40, 0.55);
pub const RADIOACTIVE_COLOR: Color = Color::srgb(0.35, 0.55, 0.20);

/// Puddle slow: enemy moves at 70% speed (30% reduction).
pub const PUDDLE_SLOW_FACTOR: f32 = 0.70;

/// Radioactive damage per second.
pub const RADIOACTIVE_DPS: f32 = 8.0;

// ---------------------------------------------------------------------------
// Tower health & degradation
// ---------------------------------------------------------------------------

/// Tower HP = base_cost × this multiplier.
pub const TOWER_HP_COST_MULT: f32 = 3.0;

/// HP tier multipliers (index = tier).
pub const TOWER_HP_TIER_MULT: [f32; 3] = [1.0, 1.4, 2.0];

/// Brute melee damage per hit against towers.
pub const BRUTE_ATTACK_DAMAGE: f32 = 20.0;

/// Brute attack cooldown in seconds.
pub const BRUTE_ATTACK_COOLDOWN: f32 = 1.5;

/// Max world distance for a Brute to hit an adjacent tower.
/// Covers adjacent cells (TILE_SIZE=20) plus wander offset (±7).
pub const BRUTE_ATTACK_RANGE: f32 = 30.0;

/// Rubble tower sprite color (dark grey).
pub const RUBBLE_TOWER_COLOR: Color = Color::srgb(0.3, 0.3, 0.3);

/// Repair cost as fraction of base cost (damaged tower → full HP).
pub const REPAIR_COST_FRAC: f32 = 0.3;

/// Repair cost as fraction of base cost (rubble tower → full HP).
pub const REPAIR_RUBBLE_COST_FRAC: f32 = 0.5;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_window_produces_chunk_aligned_dimensions() {
        let config = GridConfig::from_window(1280.0, 720.0);
        assert_eq!(config.width % CHUNK_SIZE, 0, "width not chunk-aligned");
        assert_eq!(config.height % CHUNK_SIZE, 0, "height not chunk-aligned");
    }

    #[test]
    fn from_window_minimum_height() {
        // Even a small window should produce at least MIN_GRID_HEIGHT tiles
        // (rounded down to chunk boundary)
        let config = GridConfig::from_window(800.0, 600.0);
        assert!(
            config.height >= MIN_GRID_HEIGHT / CHUNK_SIZE * CHUNK_SIZE,
            "height {} too small",
            config.height
        );
    }

    #[test]
    fn from_window_pixel_scale_at_least_one() {
        let config = GridConfig::from_window(320.0, 240.0);
        assert!(config.pixel_scale >= 1);
    }

    #[test]
    fn center_returns_midpoint() {
        let config = GridConfig {
            width: 40,
            height: 32,
            pixel_scale: 1,
        };
        assert_eq!(config.center(), UVec3::new(20, 16, 0));
    }
}
