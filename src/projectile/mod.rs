//! Projectile lifecycle: movement, hit detection, hit effects, and trail
//! particles.
//!
//! Towers spawn projectiles via `spawn_projectile` in the tower module.
//! Each `FixedUpdate`, projectiles move toward their target and are checked
//! for hits. A hit despawns the projectile and triggers
//! [`events::ProjectileHit`] on the target. Each effect of a hit is its own
//! observer: direct damage (with armor), AoE splash, impact particles, and
//! the AoE burst with screen shake. Trail particles are spawned and faded
//! in `Update` (visual only).

pub mod components;
pub mod events;
pub mod systems;

use crate::enemy::damage::EnemyDamageSystems;
use crate::states::{GameState, PlayPhase};
use bevy::prelude::*;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(systems::on_projectile_hit_damage)
            .add_observer(systems::on_projectile_hit_splash)
            .add_observer(systems::on_projectile_hit_particles)
            .add_observer(systems::on_projectile_hit_burst)
            .add_systems(
                FixedUpdate,
                (
                    systems::projectile_movement,
                    systems::projectile_hit_detection.in_set(EnemyDamageSystems),
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
