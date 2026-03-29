use bevy::prelude::*;

use crate::common::constants::*;
use crate::enemy::components::{Enemy, Health};

use super::components::*;

pub fn tower_targeting(
    mut towers: Query<(&Transform, &TowerStats, &mut AttackCooldown, Option<&mut CurrentTarget>), With<Tower>>,
    enemies: Query<(Entity, &Transform), With<Enemy>>,
    mut commands: Commands,
) {
    for (tower_tf, stats, _cooldown, current_target) in &mut towers {
        let range_world = stats.range * TILE_SIZE;
        let tower_pos = tower_tf.translation.truncate();

        // Find closest enemy in range
        let mut best: Option<(Entity, f32)> = None;
        for (enemy_entity, enemy_tf) in &enemies {
            let dist = tower_pos.distance(enemy_tf.translation.truncate());
            if dist <= range_world {
                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((enemy_entity, dist));
                }
            }
        }

        match (best, current_target) {
            (Some((entity, _)), Some(mut target)) => {
                target.0 = entity;
            }
            (Some((entity, _)), None) => {
                // Can't insert here easily, we'll handle in shooting
                // Just track that we found a target
                let _ = entity;
            }
            (None, _) => {}
        }
    }
}

pub fn tower_shooting(
    mut commands: Commands,
    mut towers: Query<(
        Entity,
        &Transform,
        &TowerStats,
        &Tower,
        &mut AttackCooldown,
        Option<&SlowOnHit>,
        Option<&AoEOnHit>,
    )>,
    enemies: Query<(Entity, &Transform), With<Enemy>>,
    time: Res<Time>,
) {
    for (tower_entity, tower_tf, stats, tower, mut cooldown, slow, aoe) in &mut towers {
        cooldown.timer.tick(time.delta());

        if !cooldown.timer.just_finished() {
            continue;
        }

        let range_world = stats.range * TILE_SIZE;
        let tower_pos = tower_tf.translation.truncate();

        // Find closest enemy in range
        let mut best: Option<(Entity, f32)> = None;
        for (enemy_entity, enemy_tf) in &enemies {
            let dist = tower_pos.distance(enemy_tf.translation.truncate());
            if dist <= range_world {
                if best.is_none() || dist < best.unwrap().1 {
                    best = Some((enemy_entity, dist));
                }
            }
        }

        let Some((target_entity, _)) = best else {
            continue;
        };

        // Spawn projectile
        use crate::projectile::components::*;

        let mut proj = commands.spawn((
            Projectile {
                damage: stats.damage,
                speed: 300.0,
                target: target_entity,
            },
            Sprite::from_color(Color::srgb(1.0, 1.0, 0.6), Vec2::splat(4.0)),
            Transform::from_translation(tower_tf.translation + Vec3::Z * 0.5),
        ));

        if let Some(slow) = slow {
            proj.insert(SlowPayload {
                factor: slow.factor,
                duration: slow.duration,
            });
        }

        if let Some(aoe) = aoe {
            proj.insert(AoEPayload {
                radius: aoe.radius,
                damage: aoe.damage,
            });
        }
    }
}
