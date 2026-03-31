use bevy::prelude::*;

use crate::common::constants::*;
use crate::enemy::components::{Dead, Dying, Enemy, Health, SlowEffect};
use crate::projectile::components::{AoEPayload, Projectile, TrailEmitter};

use super::components::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Find the lowest-health enemy within range.
fn find_best_target(
    enemies: &Query<(Entity, &Transform, &Health), (With<Enemy>, Without<Dead>, Without<Dying>)>,
    tower_pos: Vec2,
    range: f32,
) -> Option<Entity> {
    let mut best: Option<(Entity, f32)> = None;
    for (entity, tf, health) in enemies.iter() {
        let dist = tower_pos.distance(tf.translation.truncate());
        if dist <= range && (best.is_none() || health.current < best.unwrap().1) {
            best = Some((entity, health.current));
        }
    }
    best.map(|(e, _)| e)
}

/// Check whether the tower's rotation is within aim tolerance of a target.
fn is_aimed_at(
    tower_tf: &Transform,
    target_tf: &Transform,
    tower_pos: Vec2,
    tolerance: f32,
) -> bool {
    let to_target = target_tf.translation.truncate() - tower_pos;
    let desired = Quat::from_rotation_z(to_target.y.atan2(to_target.x));
    tower_tf.rotation.angle_between(desired) <= tolerance
}

/// Check whether a target entity is still alive and in range.
fn target_valid(
    entity: Entity,
    enemies: &Query<(Entity, &Transform, &Health), (With<Enemy>, Without<Dead>, Without<Dying>)>,
    tower_pos: Vec2,
    range: f32,
) -> bool {
    enemies
        .get(entity)
        .is_ok_and(|(_, tf, _)| tower_pos.distance(tf.translation.truncate()) <= range)
}

fn spawn_projectile(
    commands: &mut Commands,
    visuals: &ProjectileVisuals,
    tower_tf: &Transform,
    stats: &TowerStats,
    target: Entity,
    aoe: Option<&AoEOnHit>,
) {
    let mut proj = commands.spawn((
        Projectile {
            damage: stats.damage,
            speed: visuals.speed,
            target,
        },
        Sprite::from_color(visuals.color, visuals.size),
        Transform::from_translation(tower_tf.translation + Vec3::Z * 0.5),
        TrailEmitter {
            timer: Timer::from_seconds(visuals.trail_interval, TimerMode::Repeating),
            color: visuals.trail_color,
            particle_size: visuals.particle_size,
            particle_lifetime: visuals.particle_lifetime,
        },
    ));

    if let Some(aoe) = aoe {
        proj.insert(AoEPayload {
            radius: aoe.radius,
            damage: aoe.damage,
        });
    }
}

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------

