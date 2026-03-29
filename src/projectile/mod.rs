pub mod components;
pub mod systems;

use bevy::prelude::*;
use crate::states::{GameState, PlayPhase};

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                systems::projectile_movement,
                systems::projectile_hit_detection,
            )
                .chain()
                .run_if(in_state(GameState::Playing))
                .run_if(in_state(PlayPhase::Defending)),
        );
    }
}
