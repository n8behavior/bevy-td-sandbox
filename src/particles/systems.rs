use bevy::prelude::*;
use rand::Rng;

use super::components::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

// Impact particles
const IMPACT_COUNT_MIN: u32 = 3;
const IMPACT_COUNT_MAX: u32 = 5;
const IMPACT_SPEED_MIN: f32 = 40.0;
const IMPACT_SPEED_MAX: f32 = 100.0;
const IMPACT_DURATION: f32 = 0.3;
const IMPACT_INITIAL_ALPHA: f32 = 0.9;
const IMPACT_SPRITE_SIZE: f32 = 3.0;
const IMPACT_SHRINK: f32 = 0.6;

// Sparkle particles
const SPARKLE_COUNT_MIN: u32 = 2;
const SPARKLE_COUNT_MAX: u32 = 3;
const SPARKLE_DURATION: f32 = 0.4;
const SPARKLE_OFFSET_RANGE: f32 = 4.0;
const SPARKLE_INITIAL_ALPHA: f32 = 0.9;
const SPARKLE_SPRITE_SIZE: f32 = 2.0;
const SPARKLE_FLOAT_SPEED: f32 = 20.0;
const SPARKLE_GLINT_FREQ: f32 = 8.0;

// Death particles
const DEATH_COUNT_MIN: u32 = 5;
const DEATH_COUNT_MAX: u32 = 8;
const DEATH_SPEED_MIN: f32 = 30.0;
const DEATH_SPEED_MAX: f32 = 80.0;
const DEATH_DURATION: f32 = 0.4;
const DEATH_INITIAL_ALPHA: f32 = 0.8;
const DEATH_SPRITE_SIZE: f32 = 4.0;
const DEATH_GRAVITY: f32 = 60.0;

// Shared
const PARTICLE_Z: f32 = 3.0;

// ---------------------------------------------------------------------------
// Pure functions (extracted for testability)
// ---------------------------------------------------------------------------

/// Linear alpha fadeout from `initial` to 0 over normalized time `t` (0..1).
pub(crate) fn linear_alpha(initial: f32, t: f32) -> f32 {
    initial * (1.0 - t)
}

/// Oscillating glint alpha that fades to zero over normalized time `t` (0..1).
///
/// Produces a sine-wave glint at `frequency` cycles modulated by a linear
/// fade, so the sparkle pulses brightly at first and dims to nothing.
pub(crate) fn sparkle_alpha(t: f32, frequency: f32) -> f32 {
    ((t * frequency * std::f32::consts::PI).sin() * 0.5 + 0.5) * (1.0 - t)
}

/// Scale that shrinks from 1.0 toward `(1.0 - factor)` over normalized time `t`.
pub(crate) fn shrink_scale(t: f32, factor: f32) -> Vec3 {
    Vec3::splat(1.0 - t * factor)
}

/// Random velocity vector with uniform angle and speed in `[speed_min, speed_max)`.
pub(crate) fn random_radial_velocity(rng: &mut impl Rng, speed_min: f32, speed_max: f32) -> Vec2 {
    let angle = rng.random_range(0.0..std::f32::consts::TAU);
    let speed = rng.random_range(speed_min..speed_max);
    Vec2::new(angle.cos(), angle.sin()) * speed
}

// ---------------------------------------------------------------------------
// Spawn helpers (called from other modules at trigger points)
// ---------------------------------------------------------------------------

/// Spawn 3-5 small sprites at `position` with random velocity, fading over 0.3s.
pub fn spawn_impact_particles(
    commands: &mut Commands,
    position: Vec2,
    color: Color,
    rng: &mut impl Rng,
) {
    let count = rng.random_range(IMPACT_COUNT_MIN..=IMPACT_COUNT_MAX);
    for _ in 0..count {
        let velocity = random_radial_velocity(rng, IMPACT_SPEED_MIN, IMPACT_SPEED_MAX);
        commands.spawn((
            ImpactParticle {
                timer: Timer::from_seconds(IMPACT_DURATION, TimerMode::Once),
                velocity,
            },
            Sprite::from_color(
                color.with_alpha(IMPACT_INITIAL_ALPHA),
                Vec2::splat(IMPACT_SPRITE_SIZE),
            ),
            Transform::from_translation(position.extend(PARTICLE_Z)),
        ));
    }
}

