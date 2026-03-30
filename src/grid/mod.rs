pub mod components;
pub mod systems;

use bevy::prelude::*;
use crate::states::GameState;

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Playing),
            (
                systems::compute_grid_config,
                (systems::setup_camera, systems::setup_grid),
            )
                .chain(),
        )
        .add_systems(
            Update,
            systems::animate_special_cell_glow.run_if(in_state(GameState::Playing)),
        );
    }
}
