pub mod components;
pub mod systems;

use crate::states::GameState;
use bevy::prelude::*;

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Playing),
            (
                systems::compute_grid_config,
                (
                    systems::setup_camera,
                    systems::spawn_nav_grid,
                    systems::spawn_grid_visuals,
                ),
            )
                .chain(),
        );
    }
}
