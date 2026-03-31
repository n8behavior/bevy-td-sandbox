pub mod components;
pub mod placement;
pub mod systems;
pub mod types;

use bevy::prelude::*;
use crate::states::{GameState, PlayPhase};

use components::TowerRegistry;

pub struct TowerPlugin;

impl Plugin for TowerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<placement::SelectedTower>()
            .init_resource::<TowerRegistry>()
            .add_plugins((
                types::ScrapGunPlugin,
                types::TarPitPlugin,
                types::ExplosivePlugin,
                types::RailgunPlugin,
            ))
            .add_systems(
                Update,
                (
                    placement::handle_tower_selection,
                    placement::update_placing_tower,
                    placement::tint_placing_tower,
                    placement::confirm_tower_placement,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (systems::turret_state_machine, systems::slow_aura)
                    .run_if(in_state(GameState::Playing))
                    .run_if(in_state(PlayPhase::Defending)),
            )
            .add_systems(
                Update,
                systems::rotate_towers_to_target
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
