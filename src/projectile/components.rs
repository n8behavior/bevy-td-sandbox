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
