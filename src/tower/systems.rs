use std::time::Duration;

use bevy::prelude::*;

use crate::audio::resources::SoundAssets;
use crate::audio::systems::play_sound;
use crate::common::constants::*;
use crate::economy::components::ScrapDrop;
use crate::enemy::components::{DamageFlash, Dead, Dying, Enemy, Health, SlowEffect};
use crate::grid::systems::grid_to_world_cfg;
use crate::pile::resources::{PileScrap, PileState};
use crate::projectile::components::{AoEPayload, Projectile, TrailEmitter};
use crate::stats::resources::RunStats;

use super::components::*;
use super::types::explosive::Explosive;
use super::types::railgun::Railgun;
use super::types::scrap_gun::ScrapGun;
use super::types::scrap_magnet::ScrapMagnet;
use super::upgrade::degradation_color;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Compute a targeting score for an enemy. Lower score = higher priority.
pub(crate) fn targeting_score(
    mode: TargetingMode,
    dist_to_tower: f32,
    health_current: f32,
    enemy_pos: Vec2,
    pile_center_world: Vec2,
) -> f32 {
    match mode {
        TargetingMode::Closest => dist_to_tower,
        TargetingMode::LowestHp => health_current,
        TargetingMode::HighestHp => -health_current,
        TargetingMode::FurthestAlongPath => pile_center_world.distance(enemy_pos),
    }
}

/// Find the best enemy within range according to the given targeting mode.
pub(crate) fn find_best_target(
    enemies: &Query<(Entity, &Transform, &Health), (With<Enemy>, Without<Dead>, Without<Dying>)>,
    tower_pos: Vec2,
    range: f32,
    mode: TargetingMode,
    pile_center_world: Vec2,
) -> Option<Entity> {
    let mut best: Option<(Entity, f32)> = None;
    for (entity, tf, health) in enemies.iter() {
        let enemy_pos = tf.translation.truncate();
        let dist = tower_pos.distance(enemy_pos);
        if dist <= range {
            let score = targeting_score(mode, dist, health.current, enemy_pos, pile_center_world);
            if best.is_none_or(|(_, s)| score < s) {
                best = Some((entity, score));
            }
        }
    }
    best.map(|(e, _)| e)
}

