//! Junk pile: the player's scrap stockpile at the grid center.
//!
//! `PileScrap` tracks the current amount; `update_pile_state` converts it to
//! a circular cell footprint (`PileState`), and `update_pile_visuals` syncs
//! grid cell sprites to match.

pub mod components;
pub mod resources;
pub mod systems;

pub use systems::{compute_pile_cells, nearest_edge_cell, nearest_pile_cell, pile_radius};

use bevy::prelude::*;

use crate::common::constants::*;
use crate::states::{GameMode, GameState};
use crate::tower::components::{AuraRingConfig, ScrapCollector};

use resources::{EdgeCells, PileScrap, PileState};

pub struct PilePlugin;

impl Plugin for PilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Playing),
            init_pile.after(crate::grid::systems::spawn_grid_visuals),
        )
        .add_systems(
            Update,
            // update_pile_state must run before update_pile_visuals
            // so visuals always reflect the current pile cell set.
            (systems::update_pile_state, systems::update_pile_visuals)
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
    }
}

pub fn init_pile(mut commands: Commands, config: Res<GridConfig>, game_mode: Res<GameMode>) {
    let starting_scrap = match *game_mode {
        GameMode::Classic => STARTING_SCRAP,
        GameMode::Endless => ENDLESS_STARTING_SCRAP,
    };
    let center = config.center();
    let radius = systems::pile_radius(starting_scrap);

    let cells = systems::compute_pile_cells(center, radius, config.width, config.height);

    commands.insert_resource(PileScrap {
        amount: starting_scrap,
    });
    commands.insert_resource(PileState {
        cells,
        center,
        radius_tiles: radius,
        last_radius_int: radius as u32,
    });

    // Compute edge cells once.
    let mut edges = Vec::new();
    for x in 0..config.width {
        edges.push(UVec3::new(x, 0, 0));
        edges.push(UVec3::new(x, config.height - 1, 0));
    }
    for y in 1..config.height - 1 {
        edges.push(UVec3::new(0, y, 0));
        edges.push(UVec3::new(config.width - 1, y, 0));
    }
    commands.insert_resource(EdgeCells(edges));

    // Pile center entity: acts as a scrap collector with aura visual.
    let pile_range = PILE_COLLECTOR_RANGE;
    let world_pos = crate::grid::systems::grid_to_world_cfg(center, &config);
    commands.spawn((
        ScrapCollector { range: pile_range },
        AuraRingConfig {
            range: pile_range,
            color: MAGNET_AURA_COLOR,
        },
        Transform::from_translation(world_pos.extend(0.25)),
        Visibility::Inherited,
        DespawnOnExit(GameState::Playing),
    ));
}
