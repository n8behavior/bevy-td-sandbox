pub mod components;
pub mod placement;
pub mod systems;

use bevy::prelude::*;
use crate::states::{GameState, PlayPhase};

pub struct TowerPlugin;

impl Plugin for TowerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<placement::SelectedTower>()
            .add_systems(
                Update,
                (
                    placement::handle_tower_selection,
                    placement::handle_tower_placement,
                    placement::tower_placement_preview,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (systems::tower_shooting, systems::tarpit_aura)
                    .run_if(in_state(GameState::Playing))
                    .run_if(in_state(PlayPhase::Defending)),
            );
    }
}