/// Check whether the tower's rotation is within aim tolerance of a target.
pub(crate) fn is_aimed_at(
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
pub(crate) fn target_valid(
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
    damage: f32,
    target: Entity,
    aoe: Option<&AoEOnHit>,
) {
    let mut proj = commands.spawn((
        Projectile {
            damage,
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
            Option<&TargetingMode>,
            Option<&ScrapGun>,
            Option<&Explosive>,
            Option<&Railgun>,
            Option<&TowerHealth>,
        ),
        (With<Tower>, Without<Placing>, Without<TowerRubble>),
    >,
    enemies: Query<(Entity, &Transform, &Health), (With<Enemy>, Without<Dead>, Without<Dying>)>,
    time: Res<Time>,
    pile_state: Res<PileState>,
    config: Res<GridConfig>,
    sounds: Res<SoundAssets>,
) {
    let pile_center_world = grid_to_world_cfg(pile_state.center, &config);
    for (
        tower_tf,
        stats,
        mut state,
        aim_tol,
        visuals,
        aoe,
        targeting,
        is_scrapgun,
        is_explosive,
        is_railgun,
        tower_health,
    ) in &mut towers
    {
        let eff = tower_health.map_or(1.0, |h| h.effectiveness());
        let range = stats.range;
        let tower_pos = tower_tf.translation.truncate();
        let mode = targeting.copied().unwrap_or_default();

        // Cooldown ticks scaled by effectiveness (slower fire when damaged).
        state
            .cooldown
            .tick(Duration::from_secs_f64(time.delta_secs_f64() * eff as f64));

        let best = find_best_target(&enemies, tower_pos, range, mode, pile_center_world);

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
                        spawn_projectile(
                            &mut commands,
                            visuals,
                            tower_tf,
                            stats.damage * eff,
                            target,
                            aoe,
                        );
                        state.cooldown.reset();

                        if is_scrapgun.is_some() {
                            play_sound(&mut commands, &sounds.tower_scrapgun, 0.3);
                        } else if is_explosive.is_some() {
                            play_sound(&mut commands, &sounds.tower_explosive, 0.4);
                        } else if is_railgun.is_some() {
                            play_sound(&mut commands, &sounds.tower_railgun, 0.3);
                        }
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
    aura_towers: Query<
        (&Transform, &TowerStats, &SlowOnHit, Option<&TowerHealth>),
        (With<Tower>, Without<Placing>, Without<TowerRubble>),
    >,
    enemies: Query<(Entity, &Transform), (With<Enemy>, Without<Dead>, Without<Dying>)>,
) {
    for (tower_tf, stats, slow, tower_health) in aura_towers.iter() {
        let eff = tower_health.map_or(1.0, |h| h.effectiveness());
        let range = stats.range;
        let tower_pos = tower_tf.translation.truncate();

        for (enemy_entity, enemy_tf) in &enemies {
            let dist = tower_pos.distance(enemy_tf.translation.truncate());
            if dist <= range {
                // Slow proportional to distance: full slow at center, no slow at edge.
                let t = (dist / range).clamp(0.0, 1.0);
                let base_factor = slow.factor + (1.0 - slow.factor) * t;
                // Scale slow strength by effectiveness.
                let factor = 1.0 - (1.0 - base_factor) * eff;
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
    collectors: Query<
        (&Transform, &ScrapCollector, Option<&TowerRubble>),
        (Without<Placing>, Without<ScrapDrop>),
    >,
    mut drops: Query<(Entity, &ScrapDrop, &mut Transform), Without<ScrapCollector>>,
    mut pile_scrap: ResMut<PileScrap>,
    mut commands: Commands,
    time: Res<Time>,
    mut stats: Option<ResMut<RunStats>>,
    sounds: Res<SoundAssets>,
) {
    for (entity, drop, mut drop_tf) in &mut drops {
        let drop_pos = drop_tf.translation.truncate();

        // Find nearest collector in range.
        let mut best: Option<(Vec2, f32, f32)> = None;
        for (col_tf, collector, rubble) in &collectors {
            if rubble.is_some() {
                continue;
            }
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
                play_sound(&mut commands, &sounds.scrap_collected, 0.2);
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

/// Find the best initial target for chain lightning within range.
fn find_chain_target(
    enemies: &Query<
        (Entity, &mut Health, &Transform, &Sprite),
        (With<Enemy>, Without<Dead>, Without<Dying>),
    >,
    tower_pos: Vec2,
    range: f32,
    mode: TargetingMode,
    pile_center_world: Vec2,
) -> Option<Entity> {
    let mut best: Option<(Entity, f32)> = None;
    for (entity, health, tf, _) in enemies.iter() {
        let enemy_pos = tf.translation.truncate();
        let dist = tower_pos.distance(enemy_pos);
        if dist <= range {
            let score = targeting_score(mode, dist, health.current, enemy_pos, pile_center_world);
            if best.is_none_or(|(_, s)| score < s) {
                best = Some((entity, score));
            }
        }
    }
    best.map(|(e, _)| e)
}

/// Chain lightning towers: find target, build chain, deal damage, spawn arcs.
pub fn chain_lightning_fire(
    mut commands: Commands,
    mut towers: Query<
        (
            &Transform,
            &TowerStats,
            &ChainLightning,
            &mut ChainCooldown,
            Option<&TargetingMode>,
            Option<&TowerHealth>,
        ),
        (With<Tower>, Without<Placing>, Without<TowerRubble>),
    >,
    mut enemies: Query<
        (Entity, &mut Health, &Transform, &Sprite),
        (With<Enemy>, Without<Dead>, Without<Dying>),
    >,
    time: Res<Time>,
    pile_state: Res<PileState>,
    config: Res<GridConfig>,
    sounds: Res<SoundAssets>,
) {
    let pile_center_world = grid_to_world_cfg(pile_state.center, &config);
    for (tower_tf, stats, chain, mut cooldown, targeting, tower_health) in &mut towers {
        let eff = tower_health.map_or(1.0, |h| h.effectiveness());
        cooldown
            .timer
            .tick(Duration::from_secs_f64(time.delta_secs_f64() * eff as f64));
        if !cooldown.timer.is_finished() {
            continue;
        }

        let tower_pos = tower_tf.translation.truncate();
        let mode = targeting.copied().unwrap_or_default();

        let Some(first_target) =
            find_chain_target(&enemies, tower_pos, stats.range, mode, pile_center_world)
        else {
            continue;
        };

        cooldown.timer.reset();
        play_sound(&mut commands, &sounds.tower_chain_lightning, 0.3);

        // Build chain (read-only pass via .iter() / .get()).
        let mut chain_targets: Vec<(Entity, Vec2, f32)> = Vec::new();
        let mut hit_set = vec![first_target];
        let mut current_damage = stats.damage * eff;

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
        (
            With<ScrapMagnet>,
            With<Tower>,
            Without<Placing>,
            Without<TowerRubble>,
        ),
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

// ---------------------------------------------------------------------------
// Tower damage / rubble systems
// ---------------------------------------------------------------------------

/// When a tower becomes rubble, set sprite to grey, despawn ring children,
/// and reset turret state.
pub fn on_tower_becomes_rubble(
    mut commands: Commands,
    mut rubble_towers: Query<
        (Entity, &Children, &mut Sprite, Option<&mut TurretState>),
        Added<TowerRubble>,
    >,
    range_rings: Query<Entity, With<RangeRing>>,
    aura_visuals: Query<Entity, With<AuraVisual>>,
    magnet_auras: Query<Entity, With<MagnetAura>>,
    sounds: Res<SoundAssets>,
) {
    for (entity, children, mut sprite, turret) in &mut rubble_towers {
        sprite.color = RUBBLE_TOWER_COLOR;

        if let Some(mut turret) = turret {
            turret.phase = TurretPhase::Idle;
        }

        // Despawn visual ring children (keep config components for repair).
        for child in children.iter() {
            if range_rings.contains(child)
                || aura_visuals.contains(child)
                || magnet_auras.contains(child)
            {
                commands.entity(child).despawn();
            }
        }

        // Remove the UpgradeFlash so the rubble color sticks.
        commands.entity(entity).remove::<UpgradeFlash>();

        play_sound(&mut commands, &sounds.tower_destroyed, 0.5);
    }
}

/// Update tower sprite color based on health degradation thresholds.
/// Skips towers with an active UpgradeFlash (the flash will restore the
/// correct color when it finishes).
pub fn update_tower_degradation_visual(
    mut towers: Query<
        (&TowerHealth, &BaseStats, &TowerTier, &mut Sprite),
        (
            With<Tower>,
            Without<Placing>,
            Without<TowerRubble>,
            Without<UpgradeFlash>,
            Changed<TowerHealth>,
        ),
    >,
) {
    for (health, base, tier, mut sprite) in &mut towers {
        sprite.color = degradation_color(base.color, tier.0, health);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tower::components::TargetingMode;

    #[test]
    fn targeting_score_closest_returns_distance() {
        let score = targeting_score(TargetingMode::Closest, 42.0, 100.0, Vec2::ZERO, Vec2::ZERO);
        assert_eq!(score, 42.0);
    }

    #[test]
    fn targeting_score_lowest_hp_returns_health() {
        let score = targeting_score(TargetingMode::LowestHp, 10.0, 55.0, Vec2::ZERO, Vec2::ZERO);
        assert_eq!(score, 55.0);
    }

    #[test]
    fn targeting_score_highest_hp_returns_negative_health() {
        let score = targeting_score(TargetingMode::HighestHp, 10.0, 55.0, Vec2::ZERO, Vec2::ZERO);
        assert_eq!(score, -55.0);
    }

    #[test]
    fn targeting_score_furthest_along_path() {
        let pile_center = Vec2::new(100.0, 100.0);
        let close_to_pile = Vec2::new(95.0, 95.0);
        let far_from_pile = Vec2::new(10.0, 10.0);

        let score_close = targeting_score(
            TargetingMode::FurthestAlongPath,
            0.0,
            0.0,
            close_to_pile,
            pile_center,
        );
        let score_far = targeting_score(
            TargetingMode::FurthestAlongPath,
            0.0,
            0.0,
            far_from_pile,
            pile_center,
        );
        // Closer to pile = lower score = higher priority
        assert!(score_close < score_far);
    }

    #[test]
    fn targeting_closest_selects_nearest() {
        // With Closest mode, the enemy with smallest distance should have lowest score
        let d1 = targeting_score(TargetingMode::Closest, 10.0, 100.0, Vec2::ZERO, Vec2::ZERO);
        let d2 = targeting_score(TargetingMode::Closest, 50.0, 100.0, Vec2::ZERO, Vec2::ZERO);
        assert!(d1 < d2);
    }
}
