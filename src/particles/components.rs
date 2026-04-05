use bevy::prelude::*;

/// Small sprite burst at projectile impact point.
#[derive(Component)]
pub struct ImpactParticle {
    pub timer: Timer,
    pub velocity: Vec2,
}

/// Glint/sparkle effect when scrap drops land.
#[derive(Component)]
pub struct SparkleParticle {
    pub timer: Timer,
}

/// Outward burst on enemy death, color-matched to enemy type.
#[derive(Component)]
pub struct DeathParticle {
    pub timer: Timer,
    pub velocity: Vec2,
}
