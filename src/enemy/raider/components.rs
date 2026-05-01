//! Raider-specific components.

use bevy::prelude::*;

/// Marker for Raider enemies.
#[derive(Component)]
pub struct Raider;

/// Tower the Raider is currently hunting. `None` until the per-enemy
/// system picks one. The system clears this back to `None` when the
/// targeted tower despawns or stops being a valid target.
#[derive(Component)]
pub struct RaiderTarget(pub Option<Entity>);
