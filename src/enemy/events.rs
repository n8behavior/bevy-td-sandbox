//! Enemy lifecycle events.
//!
//! All events are `EntityEvent`s carrying only the affected entity (plus
//! spawn-time wave context for `EnemySpawned`). Observers query whatever
//! components they need from the entity. This avoids baking per-capability
//! state (e.g. stolen scrap) into event payloads, so an enemy that doesn't
//! steal isn't carrying meaningless fields.
//!
//! Direct template: `tower::events`. Same `EntityEvent` + observer-with-
//! marker-filter pattern.

use bevy::prelude::*;

/// Targeted at an enemy entity that has just reached zero health.
///
/// The shared `check_enemy_death` system triggers this event. The dying
/// entity is **not** despawned synchronously — it gains a `DeathAnimation`
/// and is cleaned up later — so observer queries against `entity` succeed
/// inside `On<EnemyDied>`.
///
/// Default observers in `enemy::systems` handle loot drops, particles,
/// sound, scrap-carrier handoff, and `SplitsOnDeath`. Per-blueprint
/// observers (registered in each enemy plugin) handle bespoke effects
/// filtered by their marker (e.g. screen shake on Boss death).
#[derive(EntityEvent)]
pub struct EnemyDied {
    /// The dying enemy — automatically the event target.
    pub entity: Entity,
}

/// Targeted at an enemy entity that has just reached the map edge while
/// fleeing. Emitted only for `StealsScrap` enemies — fleeing is part of
/// the steal-scrap lifecycle.
#[derive(EntityEvent)]
pub struct EnemyEscaped {
    /// The escaping enemy — automatically the event target.
    pub entity: Entity,
}

/// Triggered immediately after an enemy is spawned by `spawn_from_blueprint`.
///
/// Carries **only the wave number**. Per-capability scaling observers
/// listen on this event and apply their own wave-based scaling formula
/// to the components they own (e.g. `scale_health_on_spawn` reads `Health`,
/// `scale_speed_on_spawn` reads `MoveSpeed`). Adding a new capability
/// that wants wave scaling means adding a new observer — never modifying
/// the event payload or shared code.
#[derive(EntityEvent)]
pub struct EnemySpawned {
    /// The newly spawned enemy — automatically the event target.
    pub entity: Entity,
    /// Wave number this enemy spawned in (1-indexed for display, but
    /// callers should pass whatever value they use as the difficulty key).
    pub wave: u32,
}
