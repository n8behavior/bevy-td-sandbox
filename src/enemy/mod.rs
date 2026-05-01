//! Enemy module: lifecycle, capability components, and per-type plugins.
//!
//! Each enemy type lives in its own subdirectory (`shambler/`, `runner/`,
//! `brute/`, `boss/`, `raider/`) and registers an `EnemyBlueprint` via
//! its plugin during `Startup`. Wave/endless code looks up blueprints by
//! name (`registry.lookup("Shambler")`) and calls
//! `spawn::spawn_from_blueprint` — adding a new enemy type is creating a
//! new module and adding one line to `EnemyPlugin::build`.
//!
//! Shared scaffolding (movement, slow pipeline, animations, death
//! detection, default lifecycle observers, capability systems) lives in
//! `systems.rs`. Universal capabilities (`Regeneration`, `Armor`,
//! `SplitsOnDeath`, `StealsScrap`, `AttacksTowers`) live in
//! `components.rs` so any blueprint can opt in.

pub mod boss;
pub mod brute;
pub mod components;
pub mod events;
pub mod raider;
pub mod runner;
pub mod shambler;
pub mod spawn;
pub mod systems;

use crate::states::{GameState, PlayPhase};
use bevy::prelude::*;

use components::EnemyRegistry;

pub struct EnemyPlugin;

impl Plugin for EnemyPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EnemyRegistry>()
            .add_observer(systems::on_enemy_died_sound)
            .add_observer(systems::on_enemy_died_particles)
            .add_observer(systems::on_splits_on_death)
            .add_observer(systems::scale_health_on_spawn)
            .add_observer(systems::scale_speed_on_spawn)
            .add_plugins((
                shambler::ShamblerPlugin,
                runner::RunnerPlugin,
                brute::BrutePlugin,
                boss::BossPlugin,
                raider::RaiderPlugin,
            ))
            .add_systems(
                FixedUpdate,
                (
                    // Speed pipeline: reset → apply slows → movement.
                    (
                        systems::reset_speed,
                        systems::apply_slow_effects,
                        systems::enemy_movement,
                    )
                        .chain(),
                    systems::regeneration_system,
                    systems::enemy_reached_pile,
                    systems::enemy_escaped,
                    systems::check_enemy_death,
                    systems::attacks_towers_system,
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
            );
    }
}
