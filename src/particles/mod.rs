pub mod components;
pub mod systems;

use crate::states::GameState;
use bevy::prelude::*;

pub struct ParticlesPlugin;

impl Plugin for ParticlesPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                systems::animate_impact_particles,
                systems::animate_sparkle_particles,
                systems::animate_death_particles,
            )
                .run_if(in_state(GameState::Playing)),
        );
    }
}
