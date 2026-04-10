use bevy::prelude::*;

use bevy::sprite_render::MeshMaterial2d;

use crate::camera::components::ScreenShake;
use crate::enemy::components::{AoEBurst, Armor, DamageFlash, Enemy, Health};
use crate::particles::systems::spawn_impact_particles;
use crate::shader::{CircleMaterial, CircleMesh};
use crate::states::PlayPhase;

use super::components::*;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Distance at which a projectile snaps to its target and registers a hit.
/// Shared by `projectile_movement` (snap) and `projectile_hit_detection`.
const HIT_DISTANCE: f32 = 5.0;

/// Duration of the white damage-flash effect on hit enemies.
const DAMAGE_FLASH_SECS: f32 = 0.1;

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

/// Apply damage to an enemy, reducing by armor. Armor can never fully
/// negate damage — minimum 1 point always gets through.
pub(crate) fn apply_damage(health: &mut Health, raw_damage: f32, armor: Option<f32>) {
    let effective = match armor {
        Some(reduction) => (raw_damage - reduction).max(1.0),
        None => raw_damage,
    };
    health.current -= effective;
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

struct PendingHit {
    proj_entity: Entity,
    target: Entity,
    damage: f32,
    hit_pos: Vec3,
    aoe: Option<(f32, f32)>,
    armor: Option<f32>,
}

pub fn projectile_hit_detection(
    mut commands: Commands,
    projectiles: Query<(Entity, &Projectile, &Transform, Option<&AoEPayload>)>,
    mut enemies: Query<(Entity, &mut Health, &Transform, &Sprite, Option<&Armor>), With<Enemy>>,
    mut shake: ResMut<ScreenShake>,
    circle_mesh: Res<CircleMesh>,
    mut circle_mats: ResMut<Assets<CircleMaterial>>,
) {
    let mut hits: Vec<PendingHit> = Vec::new();

    for (proj_entity, proj, proj_tf, aoe) in &projectiles {
        let Ok((_, _, target_tf, _, target_armor)) = enemies.get(proj.target) else {
            commands.entity(proj_entity).despawn();
            continue;
        };

        let distance = proj_tf.translation.distance(target_tf.translation);
        if distance > HIT_DISTANCE {
            continue;
        }

        hits.push(PendingHit {
            proj_entity,
            target: proj.target,
            damage: proj.damage,
            hit_pos: proj_tf.translation,
            aoe: aoe.map(|a| (a.radius, a.damage)),
            armor: target_armor.map(|a| a.reduction),
        });
    }

    for hit in hits {
        commands.entity(hit.proj_entity).despawn();

        if let Ok((_, mut health, _, sprite, _)) = enemies.get_mut(hit.target) {
            apply_damage(&mut health, hit.damage, hit.armor);
            // Flash white on damage.
            commands.entity(hit.target).insert(DamageFlash {
                timer: Timer::from_seconds(DAMAGE_FLASH_SECS, TimerMode::Once),
                original_color: sprite.color,
            });
        }

        spawn_impact_particles(
            &mut commands,
            hit.hit_pos.truncate(),
            Color::srgba(1.0, 0.95, 0.6, 0.9),
            &mut rand::rng(),
        );

        if let Some((radius, aoe_damage)) = hit.aoe {
            // Screen shake on AoE.
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
                    max_radius: radius * 2.0,
                },
                Mesh2d(circle_mesh.0.clone()),
                MeshMaterial2d(burst_mat),
                Transform::from_translation(hit.hit_pos).with_scale(Vec3::splat(AOE_BURST_SCALE)),
                DespawnOnExit(PlayPhase::Defending),
            ));

            let aoe_targets: Vec<(Entity, f32, Option<f32>)> = enemies
                .iter()
                .filter(|(e, _, _, _, _)| *e != hit.target)
                .filter_map(|(e, _, tf, _, armor)| {
                    let dist = tf.translation.distance(hit.hit_pos);
                    (dist <= radius).then_some((e, dist, armor.map(|a| a.reduction)))
                })
                .collect();

            for (aoe_entity, dist, aoe_armor) in aoe_targets {
                if let Ok((_, mut health, _, sprite, _)) = enemies.get_mut(aoe_entity) {
                    let falloff = aoe_falloff(dist, radius);
                    apply_damage(&mut health, aoe_damage * falloff, aoe_armor);
                    commands.entity(aoe_entity).insert(DamageFlash {
                        timer: Timer::from_seconds(DAMAGE_FLASH_SECS, TimerMode::Once),
                        original_color: sprite.color,
                    });
                }
            }
        }
    }
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

    // -- apply_damage --

    #[test]
    fn no_armor_full_damage() {
        let mut health = Health {
            current: 100.0,
            max: 100.0,
        };
        apply_damage(&mut health, 25.0, None);
        assert_eq!(health.current, 75.0);
    }

    #[test]
    fn armor_reduces_damage() {
        let mut health = Health {
            current: 100.0,
            max: 100.0,
        };
        apply_damage(&mut health, 25.0, Some(10.0));
        assert_eq!(health.current, 85.0); // 25 - 10 = 15 damage
    }

    #[test]
    fn armor_cannot_negate_all() {
        let mut health = Health {
            current: 100.0,
            max: 100.0,
        };
        apply_damage(&mut health, 5.0, Some(50.0));
        assert_eq!(health.current, 99.0); // min 1 damage
    }

    #[test]
    fn zero_damage_with_armor() {
        let mut health = Health {
            current: 100.0,
            max: 100.0,
        };
        apply_damage(&mut health, 0.0, Some(10.0));
        assert_eq!(health.current, 99.0); // (0 - 10).max(1) = 1
    }

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
