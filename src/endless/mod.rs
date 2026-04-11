//! Endless mode — infinite, escalating enemy spawns with no build phases.
//!
//! Unlike classic (wave-based) mode, endless mode skips the Building phase
//! entirely and spawns enemies continuously with time-based difficulty
//! scaling. The three core systems form a pipeline:
//!
//! 1. **[`systems::skip_building_phase`]** — on entering `Playing`, jumps
//!    straight to `Defending`.
//! 2. **[`systems::init_endless`]** — on entering `Defending`, inserts the
//!    [`resources::EndlessSpawner`] resource that tracks elapsed time,
//!    spawn cadence, and total enemies spawned.
//! 3. **[`systems::endless_spawn_enemies`]** — runs every fixed tick,
//!    picks an enemy type via [`systems::pick_enemy_type`], applies
//!    HP/speed scaling via [`systems::compute_scaling`], and spawns it.
//! 4. **[`systems::endless_check_game_over`]** — runs every frame, checks
//!    the three-fold bankruptcy gate (pile empty AND no drops AND no
//!    enemies) and transitions to `GameOver` when recovery is impossible.
//!
//! Spawning uses [`FixedUpdate`] for deterministic, frame-rate-independent
//! timing (matching wave mode). Game-over detection uses [`Update`] because
//! it is a reactive state check with no timing sensitivity.

pub mod resources;
pub mod systems;

use crate::states::{GameMode, GameState, PlayPhase};
use bevy::prelude::*;

fn is_endless(mode: Res<GameMode>) -> bool {
    *mode == GameMode::Endless
}

pub struct EndlessPlugin;

impl Plugin for EndlessPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Playing),
            systems::skip_building_phase.run_if(is_endless),
        )
        .add_systems(
            OnEnter(PlayPhase::Defending),
            systems::init_endless.run_if(is_endless),
        )
        // FixedUpdate for deterministic spawn timing (see module docs).
        .add_systems(
            FixedUpdate,
            systems::endless_spawn_enemies
                .run_if(in_state(GameState::Playing))
                .run_if(in_state(PlayPhase::Defending))
                .run_if(is_endless),
        )
        .add_systems(
            Update,
            systems::endless_check_game_over
                .run_if(in_state(GameState::Playing))
                .run_if(in_state(PlayPhase::Defending))
                .run_if(is_endless),
        );
    }
}
