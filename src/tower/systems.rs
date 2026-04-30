use std::time::Duration;

use bevy::prelude::*;

use crate::audio::{GameSound, PlaySound};
use crate::common::constants::*;
use crate::common::math::{angle_to_dir, rotate_toward};
use crate::economy::components::ScrapDrop;
use crate::enemy::components::{Enemy, Health, SlowEffect};
use crate::grid::systems::grid_to_world_cfg;
use crate::pile::resources::{PileScrap, PileState};
use crate::projectile::components::{AoEPayload, Projectile, TrailEmitter};
use crate::stats::resources::RunStats;

use super::components::*;
use super::events::{TowerFired, TowerWantsToFire};
use super::upgrade::{Primary, UpgradeTrack, degradation_color};

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

/// Find the best entity from an iterator of `(Entity, position, health)` tuples
/// within `range` of `tower_pos`, scored by the given targeting mode.
pub(crate) fn best_target_from(
    candidates: impl Iterator<Item = (Entity, Vec2, f32)>,
    tower_pos: Vec2,
    range: f32,
    mode: TargetingMode,
    pile_center_world: Vec2,
) -> Option<Entity> {
    let mut best: Option<(Entity, f32)> = None;
    for (entity, enemy_pos, health) in candidates {
        let dist = tower_pos.distance(enemy_pos);
        if dist <= range {
            let score = targeting_score(mode, dist, health, enemy_pos, pile_center_world);
            if best.is_none_or(|(_, s)| score < s) {
                best = Some((entity, score));
            }
        }
    }
    best.map(|(e, _)| e)
}

/// Find the best enemy within range according to the given targeting mode.
pub(crate) fn find_best_target(
    enemies: &Query<(Entity, &Transform, &Health), With<Enemy>>,
    tower_pos: Vec2,
    range: f32,
    mode: TargetingMode,
    pile_center_world: Vec2,
) -> Option<Entity> {
    best_target_from(
        enemies
            .iter()
            .map(|(e, tf, h)| (e, tf.translation.truncate(), h.current)),
        tower_pos,
        range,
        mode,
        pile_center_world,
    )
}

/// Check whether the tower's rotation is within aim tolerance of a target.
pub(crate) fn is_aimed_at(
    tower_tf: &Transform,
    target_tf: &Transform,
    tower_pos: Vec2,
    tolerance: f32,
) -> bool {
    let to_target = (target_tf.translation.truncate() - tower_pos).normalize();
    angle_to_dir(tower_tf, to_target) <= tolerance
}

/// Check whether a target entity is still alive and in range.
pub(crate) fn target_valid(
    entity: Entity,
    enemies: &Query<(Entity, &Transform, &Health), With<Enemy>>,
    tower_pos: Vec2,
    range: f32,
) -> bool {
    enemies
        .get(entity)
        .is_ok_and(|(_, tf, _)| tower_pos.distance(tf.translation.truncate()) <= range)
}

/// Decision output from the pure turret state machine.
#[derive(Debug, PartialEq)]
pub(crate) enum TurretDecision {
    /// No phase change, no fire.
    Stay,
    /// Transition to a new phase (no fire).
    Transition(TurretPhase),
    /// Fire at the current tracking target (phase stays Tracking).
    Fire,
}

/// Pure state machine: determines the next turret action from current state
/// and sensor inputs. No side effects — projectile spawning and sound remain
/// in the `FixedUpdate` system for determinism.
pub(crate) fn advance_turret(
    phase: &TurretPhase,
    best_target: Option<Entity>,
    current_target_valid: bool,
    is_aimed: bool,
    cooldown_ready: bool,
) -> TurretDecision {
    match phase {
        TurretPhase::Idle => match best_target {
            Some(target) => TurretDecision::Transition(TurretPhase::Acquiring { target }),
            None => TurretDecision::Stay,
        },
        TurretPhase::Acquiring { target } => {
            if !current_target_valid {
                TurretDecision::Transition(match best_target {
                    Some(new) => TurretPhase::Acquiring { target: new },
                    None => TurretPhase::Idle,
                })
            } else if is_aimed {
                TurretDecision::Transition(TurretPhase::Tracking { target: *target })
            } else {
                TurretDecision::Stay
            }
        }
        TurretPhase::Tracking { target } => {
            if !current_target_valid {
                TurretDecision::Transition(match best_target {
                    Some(new) => TurretPhase::Acquiring { target: new },
                    None => TurretPhase::Idle,
                })
            } else if !is_aimed {
                TurretDecision::Transition(TurretPhase::Acquiring { target: *target })
            } else if cooldown_ready {
                TurretDecision::Fire
            } else {
                TurretDecision::Stay
            }
        }
    }
}

