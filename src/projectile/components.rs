use bevy::prelude::*;

#[derive(Component)]
pub struct Projectile {
    pub damage: f32,
    pub speed: f32,
    pub target: Entity,
}

#[derive(Component)]
pub struct AoEPayload {
    pub radius: f32,
    pub damage: f32,
}

#[derive(Component)]
pub struct SlowPayload {
    pub factor: f32,
    pub duration: f32,
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
