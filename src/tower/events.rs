//! Tower-related events used to decouple shared firing logic from per-tower
//! side effects (sounds, particles, screen shake, etc.).

use bevy::prelude::*;

/// Triggered when a tower fires a shot. Per-tower modules observe this and
/// react with their own sound, vfx, or other side effects, filtered by the
/// tower's marker component.
#[derive(Event)]
pub struct TowerFired {
    pub entity: Entity,
}
