use bevy::prelude::*;

use crate::common::constants::*;
use crate::enemy::components::{Dead, Enemy, SlowEffect};
use crate::projectile::components::*;

use super::components::*;

/// Projectile-based towers (ScrapGun, Explosive, Railgun). TarPit excluded.
pub fn tower_shooting(
    mut commands: Commands,
    mut towers: Query<(
        &Tower,
        &Transform,
        &TowerStats,
        &mut AttackCooldown,
        Option<&AoEOnHit>,
    )>,
    enemies: Query<(Entity, &Transform), (With<Enemy>, Without<Dead>)>,
    time: Res<Time>,
) {
    for (tower, tower_tf, stats, mut cooldown, aoe) in &mut towers {
        // TarPit uses aura, not projectiles
        if tower.tower_type == TowerType::TarPit {
            continue;
        }

        cooldown.timer.tick(time.delta());

        if cooldown.timer.times_finished_this_tick() == 0 {
            continue;
        }

        let range_world = stats.range * TILE_SIZE;
        let tower_pos = tower_tf.translation.truncate();

        let mut best: Option<(Entity, f32)> = None;
        for (enemy_entity, enemy_tf) in &enemies {
            let dist = tower_pos.distance(enemy_tf.translation.truncate());
            if dist <= range_world && (best.is_none() || dist < best.unwrap().1) {
                best = Some((enemy_entity, dist));
            }
        }

        let Some((target_entity, _)) = best else {
            continue;
        };

        let mut proj = commands.spawn((
            Projectile {
                damage: stats.damage,
                speed: 200.0,
                target: target_entity,
            },
            Sprite::from_color(Color::srgb(1.0, 1.0, 0.6), Vec2::splat(6.0)),
            Transform::from_translation(tower_tf.translation + Vec3::Z * 0.5),
        ));

        if let Some(aoe) = aoe {
            proj.insert(AoEPayload {
                radius: aoe.radius,
                damage: aoe.damage,
            });
        }
    }
}

/// TarPit aura: continuously slow enemies within range (no projectile needed)
pub fn tarpit_aura(
    mut commands: Commands,
    tarpits: Query<(&Transform, &TowerStats, &SlowOnHit), With<Tower>>,
    enemies: Query<(Entity, &Transform), (With<Enemy>, Without<Dead>)>,
) {
    for (tower_tf, stats, slow) in &tarpits {
        let range_world = stats.range * TILE_SIZE;
        let tower_pos = tower_tf.translation.truncate();

        for (enemy_entity, enemy_tf) in &enemies {
            let dist = tower_pos.distance(enemy_tf.translation.truncate());
            if dist <= range_world {
                commands.entity(enemy_entity).insert(SlowEffect {
                    factor: slow.factor,
                    remaining: Timer::from_seconds(slow.duration, TimerMode::Once),
                });
            }
        }
    }
}
