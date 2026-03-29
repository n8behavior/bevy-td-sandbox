pub mod components;
pub mod systems;

use bevy::prelude::*;
use crate::states::GameState;

pub struct GridPlugin;

impl Plugin for GridPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Playing),
            (systems::setup_camera, systems::setup_grid),
        );
    }
}
