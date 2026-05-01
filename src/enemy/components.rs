//! Enemy module components.
//!
//! This file holds **shared scaffolding** every enemy is built from
//! (`Enemy`, `Health`, `MoveSpeed`, animations, the `HealthBar` child) plus
//! the universal **capability components** any blueprint can opt into
//! (`Regeneration`, `Armor`, `SplitsOnDeath`, `StealsScrap`, `AttacksTowers`).
//!
//! Per-enemy markers and bespoke capabilities live in each enemy's own
//! sub-module (e.g. `enemy::brute::components::BruteAttack`).
//!
//! The blueprint registry (`EnemyBlueprint`, `EnemyRegistry`) lets per-type
//! plugins register themselves so wave/endless code can spawn enemies by
//! name (`registry.lookup("Shambler")`) without touching shared code.
//! This mirrors `tower::components::TowerBlueprint`/`TowerRegistry`.

use bevy::ecs::system::EntityCommands;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

// ---------------------------------------------------------------------------
// Core marker + identity
// ---------------------------------------------------------------------------

/// Marker for any active enemy entity.
#[derive(Component)]
pub struct Enemy;

/// Blueprint name copied onto the entity at spawn time. Observers that need
/// to bucket events by enemy type (stats, debug logs, UI) read this instead
/// of matching on a central enum. Mirrors `tower::components::TowerName`.
#[derive(Component, Clone, Copy, Debug)]
pub struct EnemyName(pub &'static str);

// ---------------------------------------------------------------------------
// Shared per-entity components
// ---------------------------------------------------------------------------

/// Hit points. Optional — a "Ghost" enemy that can't be damaged simply
/// omits this component, and the shared damage/death systems no-op for it.
#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

/// Movement speed. `current` is what `enemy_movement` consumes each tick;
/// `apply_slow_effects` resets it from `base` and applies any active slow.
/// Optional — stationary enemies omit it.
#[derive(Component)]
pub struct MoveSpeed {
    pub base: f32,
    pub current: f32,
}

/// Active slow effect. While present, `current` speed = `base * factor`.
/// Removed when the timer finishes.
#[derive(Component)]
pub struct SlowEffect {
    pub factor: f32,
    pub remaining: Timer,
}

/// Loot dropped on death (in scrap units). Read by the default
/// `EnemyDied` observer that spawns a `ScrapDrop`.
#[derive(Component)]
pub struct LootValue(pub u32);

/// Scale-up animation on spawn (enemies, towers, scrap drops).
#[derive(Component)]
pub struct SpawnAnimation {
    pub timer: Timer,
}

/// Shrink + fade death animation. Despawns the entity when complete.
#[derive(Component)]
pub struct DeathAnimation {
    pub timer: Timer,
}

/// Per-cell random jitter so enemies don't all walk the exact same pixel
/// path. Refreshed each time the agent advances to a new cell.
#[derive(Component)]
pub struct CellJitter(pub Vec2);

/// Brief white flash on damage; restores `original_color` when finished.
#[derive(Component)]
pub struct DamageFlash {
    pub timer: Timer,
    pub original_color: Color,
}

/// Expanding/fading AoE burst visual (shader-driven circle).
#[derive(Component)]
pub struct AoEBurst {
    pub timer: Timer,
    pub max_radius: f32,
}

/// Visual health bar rendered above an enemy sprite.
#[derive(Component)]
pub struct HealthBar {
    pub y_offset: f32,
}

// ---------------------------------------------------------------------------
// Scrap-stealer capability
// ---------------------------------------------------------------------------

/// Capability marker for enemies whose goal is "approach pile → steal
/// scrap → flee to map edge". Without this marker, `enemy_reached_pile`
/// and `enemy_escaped` skip the entity.
#[derive(Component)]
pub struct StealsScrap;

/// State machine for `StealsScrap` enemies.
///
/// `Approaching` → `Fleeing` once the pile is reached. Other capability
/// markers (e.g. `AttacksTowers`) drive different goals and don't read
/// this component.
#[derive(Component, Default, PartialEq, Eq, Debug, Clone, Copy)]
pub enum EnemyState {
    #[default]
    Approaching,
    Fleeing,
}

/// Scrap stolen from the pile that a `StealsScrap` enemy is carrying.
#[derive(Component)]
pub struct StolenScrap(pub u32);

/// Visual decal child indicating an enemy is carrying stolen scrap.
#[derive(Component)]
pub struct ScrapCarrierDecal;

// ---------------------------------------------------------------------------
// Universal capabilities — any blueprint can opt in by inserting these
// ---------------------------------------------------------------------------

/// Heal over time. Driven by the shared `regeneration_system`.
#[derive(Component)]
pub struct Regeneration {
    pub rate: f32,
}

/// Flat damage reduction per hit (minimum 1 damage). Read by
/// `projectile/systems.rs::apply_damage`.
#[derive(Component)]
pub struct Armor {
    pub reduction: f32,
}

/// On death, spawn `count` enemies of `spawn_blueprint` at the death
/// position. Made universal in this refactor — the splitter blueprint
/// chooses what it splits into instead of hardcoding "Shambler".
#[derive(Component)]
pub struct SplitsOnDeath {
    pub count: u32,
    pub spawn_blueprint: &'static str,
}

/// Capability for enemies that attack adjacent towers. Carries the
/// per-enemy attack values so different attackers can have different
/// cooldowns and damage. The shared `attacks_towers_system` filters by
/// this component and applies damage to nearby tower entities.
#[derive(Component)]
pub struct AttacksTowers {
    pub cooldown: Timer,
    pub damage: f32,
    pub range: f32,
}

// ---------------------------------------------------------------------------
// Blueprint registry — direct analog of TowerBlueprint / TowerRegistry
// ---------------------------------------------------------------------------

/// A blueprint describing how to spawn one enemy type. Per-enemy plugins
/// push one of these into `EnemyRegistry` during startup. Mirrors
/// `tower::components::TowerBlueprint`.
///
/// Blueprints carry **only registry metadata** (name, colors). All
/// gameplay components are inserted by `spawn_fn` so each enemy can opt
/// in or out freely. A "Ghost" enemy that can't take damage simply omits
/// `Health` from its `spawn_fn`.
pub struct EnemyBlueprint {
    pub name: &'static str,
    /// Representative color for menu/wave-info displays.
    pub color: Color,
    /// Contrast color for end-of-run kill counts and other panel UI.
    pub ui_color: Color,
    /// Inserts every gameplay component (markers, Sprite, Health,
    /// MoveSpeed, capabilities, etc.) onto the freshly spawned entity.
    pub spawn_fn: fn(&mut EntityCommands),
}

/// Registry of all available enemy types. Per-enemy plugins push their
/// blueprints during startup; wave/endless/spawn helpers look up entries
/// by name.
#[derive(Resource, Default)]
pub struct EnemyRegistry {
    pub blueprints: Vec<EnemyBlueprint>,
}

impl EnemyRegistry {
    /// Find a blueprint by exact name match. Returns `None` if no entry
    /// exists — callers should treat this as a missing registration.
    pub fn lookup(&self, name: &str) -> Option<&EnemyBlueprint> {
        self.blueprints.iter().find(|b| b.name == name)
    }
}

// ---------------------------------------------------------------------------
// Run-stat support: kill counts keyed by blueprint name
// ---------------------------------------------------------------------------

/// Convenience type for stats observers and end-of-run UIs.
pub type EnemyKillCounts = HashMap<&'static str, u32>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enemy_state_default_is_approaching() {
        assert_eq!(EnemyState::default(), EnemyState::Approaching);
    }

    #[test]
    fn registry_lookup_returns_none_for_missing() {
        let reg = EnemyRegistry::default();
        assert!(reg.lookup("Shambler").is_none());
    }

    #[test]
    fn registry_lookup_finds_blueprint() {
        let mut reg = EnemyRegistry::default();
        reg.blueprints.push(EnemyBlueprint {
            name: "TestEnemy",
            color: Color::WHITE,
            ui_color: Color::WHITE,
            spawn_fn: |_| {},
        });
        assert!(reg.lookup("TestEnemy").is_some());
        assert!(reg.lookup("Other").is_none());
    }
}
