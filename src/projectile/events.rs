//! Projectile triggers.

use bevy::prelude::*;

use crate::records::DamageSource;

use super::components::AoEPayload;

/// Targeted at the enemy a projectile just hit. `projectile_hit_detection`
/// despawns the projectile and triggers this, copying the fields from the
/// projectile.
///
/// `ProjectilePlugin` registers one observer per effect: direct damage,
/// splash, impact particles, and the AoE burst with screen shake. A new
/// hit reaction is a new observer; hit detection doesn't change.
///
/// Bevy runs the observers of one event in an arbitrary order, so none of
/// them may depend on another. An observer that needs the result of a hit
/// reads `DamageDealt`, not `Health`.
#[derive(EntityEvent, Clone, Copy, Debug)]
pub struct ProjectileHit {
    /// The enemy hit — automatically the event target.
    pub entity: Entity,
    /// Who fired the projectile.
    pub source: DamageSource,
    /// Where the projectile was when it hit.
    pub position: Vec3,
    /// Direct-hit damage, before armor.
    pub damage: f32,
    /// Splash around `position`, for Explosive and Mortar shells.
    pub aoe: Option<AoEPayload>,
}