/// Turret state machine: targeting, aiming, firing.
/// Only runs on towers with TurretState + AimTolerance + ProjectileVisuals.
pub fn turret_state_machine(
    mut commands: Commands,
    mut towers: Query<(
        &Transform,
        &TowerStats,
        &mut TurretState,
        &AimTolerance,
        &ProjectileVisuals,
        Option<&AoEOnHit>,
    ), (With<Tower>, Without<Placing>)>,
    enemies: Query<(Entity, &Transform, &Health), (With<Enemy>, Without<Dead>, Without<Dying>)>,
    time: Res<Time>,
) {
    for (tower_tf, stats, mut state, aim_tol, visuals, aoe) in &mut towers {
        let range_world = stats.range * TILE_SIZE;
        let tower_pos = tower_tf.translation.truncate();

        // Cooldown ticks in all phases.
        state.cooldown.tick(time.delta());

        let best = find_best_target(&enemies, tower_pos, range_world);

        match state.phase {
            TurretPhase::Idle => {
                if let Some(target) = best {
                    state.phase = TurretPhase::Acquiring { target };
                }
            }

            TurretPhase::Acquiring { target } => {
                if !target_valid(target, &enemies, tower_pos, range_world) {
                    // Target lost — retarget or idle.
                    state.phase = match best {
                        Some(new) => TurretPhase::Acquiring { target: new },
                        None => TurretPhase::Idle,
                    };
                } else if let Ok((_, target_tf, _)) = enemies.get(target)
                    && is_aimed_at(tower_tf, target_tf, tower_pos, aim_tol.0)
                {
                    state.phase = TurretPhase::Tracking { target };
                }
            }

            TurretPhase::Tracking { target } => {
                if !target_valid(target, &enemies, tower_pos, range_world) {
                    state.phase = match best {
                        Some(new) => TurretPhase::Acquiring { target: new },
                        None => TurretPhase::Idle,
                    };
                } else if let Ok((_, target_tf, _)) = enemies.get(target) {
                    if !is_aimed_at(tower_tf, target_tf, tower_pos, aim_tol.0) {
                        // Lost aim — back to acquiring.
                        state.phase = TurretPhase::Acquiring { target };
                    } else if state.cooldown.is_finished() {
                        // FIRE!
                        spawn_projectile(
                            &mut commands,
                            visuals,
                            tower_tf,
                            stats,
                            target,
                            aoe,
                        );
                        state.cooldown.reset();
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Aura (for SlowOnHit towers — no turret needed)
// ---------------------------------------------------------------------------

/// Continuously slow enemies within range of any tower with SlowOnHit.
pub fn slow_aura(
    mut commands: Commands,
    aura_towers: Query<(&Transform, &TowerStats, &SlowOnHit), (With<Tower>, Without<Placing>)>,
    enemies: Query<(Entity, &Transform), (With<Enemy>, Without<Dead>, Without<Dying>)>,
) {
    for (tower_tf, stats, slow) in aura_towers.iter() {
        let range_world = stats.range * TILE_SIZE;
        let tower_pos = tower_tf.translation.truncate();

        for (enemy_entity, enemy_tf) in &enemies {
            let dist = tower_pos.distance(enemy_tf.translation.truncate());
            if dist <= range_world {
                // Slow proportional to distance: full slow at center, no slow at edge
                let t = (dist / range_world).clamp(0.0, 1.0);
                let factor = slow.factor + (1.0 - slow.factor) * t;
                commands.entity(enemy_entity).insert(SlowEffect {
                    factor,
                    remaining: Timer::from_seconds(slow.duration, TimerMode::Once),
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rotation (reads target from TurretState)
// ---------------------------------------------------------------------------

/// Radians per second the tower turret rotates.
const TOWER_ROTATION_SPEED: f32 = 3.0;

/// Smoothly rotate towers toward their current target (shortest arc).
pub fn rotate_towers_to_target(
    mut towers: Query<(&mut Transform, &TurretState), (With<Tower>, Without<Placing>)>,
    targets: Query<&Transform, Without<Tower>>,
    time: Res<Time>,
) {
    for (mut tower_tf, turret_state) in &mut towers {
        let target_angle = turret_state
            .target()
            .and_then(|e| targets.get(e).ok())
            .map(|target_tf| {
                let dir = target_tf.translation.truncate() - tower_tf.translation.truncate();
                dir.y.atan2(dir.x)
            });

        let goal = match target_angle {
            Some(angle) => Quat::from_rotation_z(angle),
            None => Quat::IDENTITY,
        };

        // Shortest-arc slerp.
        let dot = tower_tf.rotation.dot(goal);
        let goal = if dot < 0.0 { -goal } else { goal };
        let angle_remaining = tower_tf.rotation.angle_between(goal);
        let max_step = TOWER_ROTATION_SPEED * time.delta_secs();
        let t = if angle_remaining > 0.0 {
            (max_step / angle_remaining).min(1.0)
        } else {
            1.0
        };
        tower_tf.rotation = tower_tf.rotation.slerp(goal, t);
    }
}