/// Compute the slow speed-multiplier for an enemy at `dist` from a tower with
/// `base_factor` slow, scaled by tower `effectiveness`.
/// At dist=0 the enemy gets full slow; at dist=range the slow fades to zero.
pub(crate) fn slow_aura_factor(base_factor: f32, dist: f32, range: f32, effectiveness: f32) -> f32 {
    let t = (dist / range).clamp(0.0, 1.0);
    let base = base_factor + (1.0 - base_factor) * t;
    1.0 - (1.0 - base) * effectiveness
}

/// Compute the pull displacement for a magnet field this frame.
/// Strength falls off linearly from center (full) to edge (zero).
pub(crate) fn magnetic_pull(
    enemy_pos: Vec2,
    magnet_pos: Vec2,
    range: f32,
    pull_speed: f32,
    dt: f32,
) -> Vec2 {
    let direction = (magnet_pos - enemy_pos).normalize();
    let dist = magnet_pos.distance(enemy_pos);
    let strength = 1.0 - (dist / range);
    direction * pull_speed * strength * dt
}

pub(crate) fn spawn_projectile(
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
///
/// On a `Fire` decision the system triggers `TowerWantsToFire` rather than
/// spawning a projectile directly. The default `default_fire_observer`
/// handles the spawn for stock turrets; per-tower observers (with the
/// `CustomFire` marker) can override the firing behavior.
pub fn turret_state_machine(
    mut commands: Commands,
    mut towers: Query<
        (
            Entity,
            &Transform,
            &mut Turret,
            Option<&TargetingMode>,
            Option<&TowerHealth>,
            &TowerState,
        ),
        With<Tower>,
    >,
    enemies: Query<(Entity, &Transform, &Health), With<Enemy>>,
    time: Res<Time>,
    pile_state: Res<PileState>,
    config: Res<GridConfig>,
) {
    let pile_center_world = grid_to_world_cfg(pile_state.center, &config);
    for (entity, tower_tf, mut turret, targeting, tower_health, tower_state) in &mut towers {
        if !tower_state.is_operational() {
            continue;
        }
        let eff = tower_health.map_or(1.0, |h| h.effectiveness());
        let range = turret.range.0;
        let aim_tolerance = turret.aim_tolerance;
        let damage = turret.damage.0;
        let tower_pos = tower_tf.translation.truncate();
        let mode = targeting.copied().unwrap_or_default();

        // Cooldown ticks scaled by effectiveness (slower fire when damaged).
        turret
            .cooldown
            .0
            .tick(Duration::from_secs_f64(time.delta_secs_f64() * eff as f64));

        let best = find_best_target(&enemies, tower_pos, range, mode, pile_center_world);

        let current_target = match turret.phase {
            TurretPhase::Acquiring { target } | TurretPhase::Tracking { target } => Some(target),
            TurretPhase::Idle => None,
        };
        let current_valid =
            current_target.is_some_and(|e| target_valid(e, &enemies, tower_pos, range));
        let aimed = current_target
            .and_then(|e| enemies.get(e).ok())
            .is_some_and(|(_, target_tf, _)| {
                is_aimed_at(tower_tf, target_tf, tower_pos, aim_tolerance)
            });

        match advance_turret(
            &turret.phase,
            best,
            current_valid,
            aimed,
            turret.cooldown.0.is_finished(),
        ) {
            TurretDecision::Stay => {}
            TurretDecision::Transition(new_phase) => {
                turret.phase = new_phase;
            }
            TurretDecision::Fire => {
                let target = match turret.phase {
                    TurretPhase::Acquiring { target } | TurretPhase::Tracking { target } => target,
                    TurretPhase::Idle => unreachable!("Fire decision implies a locked target"),
                };
                commands.trigger(TowerWantsToFire {
                    entity,
                    target,
                    damage: damage * eff,
                });
                turret.cooldown.0.reset();
                commands.trigger(TowerFired { entity });
            }
        }
    }
}

/// Default observer for `TowerWantsToFire`: spawns a standard projectile
/// from the tower's `ProjectileVisuals`, copying the optional `AoEOnHit`
/// payload onto the projectile. Towers with the `CustomFire` marker are
/// skipped; their own per-tower observers handle the spawn.
pub fn default_fire_observer(
    trigger: On<TowerWantsToFire>,
    mut commands: Commands,
    towers: Query<(&Transform, &ProjectileVisuals, Option<&AoEOnHit>), Without<CustomFire>>,
) {
    let Ok((tower_tf, visuals, aoe)) = towers.get(trigger.entity) else {
        return;
    };
    spawn_projectile(
        &mut commands,
        visuals,
        tower_tf,
        trigger.damage,
        trigger.target,
        aoe,
    );
}

// ---------------------------------------------------------------------------
// Aura (for SlowOnHit towers — no turret needed)
// ---------------------------------------------------------------------------

/// Continuously slow enemies within range of any tower with SlowOnHit.
pub fn slow_aura(
    mut commands: Commands,
    aura_towers: Query<(&Transform, &SlowOnHit, Option<&TowerHealth>, &TowerState), With<Tower>>,
    enemies: Query<(Entity, &Transform), With<Enemy>>,
) {
    for (tower_tf, slow, tower_health, tower_state) in aura_towers.iter() {
        if !tower_state.is_operational() {
            continue;
        }
        let eff = tower_health.map_or(1.0, |h| h.effectiveness());
        let range = slow.range.0;
        let tower_pos = tower_tf.translation.truncate();

        for (enemy_entity, enemy_tf) in &enemies {
            let dist = tower_pos.distance(enemy_tf.translation.truncate());
            if dist <= range {
                let factor = slow_aura_factor(slow.factor, dist, range, eff);
                commands.entity(enemy_entity).insert(SlowEffect {
                    factor,
                    remaining: Timer::from_seconds(slow.duration, TimerMode::Once),
                });
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rotation (reads target from Turret.phase)
// ---------------------------------------------------------------------------

/// Radians per second the tower turret rotates.
const TOWER_ROTATION_SPEED: f32 = 3.0;

/// Smoothly rotate towers toward their current target (shortest arc).
pub fn rotate_towers_to_target(
    mut towers: Query<(&mut Transform, &Turret, &TowerState), With<Tower>>,
    targets: Query<&Transform, Without<Tower>>,
    time: Res<Time>,
) {
    for (mut tower_tf, turret, tower_state) in &mut towers {
        if !tower_state.is_placed() {
            continue;
        }
        let current_target = match turret.phase {
            TurretPhase::Acquiring { target } | TurretPhase::Tracking { target } => Some(target),
            TurretPhase::Idle => None,
        };
        let target_dir = current_target
            .and_then(|e| targets.get(e).ok())
            .map(|target_tf| {
                (target_tf.translation.truncate() - tower_tf.translation.truncate()).normalize()
            });

        let dir = target_dir.unwrap_or(Vec2::X);
        rotate_toward(&mut tower_tf, dir, TOWER_ROTATION_SPEED, time.delta_secs());
    }
}

// ---------------------------------------------------------------------------
// Scrap Magnet systems
// ---------------------------------------------------------------------------

/// Pull scrap drops toward the nearest scrap collector in range; auto-collect on contact.
/// Matches the pile, magnet tower, and mechanical towers with ScrapCollector.
pub fn scrap_magnet_collect(
    collectors: Query<(&Transform, &ScrapCollector, Option<&TowerState>), Without<ScrapDrop>>,
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
        for (col_tf, collector, tower_state) in &collectors {
            // Skip towers that aren't operational (placing or rubble).
            // Non-tower collectors (pile) have no TowerState and are always valid.
            if tower_state.is_some_and(|s| !s.is_operational()) {
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
                commands.trigger(PlaySound(GameSound::ScrapCollected));
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
// Tower damage / rubble systems
// ---------------------------------------------------------------------------

/// When a tower becomes rubble, set sprite to grey, despawn ring children,
/// and reset turret state.
pub fn on_tower_becomes_rubble(
    mut commands: Commands,
    mut rubble_towers: Query<
        (
            Entity,
            &Children,
            &mut Sprite,
            Option<&mut Turret>,
            &TowerState,
        ),
        Changed<TowerState>,
    >,
    range_rings: Query<Entity, With<RangeRing>>,
    aura_visuals: Query<Entity, With<AuraVisual>>,
    magnet_auras: Query<Entity, With<MagnetAura>>,
) {
    for (entity, children, mut sprite, turret, state) in &mut rubble_towers {
        if *state != TowerState::Rubble {
            continue;
        }
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

        commands.trigger(PlaySound(GameSound::TowerDestroyed));
    }
}

/// Update tower sprite color based on health degradation thresholds.
/// Skips towers with an active UpgradeFlash (the flash will restore the
/// correct color when it finishes).
pub fn update_tower_degradation_visual(
    mut towers: Query<
        (
            &TowerHealth,
            &TowerColor,
            &UpgradeTrack<Primary>,
            &mut Sprite,
            &TowerState,
        ),
        (With<Tower>, Without<UpgradeFlash>, Changed<TowerHealth>),
    >,
) {
    for (health, color, tier, mut sprite, tower_state) in &mut towers {
        if !tower_state.is_operational() {
            continue;
        }
        sprite.color = degradation_color(color.0, tier.tier, health);
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
        let d1 = targeting_score(TargetingMode::Closest, 10.0, 100.0, Vec2::ZERO, Vec2::ZERO);
        let d2 = targeting_score(TargetingMode::Closest, 50.0, 100.0, Vec2::ZERO, Vec2::ZERO);
        assert!(d1 < d2);
    }

    // -- best_target_from --

    #[test]
    fn best_target_from_no_candidates() {
        let result = best_target_from(
            std::iter::empty(),
            Vec2::ZERO,
            100.0,
            TargetingMode::Closest,
            Vec2::ZERO,
        );
        assert!(result.is_none());
    }

    #[test]
    fn best_target_from_all_out_of_range() {
        let far_entity = Entity::from_raw_u32(1).unwrap();
        let candidates = vec![(far_entity, Vec2::new(200.0, 0.0), 100.0)];
        let result = best_target_from(
            candidates.into_iter(),
            Vec2::ZERO,
            50.0,
            TargetingMode::Closest,
            Vec2::ZERO,
        );
        assert!(result.is_none());
    }

    #[test]
    fn best_target_from_at_exact_range() {
        let e = Entity::from_raw_u32(1).unwrap();
        let candidates = vec![(e, Vec2::new(50.0, 0.0), 100.0)];
        let result = best_target_from(
            candidates.into_iter(),
            Vec2::ZERO,
            50.0, // dist == range
            TargetingMode::Closest,
            Vec2::ZERO,
        );
        assert_eq!(result, Some(e));
    }

    #[test]
    fn best_target_from_just_beyond_range() {
        let e = Entity::from_raw_u32(1).unwrap();
        let candidates = vec![(e, Vec2::new(50.1, 0.0), 100.0)];
        let result = best_target_from(
            candidates.into_iter(),
            Vec2::ZERO,
            50.0,
            TargetingMode::Closest,
            Vec2::ZERO,
        );
        assert!(result.is_none());
    }

    // -- advance_turret --

    #[test]
    fn idle_no_target_stays() {
        assert_eq!(
            advance_turret(&TurretPhase::Idle, None, false, false, false),
            TurretDecision::Stay,
        );
    }

    #[test]
    fn idle_with_target_acquires() {
        let e = Entity::from_raw_u32(1).unwrap();
        assert_eq!(
            advance_turret(&TurretPhase::Idle, Some(e), false, false, false),
            TurretDecision::Transition(TurretPhase::Acquiring { target: e }),
        );
    }

    #[test]
    fn acquiring_target_lost_idles() {
        let e = Entity::from_raw_u32(1).unwrap();
        assert_eq!(
            advance_turret(
                &TurretPhase::Acquiring { target: e },
                None,
                false,
                false,
                false
            ),
            TurretDecision::Transition(TurretPhase::Idle),
        );
    }

    #[test]
    fn acquiring_target_lost_reacquires() {
        let old = Entity::from_raw_u32(1).unwrap();
        let new = Entity::from_raw_u32(2).unwrap();
        assert_eq!(
            advance_turret(
                &TurretPhase::Acquiring { target: old },
                Some(new),
                false,
                false,
                false
            ),
            TurretDecision::Transition(TurretPhase::Acquiring { target: new }),
        );
    }

    #[test]
    fn acquiring_aimed_tracks() {
        let e = Entity::from_raw_u32(1).unwrap();
        assert_eq!(
            advance_turret(
                &TurretPhase::Acquiring { target: e },
                Some(e),
                true,
                true,
                false
            ),
            TurretDecision::Transition(TurretPhase::Tracking { target: e }),
        );
    }

    #[test]
    fn tracking_target_lost_idles() {
        let e = Entity::from_raw_u32(1).unwrap();
        assert_eq!(
            advance_turret(
                &TurretPhase::Tracking { target: e },
                None,
                false,
                true,
                true
            ),
            TurretDecision::Transition(TurretPhase::Idle),
        );
    }

    #[test]
    fn tracking_lost_aim_reacquires() {
        let e = Entity::from_raw_u32(1).unwrap();
        assert_eq!(
            advance_turret(
                &TurretPhase::Tracking { target: e },
                Some(e),
                true,
                false, // not aimed
                true
            ),
            TurretDecision::Transition(TurretPhase::Acquiring { target: e }),
        );
    }

    #[test]
    fn tracking_aimed_cooldown_ready_fires() {
        let e = Entity::from_raw_u32(1).unwrap();
        assert_eq!(
            advance_turret(
                &TurretPhase::Tracking { target: e },
                Some(e),
                true,
                true,
                true
            ),
            TurretDecision::Fire,
        );
    }

    #[test]
    fn tracking_aimed_cooldown_not_ready_stays() {
        let e = Entity::from_raw_u32(1).unwrap();
        assert_eq!(
            advance_turret(
                &TurretPhase::Tracking { target: e },
                Some(e),
                true,
                true,
                false // cooldown not ready
            ),
            TurretDecision::Stay,
        );
    }

    // -- slow_aura_factor --

    #[test]
    fn slow_at_center_full_slow() {
        // At distance 0, factor = base_factor + 0 = base_factor
        // With full effectiveness: 1.0 - (1.0 - 0.4) * 1.0 = 0.4
        let f = slow_aura_factor(0.4, 0.0, 100.0, 1.0);
        assert!((f - 0.4).abs() < 0.001);
    }

    #[test]
    fn slow_at_edge_no_slow() {
        // At distance = range, factor = base_factor + (1 - base_factor) * 1 = 1.0
        // Result: 1.0 - (1.0 - 1.0) * 1.0 = 1.0
        let f = slow_aura_factor(0.4, 100.0, 100.0, 1.0);
        assert!((f - 1.0).abs() < 0.001);
    }

    #[test]
    fn slow_half_effectiveness_scales() {
        // At center: base = 0.4, eff = 0.5
        // 1.0 - (1.0 - 0.4) * 0.5 = 1.0 - 0.3 = 0.7
        let f = slow_aura_factor(0.4, 0.0, 100.0, 0.5);
        assert!((f - 0.7).abs() < 0.001);
    }

    // -- magnetic_pull --

    #[test]
    fn magnetic_pull_direction_toward_magnet() {
        let pull = magnetic_pull(
            Vec2::new(0.0, 0.0),   // enemy
            Vec2::new(100.0, 0.0), // magnet
            200.0,
            60.0,
            1.0,
        );
        assert!(pull.x > 0.0, "should pull rightward toward magnet");
        assert!(pull.y.abs() < 0.001, "should not pull vertically");
    }

    #[test]
    fn magnetic_pull_strength_at_edge_near_zero() {
        let pull = magnetic_pull(
            Vec2::new(0.0, 0.0),
            Vec2::new(100.0, 0.0),
            100.0, // range == dist
            60.0,
            1.0,
        );
        assert!(pull.length() < 0.01, "pull should be near zero at edge");
    }
}
