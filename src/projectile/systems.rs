use bevy::prelude::*;

use bevy::sprite_render::MeshMaterial2d;

use crate::camera::components::ScreenShake;
use crate::enemy::components::{AoEBurst, Armor, Enemy, EnemyName, Health};
use crate::enemy::damage::damage_enemy;
use crate::enemy::systems::start_damage_flash;
use crate::particles::systems::spawn_impact_particles;
use crate::records::DamageDealt;
use crate::shader::{CircleMaterial, CircleMesh};
use crate::states::PlayPhase;

use super::components::*;
use super::events::ProjectileHit;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Distance at which a projectile snaps to its target and registers a hit.
/// Shared by `projectile_movement` (snap) and `projectile_hit_detection`.
const HIT_DISTANCE: f32 = 5.0;

/// Duration of the expanding AoE burst visual.
const AOE_BURST_SECS: f32 = 0.3;

/// Initial sprite scale of the AoE burst circle.
const AOE_BURST_SCALE: f32 = 4.0;

/// Screen shake intensity triggered by an AoE hit.
const AOE_SHAKE_INTENSITY: f32 = 3.0;

/// Screen shake duration for AoE hits.
const AOE_SHAKE_SECS: f32 = 0.25;

/// Screen shake per-frame decay rate for AoE hits.
const AOE_SHAKE_DECAY: f32 = 0.05;

/// Color of the particles spawned where a projectile hits.
const IMPACT_COLOR: Color = Color::srgba(1.0, 0.95, 0.6, 0.9);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Move `current` toward `target` by at most `speed * dt` units.
/// Snaps to target when within `HIT_DISTANCE` or when the step would
/// overshoot, ensuring projectiles never phase through their target.
pub(crate) fn move_toward(current: Vec3, target: Vec3, speed: f32, dt: f32) -> Vec3 {
    let direction = target - current;
    let distance = direction.length();
    if distance < HIT_DISTANCE {
        return target;
    }
    let step = direction.normalize() * speed * dt;
    if step.length() >= distance {
        target
    } else {
        current + step
    }
}

/// Linear damage falloff: full damage at center, zero at edge.
/// Clamped so distances beyond radius return 0.
pub(crate) fn aoe_falloff(dist: f32, radius: f32) -> f32 {
    1.0 - (dist / radius).clamp(0.0, 1.0)
}

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

pub fn projectile_movement(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &Projectile, &mut Transform)>,
    targets: Query<&Transform, (With<Enemy>, Without<Projectile>)>,
    time: Res<Time>,
) {
    for (entity, proj, mut proj_tf) in &mut projectiles {
        let Ok(target_tf) = targets.get(proj.target) else {
            // Target gone -- despawn projectile
            commands.entity(entity).despawn();
            continue;
        };
        proj_tf.translation = move_toward(
            proj_tf.translation,
            target_tf.translation,
            proj.speed,
            time.delta_secs(),
        );
    }
}

/// Detect projectiles within `HIT_DISTANCE` of their target: despawn each
/// one and trigger `ProjectileHit` on the target. The hit's effects are
/// the observers of `ProjectileHit`. A projectile whose target is gone is
/// despawned with no hit.
pub fn projectile_hit_detection(
    mut commands: Commands,
    projectiles: Query<(Entity, &Projectile, &Transform, Option<&AoEPayload>)>,
    targets: Query<&Transform, With<Enemy>>,
) {
    for (proj_entity, proj, proj_tf, aoe) in &projectiles {
        let Ok(target_tf) = targets.get(proj.target) else {
            commands.entity(proj_entity).despawn();
            continue;
        };

        if proj_tf.translation.distance(target_tf.translation) > HIT_DISTANCE {
            continue;
        }

        commands.entity(proj_entity).despawn();
        commands.trigger(ProjectileHit {
            entity: proj.target,
            source: proj.source,
            position: proj_tf.translation,
            damage: proj.damage,
            aoe: aoe.copied(),
        });
    }
}

// ---------------------------------------------------------------------------
// ProjectileHit observers (one per effect; see `ProjectileHit` on ordering)
// ---------------------------------------------------------------------------

/// Damage the enemy hit, through its armor, and flash it.
pub fn on_projectile_hit_damage(
    hit: On<ProjectileHit>,
    mut commands: Commands,
    mut enemies: Query<(&mut Health, Option<&Armor>, Option<&EnemyName>), With<Enemy>>,
    mut records: MessageWriter<DamageDealt>,
) {
    let Ok((mut health, armor, name)) = enemies.get_mut(hit.entity) else {
        return;
    };
    damage_enemy(
        hit.entity,
        name.map(|n| n.0),
        &mut health,
        armor.map(|a| a.reduction),
        hit.damage,
        hit.source,
        &mut records,
    );
    commands.entity(hit.entity).queue(start_damage_flash);
}

/// Splash: damage every other enemy within the AoE radius, with linear
/// falloff and through its armor, and flash it. No-op without `aoe`.
pub fn on_projectile_hit_splash(
    hit: On<ProjectileHit>,
    mut commands: Commands,
    mut enemies: Query<
        (
            Entity,
            &mut Health,
            &Transform,
            Option<&Armor>,
            Option<&EnemyName>,
        ),
        With<Enemy>,
    >,
    mut records: MessageWriter<DamageDealt>,
) {
    let Some(aoe) = hit.aoe else {
        return;
    };
    for (entity, mut health, tf, armor, name) in &mut enemies {
        if entity == hit.entity {
            continue;
        }
        let dist = tf.translation.distance(hit.position);
        if dist > aoe.radius {
            continue;
        }
        damage_enemy(
            entity,
            name.map(|n| n.0),
            &mut health,
            armor.map(|a| a.reduction),
            aoe.damage * aoe_falloff(dist, aoe.radius),
            hit.source,
            &mut records,
        );
        commands.entity(entity).queue(start_damage_flash);
    }
}

