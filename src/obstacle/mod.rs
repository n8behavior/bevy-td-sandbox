pub mod components;
pub mod systems;

use crate::states::GameState;
use bevy::prelude::*;

pub struct ObstaclePlugin;

impl Plugin for ObstaclePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Playing),
            systems::generate_obstacles.after(crate::pile::init_pile),
        );
    }
}
