use bevy::prelude::*;
use std::collections::HashSet;

use crate::audio::resources::SoundAssets;
use crate::common::constants::GridConfig;
use crate::pile::resources::{EdgeCells, PileScrap, PileState};
use crate::pile::systems::{compute_pile_cells, pile_radius};
use crate::stats::resources::RunStats;

/// Minimal headless App for testing (time ticking, no window/renderer).
pub fn test_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(bevy::state::app::StatesPlugin);
    app
}

/// Minimal headless App with asset support (for testing materials, meshes, etc.).
pub fn test_app_with_assets() -> App {
    let mut app = test_app();
    app.add_plugins(bevy::asset::AssetPlugin::default());
    app
}

/// Deterministic 40x32 grid config (equivalent to an 800x640 window).
pub fn test_grid_config() -> GridConfig {
    GridConfig {
        width: 40,
        height: 32,
        pixel_scale: 1,
    }
}

/// Initialize pile-related resources with a known scrap amount.
pub fn insert_pile(app: &mut App, scrap: u32) {
    let config = test_grid_config();
    let center = config.center();
    let radius = pile_radius(scrap);
    let cells = compute_pile_cells(center, radius, config.width, config.height);

    app.insert_resource(PileScrap { amount: scrap });
    app.insert_resource(PileState {
        cells,
        center,
        radius_tiles: radius,
        last_radius_int: radius as u32,
    });
    app.insert_resource(EdgeCells(Vec::new()));
    app.insert_resource(config);
}

/// Initialize pile resources with empty pile state (for manual control).
pub fn insert_empty_pile(app: &mut App, scrap: u32, config: GridConfig) {
    let center = config.center();
    app.insert_resource(PileScrap { amount: scrap });
    app.insert_resource(PileState {
        cells: HashSet::new(),
        center,
        radius_tiles: 0.0,
        last_radius_int: 0,
    });
    app.insert_resource(EdgeCells(Vec::new()));
    app.insert_resource(config);
}

/// `RunStats` with `start_time = 0` and all counters zeroed.
pub fn make_test_stats() -> RunStats {
    RunStats::new(0.0)
}

/// Sensible `BaseStats` defaults for unit tests. Only `cost` varies; other
/// fields use typical ScrapGun-like values.
pub fn test_base_stats(cost: u32) -> crate::tower::components::BaseStats {
    crate::tower::components::BaseStats {
        cost,
        damage: 10.0,
        range: 80.0,
        cooldown_secs: 1.0,
        aoe_radius: 0.0,
        aoe_damage: 0.0,
        slow_factor: 1.0,
        color: Color::srgb(0.5, 0.5, 0.5),
    }
}

/// Mock SoundAssets with default (invalid) handles for headless tests.
/// Systems that call play_sound will spawn AudioPlayer entities that do
/// nothing without an audio backend.
pub fn mock_sound_assets() -> SoundAssets {
    SoundAssets {
        tower_scrapgun: Handle::default(),
        tower_explosive: Handle::default(),
        tower_railgun: Handle::default(),
        tower_chain_lightning: Handle::default(),
        enemy_death: Handle::default(),
        scrap_drop: Handle::default(),
        scrap_collected: Handle::default(),
        boss_spawn: Handle::default(),
        wave_start: Handle::default(),
        game_over: Handle::default(),
        brute_attack: Handle::default(),
        tower_destroyed: Handle::default(),
        tower_repaired: Handle::default(),
    }
}
