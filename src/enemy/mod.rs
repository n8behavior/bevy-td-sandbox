//! Enemy lifecycle: spawning, pathfinding, state transitions, and animations.
//!
//! Enemies spawn at map edges and pathfind toward the scrap pile
//! (`EnemyState::Approaching`). On arrival they steal scrap and flee to the
//! nearest edge (`EnemyState::Fleeing`). Death triggers observer events for
//! loot drops, particles, sound, and boss splitting.
//!
//! Each `EnemyType` variant carries a stat block (`EnemyStats`) that drives
//! health, speed, loot, and visuals. Boss enemies gain traits (`Regeneration`,
//! `Armor`, `SplitsOnDeath`) assigned per wave.

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
                // Speed pipeline: reset → apply slows → movement
                (
                    systems::reset_speed,
                    systems::apply_slow_effects,
                    systems::enemy_movement,
                )
                    .chain(),
                systems::boss_regeneration,
                systems::enemy_reached_pile,
                systems::enemy_escaped,
                systems::check_enemy_death,
                systems::brute_attack_towers,
            )
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
        .add_observer(systems::on_boss_split)
        .add_observer(systems::on_enemy_died_sound)
        .add_observer(systems::on_enemy_died_particles)
        .add_observer(systems::on_enemy_died_shake);
    }
}
