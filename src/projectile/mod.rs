//! Projectile lifecycle: movement, hit detection, damage, and trail particles.
//!
//! Towers spawn projectiles via `spawn_projectile` in the tower module.
//! Each `FixedUpdate`, projectiles move toward their target and are checked
//! for hits. On impact, damage is applied (with optional AoE splash and armor
//! reduction). Trail particles are spawned and faded in `Update` (visual only).

pub mod components;
pub mod systems;

use crate::states::{GameState, PlayPhase};
use bevy::prelude::*;

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
        )
        .add_systems(
            Update,
            (systems::emit_trail_particles, systems::fade_trail_particles)
                .run_if(in_state(GameState::Playing))
                .run_if(in_state(PlayPhase::Defending)),
        );
    }
}