/// Spawn impact particles where the projectile hit.
pub fn on_projectile_hit_particles(hit: On<ProjectileHit>, mut commands: Commands) {
    spawn_impact_particles(
        &mut commands,
        hit.position.truncate(),
        IMPACT_COLOR,
        &mut rand::rng(),
    );
}

/// Spawn the expanding AoE burst and shake the screen. No-op without `aoe`.
pub fn on_projectile_hit_burst(
    hit: On<ProjectileHit>,
    mut commands: Commands,
    mut shake: ResMut<ScreenShake>,
    circle_mesh: Res<CircleMesh>,
    mut circle_mats: ResMut<Assets<CircleMaterial>>,
) {
    let Some(aoe) = hit.aoe else {
        return;
    };

    shake.intensity = AOE_SHAKE_INTENSITY;
    shake.timer = Timer::from_seconds(AOE_SHAKE_SECS, TimerMode::Once);
    shake.decay = AOE_SHAKE_DECAY;

    // Visual burst (circular shader).
    let burst_mat = circle_mats.add(CircleMaterial {
        color: Color::srgba(1.0, 0.5, 0.1, 0.4),
        softness: 0.15,
        fill_fade: 0.0,
        ripple_speed: 0.0,
        time: 0.0,
    });
    commands.spawn((
        AoEBurst {
            timer: Timer::from_seconds(AOE_BURST_SECS, TimerMode::Once),
            max_radius: aoe.radius * 2.0,
        },
        Mesh2d(circle_mesh.0.clone()),
        MeshMaterial2d(burst_mat),
        Transform::from_translation(hit.position).with_scale(Vec3::splat(AOE_BURST_SCALE)),
        DespawnOnExit(PlayPhase::Defending),
    ));
}

/// Spawn trail particles behind projectiles.
pub fn emit_trail_particles(
    mut commands: Commands,
    mut emitters: Query<(&Transform, &mut TrailEmitter)>,
    time: Res<Time>,
) {
    for (tf, mut emitter) in &mut emitters {
        emitter.timer.tick(time.delta());
        if emitter.timer.just_finished() {
            commands.spawn((
                TrailParticle {
                    timer: Timer::from_seconds(emitter.particle_lifetime, TimerMode::Once),
                },
                Sprite::from_color(
                    emitter.color.with_alpha(0.7),
                    Vec2::splat(emitter.particle_size),
                ),
                Transform::from_translation(tf.translation),
                DespawnOnExit(PlayPhase::Defending),
            ));
        }
    }
}

/// Fade and shrink trail particles, despawn when done.
pub fn fade_trail_particles(
    mut commands: Commands,
    mut particles: Query<(Entity, &mut TrailParticle, &mut Sprite, &mut Transform)>,
    time: Res<Time>,
) {
    for (entity, mut particle, mut sprite, mut tf) in &mut particles {
        particle.timer.tick(time.delta());
        let t = particle.timer.fraction();
        sprite.color = sprite.color.with_alpha(0.6 * (1.0 - t));
        tf.scale = Vec3::splat(1.0 - t * 0.7);
        if particle.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- move_toward --

    #[test]
    fn snaps_when_close() {
        let current = Vec3::new(0.0, 0.0, 0.0);
        let target = Vec3::new(3.0, 0.0, 0.0); // within HIT_DISTANCE
        let result = move_toward(current, target, 100.0, 1.0);
        assert_eq!(result, target);
    }

    #[test]
    fn moves_by_speed_times_dt() {
        let current = Vec3::new(0.0, 0.0, 0.0);
        let target = Vec3::new(100.0, 0.0, 0.0);
        let result = move_toward(current, target, 10.0, 1.0);
        assert!((result.x - 10.0).abs() < 0.01);
        assert_eq!(result.y, 0.0);
    }

    #[test]
    fn clamps_to_target() {
        let current = Vec3::new(0.0, 0.0, 0.0);
        let target = Vec3::new(20.0, 0.0, 0.0);
        // speed * dt = 200 > distance of 20
        let result = move_toward(current, target, 200.0, 1.0);
        assert_eq!(result, target);
    }

    #[test]
    fn zero_speed_stays() {
        let current = Vec3::new(0.0, 0.0, 0.0);
        let target = Vec3::new(100.0, 0.0, 0.0);
        let result = move_toward(current, target, 0.0, 1.0);
        assert_eq!(result, current);
    }

    // -- aoe_falloff --

    #[test]
    fn full_damage_at_center() {
        assert_eq!(aoe_falloff(0.0, 50.0), 1.0);
    }

    #[test]
    fn zero_damage_at_edge() {
        assert_eq!(aoe_falloff(50.0, 50.0), 0.0);
    }

    #[test]
    fn linear_midpoint() {
        assert!((aoe_falloff(25.0, 50.0) - 0.5).abs() < 0.001);
    }

    #[test]
    fn clamped_beyond_radius() {
        assert_eq!(aoe_falloff(100.0, 50.0), 0.0);
    }
}
