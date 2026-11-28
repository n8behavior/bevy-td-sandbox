//! Gameplay records: the event stream that stats, diagnostics and dev views
//! read.
//!
//! A record is a buffered [`Message`] that states one gameplay fact, such as
//! [`DamageDealt`]. Readers count records in batches. A record copies the
//! stable names it needs ([`TowerId`], the enemy's `EnemyName`), so a reader
//! never has to look up an entity that may be gone by the time it reads.
//!
//! Records are for counting, not for reacting. Something that must happen
//! now (damage, a flash, particles) is a trigger: an `Event` with observers,
//! such as `projectile::events::ProjectileHit`.

use bevy::prelude::*;

use crate::terrain::components::Terrain;
use crate::tower::components::TowerId;

/// What dealt a point of damage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DamageSource {
    /// A placed tower. The id outlives the tower, so a shot that lands
    /// after its tower is sold still names it.
    Tower(TowerId),
    /// A damaging terrain cell. Only `Radioactive` deals damage today.
    Terrain(Terrain),
}

/// One application of damage to an enemy. Written by
/// `enemy::damage::damage_enemy`, the only code that lowers enemy `Health`.
///
/// `dealt + overkill` is the damage after armor.
#[derive(Message, Clone, Copy, Debug)]
pub struct DamageDealt {
    pub source: DamageSource,
    /// The enemy hit. Bevy never reuses an `Entity` value, so this stays a
    /// fine key after the enemy despawns.
    pub target: Entity,
    /// The enemy's `EnemyName`; every spawned enemy has one.
    pub target_kind: Option<&'static str>,
    /// Damage before armor.
    pub raw: f32,
    /// Health removed: after armor, at most the health left, never negative.
    pub dealt: f32,
    /// Damage after armor beyond the health left.
    pub overkill: f32,
}

/// Registers the record messages.
pub struct RecordsPlugin;

impl Plugin for RecordsPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<DamageDealt>();
    }
}