/// Spawn 2-3 tiny gold/white sparkle sprites at `position`, drifting upward.
pub fn spawn_scrap_sparkle(commands: &mut Commands, position: Vec2, rng: &mut impl Rng) {
    let count = rng.random_range(SPARKLE_COUNT_MIN..=SPARKLE_COUNT_MAX);
    for _ in 0..count {
        let offset = Vec2::new(
            rng.random_range(-SPARKLE_OFFSET_RANGE..SPARKLE_OFFSET_RANGE),
            rng.random_range(-SPARKLE_OFFSET_RANGE..SPARKLE_OFFSET_RANGE),
        );
        commands.spawn((
            SparkleParticle {
                timer: Timer::from_seconds(SPARKLE_DURATION, TimerMode::Once),
            },
            Sprite::from_color(
                Color::srgba(1.0, 0.95, 0.6, SPARKLE_INITIAL_ALPHA),
                Vec2::splat(SPARKLE_SPRITE_SIZE),
            ),
            Transform::from_translation((position + offset).extend(PARTICLE_Z)),
        ));
    }
}

/// Spawn 5-8 sprites at `position` with outward burst velocity, color-matched.
pub fn spawn_death_particles(
    commands: &mut Commands,
    position: Vec2,
    color: Color,
    rng: &mut impl Rng,
) {
    let count = rng.random_range(DEATH_COUNT_MIN..=DEATH_COUNT_MAX);
    for _ in 0..count {
        let velocity = random_radial_velocity(rng, DEATH_SPEED_MIN, DEATH_SPEED_MAX);
        commands.spawn((
            DeathParticle {
                timer: Timer::from_seconds(DEATH_DURATION, TimerMode::Once),
                velocity,
            },
            Sprite::from_color(
                color.with_alpha(DEATH_INITIAL_ALPHA),
                Vec2::splat(DEATH_SPRITE_SIZE),
            ),
            Transform::from_translation(position.extend(PARTICLE_Z)),
        ));
    }
}

// ---------------------------------------------------------------------------
// Per-frame animation systems
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
        sprite.color = sprite
            .color
            .with_alpha(linear_alpha(IMPACT_INITIAL_ALPHA, t));
        tf.scale = shrink_scale(t, IMPACT_SHRINK);

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

        tf.translation.y += SPARKLE_FLOAT_SPEED * time.delta_secs();
        sprite.color = sprite
            .color
            .with_alpha(sparkle_alpha(t, SPARKLE_GLINT_FREQ));

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
        particle.velocity.y -= DEATH_GRAVITY * time.delta_secs();
        sprite.color = sprite
            .color
            .with_alpha(linear_alpha(DEATH_INITIAL_ALPHA, t));

        if particle.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    // -- linear_alpha --

    #[test]
    fn linear_alpha_full_at_start() {
        assert_eq!(linear_alpha(0.9, 0.0), 0.9);
    }

    #[test]
    fn linear_alpha_zero_at_end() {
        assert!(linear_alpha(0.9, 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn linear_alpha_midpoint() {
        assert!((linear_alpha(0.8, 0.5) - 0.4).abs() < 1e-6);
    }

    // -- sparkle_alpha --

    #[test]
    fn sparkle_alpha_zero_at_end() {
        assert!(sparkle_alpha(1.0, 8.0).abs() < 1e-6);
    }

    #[test]
    fn sparkle_alpha_non_negative() {
        for i in 0..=100 {
            let t = i as f32 / 100.0;
            assert!(sparkle_alpha(t, 8.0) >= 0.0, "negative at t={t}");
        }
    }

    #[test]
    fn sparkle_alpha_oscillates() {
        let mut found_increase = false;
        let mut prev = sparkle_alpha(0.0, 8.0);
        for i in 1..=100 {
            let t = i as f32 / 100.0;
            let val = sparkle_alpha(t, 8.0);
            if val > prev {
                found_increase = true;
                break;
            }
            prev = val;
        }
        assert!(found_increase, "sparkle alpha should oscillate");
    }

    // -- shrink_scale --

    #[test]
    fn shrink_scale_full_at_start() {
        assert_eq!(shrink_scale(0.0, 0.6), Vec3::ONE);
    }

    #[test]
    fn shrink_scale_shrinks_at_end() {
        let result = shrink_scale(1.0, 0.6);
        assert!((result.x - 0.4).abs() < 1e-6);
    }

    // -- random_radial_velocity --

    #[test]
    fn random_radial_velocity_within_speed_range() {
        let mut rng = SmallRng::seed_from_u64(42);
        for _ in 0..100 {
            let vel = random_radial_velocity(&mut rng, 40.0, 100.0);
            let speed = vel.length();
            assert!(
                (40.0 - 1e-3..=100.0 + 1e-3).contains(&speed),
                "speed {speed} outside [40, 100]"
            );
        }
    }

    #[test]
    fn random_radial_velocity_deterministic() {
        let mut rng_a = SmallRng::seed_from_u64(99);
        let mut rng_b = SmallRng::seed_from_u64(99);
        let a = random_radial_velocity(&mut rng_a, 30.0, 80.0);
        let b = random_radial_velocity(&mut rng_b, 30.0, 80.0);
        assert_eq!(a, b);
    }
}
