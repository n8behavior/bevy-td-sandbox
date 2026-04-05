use bevy::prelude::*;

use crate::common::constants::*;
use crate::economy::components::ScrapDrop;
use crate::enemy::components::{DamageFlash, Dead, Dying, Enemy, Health, SlowEffect};
use crate::pile::resources::PileScrap;
use crate::projectile::components::{AoEPayload, Projectile, TrailEmitter};
use crate::stats::resources::RunStats;

use super::components::*;
use super::types::scrap_magnet::ScrapMagnet;

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
    mut towers: Query<
        (
            &Transform,
            &TowerStats,
            &mut TurretState,
            &AimTolerance,
            &ProjectileVisuals,
            Option<&AoEOnHit>,
        ),
        (With<Tower>, Without<Placing>),
    >,
    enemies: Query<(Entity, &Transform, &Health), (With<Enemy>, Without<Dead>, Without<Dying>)>,
    time: Res<Time>,
) {
    for (tower_tf, stats, mut state, aim_tol, visuals, aoe) in &mut towers {
        let range = stats.range;
        let tower_pos = tower_tf.translation.truncate();

        // Cooldown ticks in all phases.
        state.cooldown.tick(time.delta());

        let best = find_best_target(&enemies, tower_pos, range);

        match state.phase {
            TurretPhase::Idle => {
                if let Some(target) = best {
                    state.phase = TurretPhase::Acquiring { target };
                }
            }

            TurretPhase::Acquiring { target } => {
                if !target_valid(target, &enemies, tower_pos, range) {
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
                if !target_valid(target, &enemies, tower_pos, range) {
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
                        spawn_projectile(&mut commands, visuals, tower_tf, stats, target, aoe);
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
        let range = stats.range;
        let tower_pos = tower_tf.translation.truncate();

        for (enemy_entity, enemy_tf) in &enemies {
            let dist = tower_pos.distance(enemy_tf.translation.truncate());
            if dist <= range {
                // Slow proportional to distance: full slow at center, no slow at edge
                let t = (dist / range).clamp(0.0, 1.0);
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
        let target_angle =
            turret_state
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

// ---------------------------------------------------------------------------
// Scrap Magnet systems
// ---------------------------------------------------------------------------

/// Pull scrap drops toward the nearest scrap collector in range; auto-collect on contact.
/// Matches the pile, magnet tower, and mechanical towers with ScrapCollector.
pub fn scrap_magnet_collect(
    collectors: Query<(&Transform, &ScrapCollector), (Without<Placing>, Without<ScrapDrop>)>,
    mut drops: Query<(Entity, &ScrapDrop, &mut Transform), Without<ScrapCollector>>,
    mut pile_scrap: ResMut<PileScrap>,
    mut commands: Commands,
    time: Res<Time>,
    mut stats: Option<ResMut<RunStats>>,
) {
    for (entity, drop, mut drop_tf) in &mut drops {
        let drop_pos = drop_tf.translation.truncate();

        // Find nearest collector in range.
        let mut best: Option<(Vec2, f32, f32)> = None;
        for (col_tf, collector) in &collectors {
            let col_pos = col_tf.translation.truncate();
            let dist = col_pos.distance(drop_pos);
            if dist <= collector.range && best.is_none_or(|(_, d, _)| dist < d) {
                best = Some((col_pos, dist, collector.range));
            }
        }

        if let Some((col_pos, dist, range)) = best {
            if dist < MAGNET_COLLECT_RADIUS {
                pile_scrap.amount += drop.value;
                if let Some(stats) = stats.as_mut() {
                    stats.scrap_collected += drop.value;
                }
                commands.entity(entity).despawn();
            } else {
                let direction = (col_pos - drop_pos).normalize();
                let strength = 1.0 - (dist / range);
                let pull = direction * SCRAP_PULL_SPEED * strength * time.delta_secs();
                drop_tf.translation += pull.extend(0.0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Chain Lightning
// ---------------------------------------------------------------------------

/// Find the nearest enemy within range (spatial targeting for chain lightning).
fn find_nearest_enemy(
    enemies: &Query<
        (Entity, &mut Health, &Transform, &Sprite),
        (With<Enemy>, Without<Dead>, Without<Dying>),
    >,
    pos: Vec2,
    range: f32,
) -> Option<Entity> {
    let mut best: Option<(Entity, f32)> = None;
    for (entity, _, tf, _) in enemies.iter() {
        let dist = pos.distance(tf.translation.truncate());
        if dist <= range && best.is_none_or(|(_, d)| dist < d) {
            best = Some((entity, dist));
        }
    }
    best.map(|(e, _)| e)
}

/// Chain lightning towers: find target, build chain, deal damage, spawn arcs.
pub fn chain_lightning_fire(
    mut commands: Commands,
    mut towers: Query<
        (&Transform, &TowerStats, &ChainLightning, &mut ChainCooldown),
        (With<Tower>, Without<Placing>),
    >,
    mut enemies: Query<
        (Entity, &mut Health, &Transform, &Sprite),
        (With<Enemy>, Without<Dead>, Without<Dying>),
    >,
    time: Res<Time>,
) {
    for (tower_tf, stats, chain, mut cooldown) in &mut towers {
        cooldown.timer.tick(time.delta());
        if !cooldown.timer.is_finished() {
            continue;
        }

        let tower_pos = tower_tf.translation.truncate();

        let Some(first_target) = find_nearest_enemy(&enemies, tower_pos, stats.range) else {
            continue;
        };

        cooldown.timer.reset();

        // Build chain (read-only pass via .iter() / .get()).
        let mut chain_targets: Vec<(Entity, Vec2, f32)> = Vec::new();
        let mut hit_set = vec![first_target];
        let mut current_damage = stats.damage;

        if let Ok((_, _, tf, _)) = enemies.get(first_target) {
            let pos = tf.translation.truncate();
            chain_targets.push((first_target, pos, current_damage));

            loop {
                current_damage *= chain.damage_falloff;
                if current_damage < 1.0 {
                    break;
                }

                let last_pos = chain_targets.last().unwrap().1;

                // Find nearest unhit enemy within arc range.
                let mut best: Option<(Entity, Vec2, f32)> = None;
                for (e, _, tf, _) in enemies.iter() {
                    if hit_set.contains(&e) {
                        continue;
                    }
                    let pos = tf.translation.truncate();
                    let dist = last_pos.distance(pos);
                    if dist <= chain.arc_range && best.is_none_or(|(_, _, d)| dist < d) {
                        best = Some((e, pos, dist));
                    }
                }

                if let Some((entity, pos, _)) = best {
                    chain_targets.push((entity, pos, current_damage));
                    hit_set.push(entity);
                } else {
                    break;
                }
            }
        }

        // Apply damage (mutable pass via .get_mut()).
        let arc_color = Color::srgba(0.7, 0.85, 1.0, 0.9);
        let mut prev_pos = tower_pos;

        for &(entity, pos, damage) in &chain_targets {
            if let Ok((_, mut health, _, sprite)) = enemies.get_mut(entity) {
                health.current -= damage;
                commands.entity(entity).insert(DamageFlash {
                    timer: Timer::from_seconds(0.1, TimerMode::Once),
                    original_color: sprite.color,
                });
            }

            spawn_lightning_arc(&mut commands, prev_pos, pos, arc_color);
            prev_pos = pos;
        }
    }
}

fn spawn_lightning_arc(commands: &mut Commands, from: Vec2, to: Vec2, color: Color) {
    let midpoint = (from + to) / 2.0;
    let diff = to - from;
    let distance = diff.length();
    let angle = diff.y.atan2(diff.x);

    commands.spawn((
        LightningArc {
            timer: Timer::from_seconds(0.15, TimerMode::Once),
        },
        Sprite::from_color(color, Vec2::new(1.0, 2.0)),
        Transform::from_translation(midpoint.extend(5.0))
            .with_rotation(Quat::from_rotation_z(angle))
            .with_scale(Vec3::new(distance, 1.0, 1.0)),
    ));
}

/// Fade and despawn lightning arc visuals.
pub fn animate_lightning_arcs(
    mut commands: Commands,
    mut arcs: Query<(Entity, &mut LightningArc, &mut Sprite)>,
    time: Res<Time>,
) {
    for (entity, mut arc, mut sprite) in &mut arcs {
        arc.timer.tick(time.delta());
        let t = arc.timer.fraction();
        sprite.color = sprite.color.with_alpha(0.9 * (1.0 - t));
        if arc.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

// ---------------------------------------------------------------------------
// Scrap Magnet systems
// ---------------------------------------------------------------------------

/// Pull enemies toward dedicated Magnet towers, making them struggle against the field.
/// Only the Magnet tower type (ScrapMagnet marker) pulls enemies, not all collectors.
pub fn magnetic_pull_enemies(
    magnets: Query<
        (&Transform, &ScrapCollector),
        (With<ScrapMagnet>, With<Tower>, Without<Placing>),
    >,
    mut enemies: Query<
        &mut Transform,
        (With<Enemy>, Without<Dead>, Without<Dying>, Without<Tower>),
    >,
    time: Res<Time>,
) {
    for mut enemy_tf in &mut enemies {
        let enemy_pos = enemy_tf.translation.truncate();
        for (mag_tf, collector) in &magnets {
            let mag_pos = mag_tf.translation.truncate();
            let dist = mag_pos.distance(enemy_pos);
            if dist <= collector.range && dist > 2.0 {
                let direction = (mag_pos - enemy_pos).normalize();
                let strength = 1.0 - (dist / collector.range);
                let pull = direction * ENEMY_PULL_SPEED * strength * time.delta_secs();
                enemy_tf.translation += pull.extend(0.0);
            }
        }
    }
}
