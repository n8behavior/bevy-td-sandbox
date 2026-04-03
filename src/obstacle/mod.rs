pub mod components;
pub mod systems;

use bevy::prelude::*;
use crate::states::GameState;

pub struct ObstaclePlugin;

impl Plugin for ObstaclePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Playing),
            systems::generate_obstacles.after(crate::pile::init_pile),
        );
    }
}
