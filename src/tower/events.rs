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

/// Targeted at a tower entity to request that it fire at the given enemy.
/// The shared turret state machine emits this when its decision logic
/// reaches `Fire`; observers handle the actual projectile spawn.
///
/// `TowerPlugin` registers a default observer that reads the tower's
/// `ProjectileVisuals` and optional `AoEOnHit` payload and spawns a standard
/// projectile. Towers with bespoke firing behavior (e.g. arcing mortars)
/// can opt out by adding the `CustomFire` marker component and registering
/// their own observer.
#[derive(EntityEvent)]
pub struct TowerWantsToFire {
    /// The tower firing the shot — automatically the event target.
    pub entity: Entity,
    /// The enemy entity to spawn the projectile against.
    pub target: Entity,
    /// Final damage for the shot (already scaled by tower effectiveness).
    pub damage: f32,
}
