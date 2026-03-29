pub mod components;
pub mod systems;

use bevy::prelude::*;
use crate::states::{GameState, PlayPhase};

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                systems::enemy_movement,
                systems::apply_slow_effects,
                systems::enemy_reached_goal,
                systems::check_enemy_death,
            )
                .run_if(in_state(GameState::Playing))
                .run_if(in_state(PlayPhase::Defending)),
        )
        .add_systems(
            Update,
            systems::update_health_bars
                .run_if(in_state(GameState::Playing)),
        );
    }
}
