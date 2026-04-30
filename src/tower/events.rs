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

/// Targeted at a tower entity that just transitioned into `TowerState::Rubble`.
/// The shared `on_tower_becomes_rubble` system handles universal effects
/// (sprite tint, turret-phase reset, sound); per-capability observers
/// listen on this event to despawn their own visual children (range rings,
/// auras, magnet rings) so each capability owns its lifecycle cleanup.
#[derive(EntityEvent)]
pub struct TowerBecameRubble {
    /// The rubble tower — automatically the event target.
    pub entity: Entity,
}

/// Targeted at a tower entity that just transitioned out of `TowerState::Rubble`
/// back into `Active` via `apply_repair`. Per-capability observers listen on
/// this event to re-insert their own ring config components, retriggering
/// the reactive `Added<...RingConfig>` spawners that build the visual children.
#[derive(EntityEvent)]
pub struct TowerRepaired {
    /// The repaired tower — automatically the event target.
    pub entity: Entity,
}
