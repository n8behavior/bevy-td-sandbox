use bevy::prelude::*;

use crate::records::DamageSource;

#[derive(Component)]
pub struct Projectile {
    pub damage: f32,
    pub speed: f32,
    pub target: Entity,
    /// Who fired it, copied at spawn: a projectile that lands after its
    /// tower is sold still names the tower.
    pub source: DamageSource,
}

#[derive(Component, Clone, Copy, Debug)]
pub struct AoEPayload {
    pub radius: f32,
    pub damage: f32,
}

/// Emits trail particles at intervals.
#[derive(Component)]
pub struct TrailEmitter {
    pub timer: Timer,
    pub color: Color,
    pub particle_size: f32,
    pub particle_lifetime: f32,
}

/// A fading trail particle sprite.
#[derive(Component)]
pub struct TrailParticle {
    pub timer: Timer,
}
