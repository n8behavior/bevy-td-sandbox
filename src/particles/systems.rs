use bevy::prelude::*;
use rand::Rng;

use super::components::*;

// ---------------------------------------------------------------------------
// Spawn helpers (called from other modules at trigger points)
// ---------------------------------------------------------------------------

/// Spawn 3-5 small sprites at `position` with random velocity, fading over 0.3s.
pub fn spawn_impact_particles(commands: &mut Commands, position: Vec2, color: Color) {
    let mut rng = rand::rng();
    let count = rng.random_range(3..=5);
    for _ in 0..count {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let speed = rng.random_range(40.0..100.0);
        let velocity = Vec2::new(angle.cos(), angle.sin()) * speed;
        commands.spawn((
            ImpactParticle {
                timer: Timer::from_seconds(0.3, TimerMode::Once),
                velocity,
            },
            Sprite::from_color(color.with_alpha(0.9), Vec2::splat(3.0)),
            Transform::from_translation(position.extend(3.0)),
        ));
    }
}

/// Spawn 2-3 tiny gold/white sparkle sprites at `position`, drifting upward.
pub fn spawn_scrap_sparkle(commands: &mut Commands, position: Vec2) {
    let mut rng = rand::rng();
    let count = rng.random_range(2..=3);
    for _ in 0..count {
        let offset = Vec2::new(rng.random_range(-4.0..4.0), rng.random_range(-4.0..4.0));
        commands.spawn((
            SparkleParticle {
                timer: Timer::from_seconds(0.4, TimerMode::Once),
            },
            Sprite::from_color(Color::srgba(1.0, 0.95, 0.6, 0.9), Vec2::splat(2.0)),
            Transform::from_translation((position + offset).extend(3.0)),
        ));
    }
}

/// Spawn 5-8 sprites at `position` with outward burst velocity, color-matched.
pub fn spawn_death_particles(commands: &mut Commands, position: Vec2, color: Color) {
    let mut rng = rand::rng();
    let count = rng.random_range(5..=8);
    for _ in 0..count {
        let angle = rng.random_range(0.0..std::f32::consts::TAU);
        let speed = rng.random_range(30.0..80.0);
        let velocity = Vec2::new(angle.cos(), angle.sin()) * speed;
        commands.spawn((
            DeathParticle {
                timer: Timer::from_seconds(0.4, TimerMode::Once),
                velocity,
            },
            Sprite::from_color(color.with_alpha(0.8), Vec2::splat(4.0)),
            Transform::from_translation(position.extend(3.0)),
        ));
    }
}

// ---------------------------------------------------------------------------
// Per-frame animate systems
// ---------------------------------------------------------------------------

pub fn animate_impact_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut ImpactParticle, &mut Sprite, &mut Transform)>,
    time: Res<Time>,
) {
    for (entity, mut particle, mut sprite, mut tf) in &mut query {
        particle.timer.tick(time.delta());
        let t = particle.timer.fraction();

        tf.translation += particle.velocity.extend(0.0) * time.delta_secs();
        sprite.color = sprite.color.with_alpha(0.9 * (1.0 - t));
        tf.scale = Vec3::splat(1.0 - t * 0.6);

        if particle.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn animate_sparkle_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut SparkleParticle, &mut Sprite, &mut Transform)>,
    time: Res<Time>,
) {
    for (entity, mut particle, mut sprite, mut tf) in &mut query {
        particle.timer.tick(time.delta());
        let t = particle.timer.fraction();

        // Float upward.
        tf.translation.y += 20.0 * time.delta_secs();

        // Oscillate alpha for glint effect.
        let glint = ((t * 8.0 * std::f32::consts::PI).sin() * 0.5 + 0.5) * (1.0 - t);
        sprite.color = sprite.color.with_alpha(glint);

        if particle.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

pub fn animate_death_particles(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DeathParticle, &mut Sprite, &mut Transform)>,
    time: Res<Time>,
) {
    for (entity, mut particle, mut sprite, mut tf) in &mut query {
        particle.timer.tick(time.delta());
        let t = particle.timer.fraction();

        tf.translation += particle.velocity.extend(0.0) * time.delta_secs();
        // Slight gravity pull.
        particle.velocity.y -= 60.0 * time.delta_secs();
        sprite.color = sprite.color.with_alpha(0.8 * (1.0 - t));

        if particle.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
