use bevy::prelude::*;

use crate::enemy::components::*;

use super::components::*;

pub fn projectile_movement(
    mut commands: Commands,
    mut projectiles: Query<(Entity, &Projectile, &mut Transform)>,
    targets: Query<&Transform, (With<Enemy>, Without<Dead>, Without<Projectile>)>,
    time: Res<Time>,
) {
    for (entity, proj, mut proj_tf) in &mut projectiles {
        let Ok(target_tf) = targets.get(proj.target) else {
            // Target dead or gone -- despawn projectile
            commands.entity(entity).despawn();
            continue;
        };

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
}

pub fn projectile_hit_detection(
    mut commands: Commands,
    projectiles: Query<(Entity, &Projectile, &Transform, Option<&AoEPayload>, Option<&SlowPayload>)>,
    mut enemies: Query<(Entity, &mut Health, &Transform), (With<Enemy>, Without<Dead>)>,
) {
    let mut hits: Vec<PendingHit> = Vec::new();

    for (proj_entity, proj, proj_tf, aoe, slow) in &projectiles {
        let Ok((_, _, target_tf)) = enemies.get(proj.target) else {
            commands.entity(proj_entity).despawn();
            continue;
        };

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
        });
    }

    for hit in hits {
        commands.entity(hit.proj_entity).despawn();

        if let Ok((_, mut health, _)) = enemies.get_mut(hit.target) {
            health.current -= hit.damage;
        }

        if let Some((factor, duration)) = hit.slow {
            commands.entity(hit.target).insert(SlowEffect {
                factor,
                remaining: Timer::from_seconds(duration, TimerMode::Once),
            });
        }

        if let Some((radius, aoe_damage)) = hit.aoe {
            let aoe_targets: Vec<Entity> = enemies
                .iter()
                .filter(|(e, _, _)| *e != hit.target)
                .filter(|(_, _, tf)| tf.translation.distance(hit.hit_pos) <= radius)
                .map(|(e, _, _)| e)
                .collect();

            for aoe_entity in aoe_targets {
                if let Ok((_, mut health, _)) = enemies.get_mut(aoe_entity) {
                    health.current -= aoe_damage;
                }
            }
        }
    }
}
