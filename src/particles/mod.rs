//! Particle effects: short-lived sprites for visual feedback.
//!
//! **Lifecycle:** Other modules call spawn helpers (e.g. [`systems::spawn_impact_particles`])
//! to create particle entities. Per-frame animation systems tick each particle's timer,
//! update position/alpha/scale, and despawn the entity when the timer finishes.

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
