pub mod components;
pub mod systems;

use crate::states::{GameState, PlayPhase};
use bevy::prelude::*;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (
                // Game logic first
                (
                    systems::enemy_movement,
                    systems::search_wander_movement,
                    systems::apply_slow_effects,
                    systems::boss_regeneration,
                    systems::enemy_reached_pile,
                    systems::enemy_escaped,
                    systems::check_enemy_death,
                ),
                // Cleanup runs after all game logic
                systems::cleanup_dead,
            )
                .chain()
                .run_if(in_state(GameState::Playing))
                .run_if(in_state(PlayPhase::Defending)),
        )
        .add_systems(
            Update,
            (
                systems::update_health_bars,
                systems::animate_spawn,
                systems::animate_death,
                systems::animate_damage_flash,
                systems::animate_aoe_burst,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .add_observer(systems::on_boss_split);
    }
}
