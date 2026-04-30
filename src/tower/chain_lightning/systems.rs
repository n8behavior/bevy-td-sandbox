//! Systems specific to the Chain Lightning tower.

use std::time::Duration;

use bevy::prelude::*;

use crate::common::constants::GridConfig;
use crate::enemy::components::{DamageFlash, Enemy, Health};
use crate::grid::systems::grid_to_world_cfg;
use crate::pile::resources::PileState;

use super::components::{ChainCooldown, ChainLightning, LightningArc};
use crate::tower::components::{
    PanelStats, StatLine, TargetingMode, Tower, TowerHealth, TowerState,
};
use crate::tower::events::TowerFired;
use crate::tower::systems::best_target_from;
use crate::tower::upgrade::{
    ARC_RANGE_MULT, COOLDOWN_MULT, DAMAGE_MULT, Primary, RANGE_MULT, UpgradeApplied, UpgradeKind,
    UpgradeTrack,
};

/// Stat color used by the upgrade panel for non-interactive stat lines.
/// Matches `tower::upgrade::STAT_COLOR`; redeclared here to keep this module
/// self-contained.
const STAT_COLOR: Color = Color::srgb(0.6, 0.6, 0.55);

/// Chain lightning towers: find target, build chain, deal damage, spawn arcs.
pub fn chain_lightning_fire(
    mut commands: Commands,
    mut towers: Query<
        (
            Entity,
            &Transform,
            &ChainLightning,
            &mut ChainCooldown,
            Option<&TargetingMode>,
            Option<&TowerHealth>,
            &TowerState,
        ),
        With<Tower>,
    >,
    mut enemies: Query<(Entity, &mut Health, &Transform, &Sprite), With<Enemy>>,
    time: Res<Time>,
    pile_state: Res<PileState>,
    config: Res<GridConfig>,
) {
    let pile_center_world = grid_to_world_cfg(pile_state.center, &config);
    for (tower_entity, tower_tf, chain, mut cooldown, targeting, tower_health, tower_state) in
        &mut towers
    {
        if !tower_state.is_operational() {
            continue;
        }
        let eff = tower_health.map_or(1.0, |h| h.effectiveness());
        cooldown
            .timer
            .tick(Duration::from_secs_f64(time.delta_secs_f64() * eff as f64));
        if !cooldown.timer.is_finished() {
            continue;
        }

        let tower_pos = tower_tf.translation.truncate();
        let mode = targeting.copied().unwrap_or_default();

        let Some(first_target) = best_target_from(
            enemies
                .iter()
                .map(|(e, h, tf, _)| (e, tf.translation.truncate(), h.current)),
            tower_pos,
            chain.primary_range.0,
            mode,
            pile_center_world,
        ) else {
            continue;
        };

        cooldown.timer.reset();
        commands.trigger(TowerFired {
            entity: tower_entity,
        });

        // Build chain (read-only pass via .iter() / .get()).
        let mut chain_targets: Vec<(Entity, Vec2, f32)> = Vec::new();
        let mut hit_set = vec![first_target];
        let mut current_damage = chain.damage.0 * eff;

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

/// Write Chain Lightning's per-tower stat lines (ARC, FIRE RATE) into
/// `PanelStats.extra`, plus the next-tier ARC preview into
/// `PanelStats.next_tier`. Reactive: runs only when chain-specific components
/// or the primary tier change, plus once on spawn (`Added<PanelStats>`).
#[allow(clippy::type_complexity)]
pub fn rebuild_chain_panel_stats(
    mut towers: Query<
        (
            &mut PanelStats,
            &ChainLightning,
            &ChainCooldown,
            &UpgradeTrack<Primary>,
        ),
        (
            With<Tower>,
            Or<(
                Changed<ChainLightning>,
                Changed<ChainCooldown>,
                Changed<UpgradeTrack<Primary>>,
                Added<PanelStats>,
            )>,
        ),
    >,
) {
    for (mut panel, chain, cd, tier) in &mut towers {
        panel.extra.clear();
        panel.extra.push(StatLine {
            label: "ARC",
            value: format!("{:.0}", chain.arc_range),
            color: STAT_COLOR,
        });
        panel.extra.push(StatLine {
            label: "FIRE RATE",
            value: format!("{:.2}s", cd.timer.duration().as_secs_f32()),
            color: STAT_COLOR,
        });

        panel.next_tier_extra.clear();
        if tier.tier < Primary::MAX_TIER {
            let cur = tier.tier as usize;
            let next = cur + 1;
            let next_arc = chain.arc_range * ARC_RANGE_MULT[next] / ARC_RANGE_MULT[cur];
            panel.next_tier_extra.push(StatLine {
                label: "ARC",
                value: format!("{:.0}", next_arc),
                color: STAT_COLOR,
            });
        }
    }
}

/// React to `UpgradeApplied<Primary>` on Chain Lightning towers: scale
/// primary range, damage, arc range, and cooldown using ratio math against
/// the upgrade multiplier tables.
pub fn scale_chain_on_tier_change(
    mut events: MessageReader<UpgradeApplied<Primary>>,
    mut towers: Query<(&mut ChainLightning, &mut ChainCooldown), With<Tower>>,
) {
    for ev in events.read() {
        let Ok((mut chain, mut cc)) = towers.get_mut(ev.tower) else {
            continue;
        };
        let old = ev.old_tier as usize;
        let new = ev.new_tier as usize;
        chain.damage.0 *= DAMAGE_MULT[new] / DAMAGE_MULT[old];
        chain.primary_range.0 *= RANGE_MULT[new] / RANGE_MULT[old];
        chain.arc_range *= ARC_RANGE_MULT[new] / ARC_RANGE_MULT[old];
        let cur_secs = cc.timer.duration().as_secs_f32();
        let new_secs = cur_secs * COOLDOWN_MULT[new] / COOLDOWN_MULT[old];
        cc.timer.set_duration(Duration::from_secs_f32(new_secs));
    }
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
