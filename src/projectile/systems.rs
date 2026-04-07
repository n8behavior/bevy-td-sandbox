use bevy::prelude::*;

use bevy::sprite_render::MeshMaterial2d;

use crate::camera::components::ScreenShake;
use crate::enemy::components::{
    AoEBurst, Armor, DamageFlash, Enemy, EnemyState, Health, SlowEffect,
};
use crate::particles::systems::spawn_impact_particles;
use crate::shader::{CircleMaterial, CircleMesh};

use super::components::*;

pub fn projectile_movement(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &Projectile, &mut Transform)>,
    targets: Query<(&Transform, &EnemyState), (With<Enemy>, Without<Projectile>)>,
    time: Res<Time>,
) {
    for (entity, proj, mut proj_tf) in &mut projectiles {
        let Ok((target_tf, target_state)) = targets.get(proj.target) else {
            // Target gone -- despawn projectile
            commands.entity(entity).despawn();
            continue;
        };
        if !target_state.is_alive() {
            commands.entity(entity).despawn();
            continue;
        }

        let direction = target_tf.translation - proj_tf.translation;
        let distance = direction.length();

        if distance < 5.0 {
            proj_tf.translation = target_tf.translation;
        } else {
            let step = direction.normalize() * proj.speed * time.delta_secs();
            if step.length() >= distance {
                proj_tf.translation = target_tf.translation;
            } else {
                proj_tf.translation += step;
            }
        }
    }
}

struct PendingHit {
    proj_entity: Entity,
    target: Entity,
    damage: f32,
    hit_pos: Vec3,
    slow: Option<(f32, f32)>,
    aoe: Option<(f32, f32)>,
    armor: Option<f32>,
}

fn apply_damage(health: &mut Health, raw_damage: f32, armor: Option<f32>) {
    let effective = match armor {
        Some(reduction) => (raw_damage - reduction).max(1.0),
        None => raw_damage,
    };
    health.current -= effective;
}

pub fn projectile_hit_detection(
    mut commands: Commands,
    projectiles: Query<(
        Entity,
        &Projectile,
        &Transform,
        Option<&AoEPayload>,
        Option<&SlowPayload>,
    )>,
    mut enemies: Query<
        (
            Entity,
            &mut Health,
            &Transform,
            &Sprite,
            Option<&Armor>,
            &EnemyState,
        ),
        With<Enemy>,
    >,
    mut shake: ResMut<ScreenShake>,
    circle_mesh: Res<CircleMesh>,
    mut circle_mats: ResMut<Assets<CircleMaterial>>,
) {
    let mut hits: Vec<PendingHit> = Vec::new();

    for (proj_entity, proj, proj_tf, aoe, slow) in &projectiles {
        let Ok((_, _, target_tf, _, target_armor, target_state)) = enemies.get(proj.target) else {
            commands.entity(proj_entity).despawn();
            continue;
        };
        if !target_state.is_alive() {
            commands.entity(proj_entity).despawn();
            continue;
        }

        let distance = proj_tf.translation.distance(target_tf.translation);
        if distance > 5.0 {
            continue;
        }

        hits.push(PendingHit {
            proj_entity,
            target: proj.target,
            damage: proj.damage,
            hit_pos: proj_tf.translation,
            slow: slow.map(|s| (s.factor, s.duration)),
            aoe: aoe.map(|a| (a.radius, a.damage)),
            armor: target_armor.map(|a| a.reduction),
        });
    }

    for hit in hits {
        commands.entity(hit.proj_entity).despawn();

        if let Ok((_, mut health, _, sprite, _, _)) = enemies.get_mut(hit.target) {
            apply_damage(&mut health, hit.damage, hit.armor);
            // Flash white on damage.
            commands.entity(hit.target).insert(DamageFlash {
                timer: Timer::from_seconds(0.1, TimerMode::Once),
                original_color: sprite.color,
            });
        }

        spawn_impact_particles(
            &mut commands,
            hit.hit_pos.truncate(),
            Color::srgba(1.0, 0.95, 0.6, 0.9),
        );

        if let Some((factor, duration)) = hit.slow {
            commands.entity(hit.target).insert(SlowEffect {
                factor,
                remaining: Timer::from_seconds(duration, TimerMode::Once),
            });
        }

        if let Some((radius, aoe_damage)) = hit.aoe {
            // Screen shake on AoE.
            shake.intensity = 3.0;
            shake.timer = Timer::from_seconds(0.25, TimerMode::Once);
            shake.decay = 0.05;

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
                    timer: Timer::from_seconds(0.3, TimerMode::Once),
                    max_radius: radius * 2.0,
                },
                Mesh2d(circle_mesh.0.clone()),
                MeshMaterial2d(burst_mat),
                Transform::from_translation(hit.hit_pos).with_scale(Vec3::splat(4.0)),
            ));

            let aoe_targets: Vec<(Entity, f32, Option<f32>)> = enemies
                .iter()
                .filter(|(e, _, _, _, _, state)| *e != hit.target && state.is_alive())
                .filter_map(|(e, _, tf, _, armor, _)| {
                    let dist = tf.translation.distance(hit.hit_pos);
                    (dist <= radius).then_some((e, dist, armor.map(|a| a.reduction)))
                })
                .collect();

            for (aoe_entity, dist, aoe_armor) in aoe_targets {
                if let Ok((_, mut health, _, sprite, _, _)) = enemies.get_mut(aoe_entity) {
                    // Full damage at center, zero at edge.
                    let falloff = 1.0 - (dist / radius).clamp(0.0, 1.0);
                    apply_damage(&mut health, aoe_damage * falloff, aoe_armor);
                    commands.entity(aoe_entity).insert(DamageFlash {
                        timer: Timer::from_seconds(0.1, TimerMode::Once),
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
