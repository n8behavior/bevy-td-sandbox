//! Shared enemy systems: movement, slow pipeline, animations, death
//! detection, default lifecycle observers, and capability systems.
//!
//! Per-enemy bespoke behavior lives in each enemy's submodule
//! (`enemy::brute::systems`, `enemy::boss::systems`, etc.). This file
//! holds only the generic, capability-driven systems that can run for
//! any enemy that opts into the relevant capability marker.

use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::{Rng, RngExt};

use bevy::sprite_render::MeshMaterial2d;

use crate::audio::{GameSound, PlaySound};
use crate::common::constants::{GridConfig, SCRAP_COLOR, TILE_SIZE};
use crate::common::math::rotate_toward;
use crate::grid::systems::{grid_to_world_cfg, world_to_grid};
use crate::particles::systems::spawn_death_particles;
use crate::pile::resources::{EdgeCells, PileScrap, PileState};
use crate::pile::systems::{nearest_edge_cell, nearest_pile_cell};
use crate::shader::CircleMaterial;
use crate::tower::components::{
    BlocksNav, Tower, TowerColor, TowerHealth, TowerState, UpgradeFlash,
};
use crate::tower::upgrade::{Primary, UpgradeTrack, degradation_color};

use super::components::*;
use super::events::{EnemyDied, EnemyEscaped, EnemySpawned};
use super::spawn::spawn_from_blueprint;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Radians per second enemies rotate toward their travel direction.
const ENEMY_ROTATION_SPEED: f32 = 6.0;

/// Half-width of per-cell random jitter (world units per axis).
const CELL_JITTER_RANGE: f32 = 7.0;

/// Duration of the shrink+fade death animation (seconds).
const DEATH_ANIM_SECS: f32 = 0.3;

/// Per-wave health multiplier growth: `1.0 + (wave - 1) * RATE`.
const HEALTH_SCALING_PER_WAVE: f32 = 0.15;

/// Per-wave speed multiplier growth: `1.0 + (wave - 1) * RATE`.
const SPEED_SCALING_PER_WAVE: f32 = 0.05;

// ---------------------------------------------------------------------------
// Pure helpers
// ---------------------------------------------------------------------------

/// Random offset within a grid cell so enemies don't overlap on the same
/// pixel path. Each axis is sampled uniformly from
/// `[-CELL_JITTER_RANGE, CELL_JITTER_RANGE]`.
pub fn random_cell_jitter(rng: &mut impl Rng) -> Vec2 {
    Vec2::new(
        rng.random_range(-CELL_JITTER_RANGE..CELL_JITTER_RANGE),
        rng.random_range(-CELL_JITTER_RANGE..CELL_JITTER_RANGE),
    )
}

/// How much scrap an enemy steals: the lesser of its loot capacity and
/// the pile's current amount.
pub fn compute_steal_amount(loot_value: u32, pile_amount: u32) -> u32 {
    loot_value.min(pile_amount)
}

/// Whether a world-space position is within one tile of the map boundary.
pub fn is_at_map_edge(pos: Vec2, grid_width: u32, grid_height: u32) -> bool {
    let half_w = grid_width as f32 * TILE_SIZE / 2.0;
    let half_h = grid_height as f32 * TILE_SIZE / 2.0;
    let margin = TILE_SIZE;
    pos.x <= -half_w + margin
        || pos.x >= half_w - margin
        || pos.y <= -half_h + margin
        || pos.y >= half_h - margin
}

/// Wave-based health multiplier. Each capability owns its own scaling
/// formula; this one belongs with `Health` and is consumed by
/// `scale_health_on_spawn`.
pub fn health_mult_for_wave(wave: u32) -> f32 {
    1.0 + (wave.saturating_sub(1) as f32) * HEALTH_SCALING_PER_WAVE
}

/// Wave-based move-speed multiplier.
pub fn speed_mult_for_wave(wave: u32) -> f32 {
    1.0 + (wave.saturating_sub(1) as f32) * SPEED_SCALING_PER_WAVE
}

// ---------------------------------------------------------------------------
// Movement
// ---------------------------------------------------------------------------

pub fn enemy_movement(
    mut query: Query<
        (
            Entity,
            &mut AgentPos,
            &NextPos,
            &mut Transform,
            &MoveSpeed,
            Option<&mut CellJitter>,
        ),
        With<Enemy>,
    >,
    mut commands: Commands,
    time: Res<Time>,
    config: Res<GridConfig>,
    grid_query: Query<&OrdinalGrid>,
) {
    let grid = grid_query.single().ok();
    let mut rng = rand::rng();
    for (entity, mut agent_pos, next_pos, mut transform, speed, jitter) in &mut query {
        let jitter_vec = jitter.as_deref().map_or(Vec2::ZERO, |j| j.0);
        let target_world = (grid_to_world_cfg(next_pos.0, &config) + jitter_vec).extend(1.0);
        let direction = target_world - transform.translation;
        let distance = direction.length();

        if distance > 1.0 {
            let to_target = direction.truncate().normalize();
            rotate_toward(
                &mut transform,
                to_target,
                ENEMY_ROTATION_SPEED,
                time.delta_secs(),
            );
        }

        let step_size = speed.current * time.delta_secs();
        let arrived = distance < 1.0 || step_size >= distance;

        if arrived {
            agent_pos.0 = next_pos.0;
            transform.translation = target_world;
            commands.entity(entity).remove::<NextPos>();
            commands
                .entity(entity)
                .insert(CellJitter(random_cell_jitter(&mut rng)));
        } else {
            transform.translation += direction / distance * step_size;
            if let Some(grid_pos) = world_to_grid(transform.translation.truncate(), &config) {
                let candidate = UVec3::new(grid_pos.x as u32, grid_pos.y as u32, 0);
                if let Some(g) = grid
                    && !matches!(g.nav(candidate), Some(Nav::Impassable))
                {
                    agent_pos.0 = candidate;
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Scrap-stealer lifecycle (gated by StealsScrap)
// ---------------------------------------------------------------------------

/// Approaching enemies that reach a pile cell steal scrap and start fleeing.
/// If the pile is empty, enemies flee to the edge with nothing.
///
/// Filtered by `With<StealsScrap>` — only enemies that opt into this
/// goal run the steal/flee loop.
pub fn enemy_reached_pile(
    mut commands: Commands,
    mut enemies: Query<
        (
            Entity,
            &AgentPos,
            &LootValue,
            &mut EnemyState,
            Option<&NextPos>,
            Option<&Pathfind>,
            Option<&Path>,
        ),
        (With<Enemy>, With<StealsScrap>),
    >,
    mut pile_scrap: ResMut<PileScrap>,
    edge_cells: Res<EdgeCells>,
) {
    for (entity, agent_pos, loot, mut state, next_pos, pathfind_req, path) in &mut enemies {
        if *state != EnemyState::Approaching {
            continue;
        }
        let path_active = next_pos.is_some()
            || pathfind_req.is_some()
            || path.is_some_and(|p| !p.path().is_empty());
        if path_active {
            continue;
        }

        let steal_amount = compute_steal_amount(loot.0, pile_scrap.amount);
        pile_scrap.amount = pile_scrap.amount.saturating_sub(steal_amount);

        let flee_target = nearest_edge_cell(agent_pos.0, &edge_cells.0);
        *state = EnemyState::Fleeing;
        commands
            .entity(entity)
            .insert(Pathfind::new(flee_target).mode(PathfindMode::AStar));

        if steal_amount > 0 {
            commands.entity(entity).insert(StolenScrap(steal_amount));
            commands.entity(entity).with_child((
                ScrapCarrierDecal,
                Sprite::from_color(SCRAP_COLOR, Vec2::splat(6.0)),
                Transform::from_translation(Vec3::new(0.0, -5.0, 0.1)),
            ));
        }
    }
}

/// Fleeing enemies that reach the map edge escape with stolen scrap.
/// Filtered by `With<StealsScrap>`.
pub fn enemy_escaped(
    mut commands: Commands,
    enemies: Query<(Entity, &EnemyState, &Transform), (With<Enemy>, With<StealsScrap>)>,
    config: Res<GridConfig>,
) {
    for (entity, state, transform) in &enemies {
        if *state != EnemyState::Fleeing {
            continue;
        }
        let pos = transform.translation.truncate();
        if !is_at_map_edge(pos, config.width, config.height) {
            continue;
        }
        commands.trigger(EnemyEscaped { entity });
        commands.entity(entity).remove::<Enemy>();
        commands.entity(entity).insert(DeathAnimation {
            timer: Timer::from_seconds(DEATH_ANIM_SECS, TimerMode::Once),
        });
    }
}

// ---------------------------------------------------------------------------
// Death detection
// ---------------------------------------------------------------------------

/// Mark enemies with zero health as dying. Triggers `EnemyDied` (an
/// `EntityEvent`) — observers query the dying entity for whatever they
/// need (loot, position, sprite color, capability components).
pub fn check_enemy_death(mut commands: Commands, enemies: Query<(Entity, &Health), With<Enemy>>) {
    for (entity, health) in &enemies {
        if health.current <= 0.0 {
            commands.trigger(EnemyDied { entity });
            commands.entity(entity).remove::<Enemy>();
            commands.entity(entity).insert(DeathAnimation {
                timer: Timer::from_seconds(DEATH_ANIM_SECS, TimerMode::Once),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Default EnemyDied observers (loot, particles, sound)
// ---------------------------------------------------------------------------

/// Play the global enemy-death sound. Fires for every enemy.
pub fn on_enemy_died_sound(_trigger: On<EnemyDied>, mut commands: Commands) {
    commands.trigger(PlaySound(GameSound::EnemyDeath));
}

/// Spawn the death-particle burst at the dying enemy's position, tinted
/// with its current sprite color. No-ops for enemies that lack a Sprite
/// or Transform.
pub fn on_enemy_died_particles(
    trigger: On<EnemyDied>,
    enemies: Query<(&Transform, &Sprite)>,
    mut commands: Commands,
) {
    if let Ok((tf, sprite)) = enemies.get(trigger.entity) {
        spawn_death_particles(
            &mut commands,
            tf.translation.truncate(),
            sprite.color,
            &mut rand::rng(),
        );
    }
}

// ---------------------------------------------------------------------------
// Capability lifecycle observers
// ---------------------------------------------------------------------------

/// On death of an enemy carrying `SplitsOnDeath`, spawn `count` enemies
/// of the configured blueprint at the death position. Replaces the
/// previously hardcoded `on_boss_split` (which always spawned Shamblers).
pub fn on_splits_on_death(
    trigger: On<EnemyDied>,
    enemies: Query<(&Transform, &SplitsOnDeath)>,
    mut commands: Commands,
    config: Res<GridConfig>,
    grid_query: Query<Entity, With<OrdinalGrid>>,
    pile_state: Res<PileState>,
    registry: Res<EnemyRegistry>,
) {
    let Ok((tf, splits)) = enemies.get(trigger.entity) else {
        return;
    };
    let Ok(grid_entity) = grid_query.single() else {
        return;
    };
    let Some(grid_pos) = world_to_grid(tf.translation.truncate(), &config) else {
        return;
    };
    let Some(blueprint) = registry.lookup(splits.spawn_blueprint) else {
        warn!(
            "SplitsOnDeath references unknown blueprint '{}'",
            splits.spawn_blueprint
        );
        return;
    };

    let spawn_pos = UVec3::new(grid_pos.x as u32, grid_pos.y as u32, 0);
    let goal_pos = nearest_pile_cell(spawn_pos, &pile_state);

    for _ in 0..splits.count {
        spawn_from_blueprint(
            &mut commands,
            blueprint,
            spawn_pos,
            goal_pos,
            grid_entity,
            &config,
            // Splits inherit no wave scaling (they're "free" extras
            // already balanced against the parent's stats).
            1,
        );
    }
}

// ---------------------------------------------------------------------------
// Wave-difficulty scaling observers (per capability)
// ---------------------------------------------------------------------------

/// Scale Health on spawn using `health_mult_for_wave`. No-ops for enemies
/// without `Health` (e.g. a hypothetical Ghost).
pub fn scale_health_on_spawn(trigger: On<EnemySpawned>, mut q: Query<&mut Health>) {
    if let Ok(mut h) = q.get_mut(trigger.entity) {
        let mult = health_mult_for_wave(trigger.wave);
        h.max *= mult;
        h.current = h.max;
    }
}

/// Scale MoveSpeed on spawn using `speed_mult_for_wave`. No-ops for
/// stationary enemies without `MoveSpeed`.
pub fn scale_speed_on_spawn(trigger: On<EnemySpawned>, mut q: Query<&mut MoveSpeed>) {
    if let Ok(mut s) = q.get_mut(trigger.entity) {
        let mult = speed_mult_for_wave(trigger.wave);
        s.base *= mult;
        s.current = s.base;
    }
}

// ---------------------------------------------------------------------------
// Speed / slow pipeline
// ---------------------------------------------------------------------------

/// Reset all enemy speeds to base each tick, before slow effects re-apply.
pub fn reset_speed(mut query: Query<&mut MoveSpeed, With<Enemy>>) {
    for mut speed in &mut query {
        speed.current = speed.base;
    }
}

pub fn apply_slow_effects(
    mut commands: Commands,
    mut query: Query<(Entity, &mut MoveSpeed, &mut SlowEffect), With<Enemy>>,
    time: Res<Time>,
) {
    for (entity, mut speed, mut slow) in &mut query {
        slow.remaining.tick(time.delta());
        speed.current = speed.base * slow.factor;
        if slow.remaining.is_finished() {
            speed.current = speed.base;
            commands.entity(entity).remove::<SlowEffect>();
        }
    }
}

// ---------------------------------------------------------------------------
// Visuals
// ---------------------------------------------------------------------------

pub fn update_health_bars(
    enemies: Query<(&Health, &Transform, &Children), With<Enemy>>,
    mut bars: Query<(&HealthBar, &mut Sprite, &mut Transform), Without<Enemy>>,
) {
    for (health, enemy_tf, children) in &enemies {
        for child in children.iter() {
            if let Ok((bar, mut sprite, mut bar_tf)) = bars.get_mut(child) {
                let frac = (health.current / health.max).clamp(0.0, 1.0);
                let bar_width = 16.0;
                sprite.custom_size = Some(Vec2::new(bar_width * frac, 2.0));
                sprite.color = if frac > 0.5 {
                    Color::srgb(0.2, 0.8, 0.2)
                } else if frac > 0.25 {
                    Color::srgb(0.9, 0.8, 0.1)
                } else {
                    Color::srgb(0.9, 0.2, 0.1)
                };
                let inv = enemy_tf.rotation.inverse();
                let desired_offset = Vec3::new(-bar_width * (1.0 - frac) / 2.0, bar.y_offset, 0.1);
                bar_tf.translation = inv * desired_offset;
                bar_tf.rotation = inv;
            }
        }
    }
}

/// Scale-up ease-out animation on spawn.
pub fn animate_spawn(
    mut commands: Commands,
    mut query: Query<(Entity, &mut SpawnAnimation, &mut Transform)>,
    time: Res<Time>,
) {
    for (entity, mut anim, mut transform) in &mut query {
        anim.timer.tick(time.delta());
        let t = anim.timer.fraction();
        let scale = 1.0 - (1.0 - t).powi(3);
        transform.scale = Vec3::splat(scale);
        if anim.timer.is_finished() {
            transform.scale = Vec3::ONE;
            commands.entity(entity).remove::<SpawnAnimation>();
        }
    }
}

/// Shrink + fade death animation. Despawns the entity when complete.
pub fn animate_death(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DeathAnimation, &mut Transform, &mut Sprite)>,
    time: Res<Time>,
) {
    for (entity, mut anim, mut transform, mut sprite) in &mut query {
        anim.timer.tick(time.delta());
        let t = anim.timer.fraction();
        let scale = 1.0 - t;
        transform.scale = Vec3::splat(scale);
        sprite.color = sprite.color.with_alpha(1.0 - t);
        if anim.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

/// Flash enemies white on damage, restore color when done.
pub fn animate_damage_flash(
    mut commands: Commands,
    mut query: Query<(Entity, &mut DamageFlash, &mut Sprite)>,
    time: Res<Time>,
) {
    for (entity, mut flash, mut sprite) in &mut query {
        flash.timer.tick(time.delta());
        if flash.timer.is_finished() {
            sprite.color = flash.original_color;
            commands.entity(entity).remove::<DamageFlash>();
        } else {
            sprite.color = Color::WHITE;
        }
    }
}

/// Expand and fade AoE burst visuals (shader-driven circles).
pub fn animate_aoe_burst(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut AoEBurst,
        &MeshMaterial2d<CircleMaterial>,
        &mut Transform,
    )>,
    mut materials: ResMut<Assets<CircleMaterial>>,
    time: Res<Time>,
) {
    for (entity, mut burst, mat_handle, mut tf) in &mut query {
        burst.timer.tick(time.delta());
        let t = burst.timer.fraction();
        let size = burst.max_radius * t;
        tf.scale = Vec3::splat(size);
        if let Some(mat) = materials.get_mut(mat_handle.id()) {
            mat.color = mat.color.with_alpha(0.4 * (1.0 - t));
        }
        if burst.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}

// ---------------------------------------------------------------------------
// Universal capabilities
// ---------------------------------------------------------------------------

/// Heal any enemy carrying `Regeneration` over time.
pub fn regeneration_system(
    mut query: Query<(&Regeneration, &mut Health), With<Enemy>>,
    time: Res<Time>,
) {
    for (regen, mut health) in &mut query {
        health.current = (health.current + regen.rate * time.delta_secs()).min(health.max);
    }
}

/// Generalized tower-attack system. Any enemy carrying `AttacksTowers`
/// damages the nearest operational tower within its range, on its own
/// cooldown. Replaces the per-Brute `brute_attack_towers` system.
pub fn attacks_towers_system(
    mut attackers: Query<(&Transform, &mut AttacksTowers), (With<Enemy>, Without<Tower>)>,
    mut towers: Query<
        (
            Entity,
            &Transform,
            &mut TowerHealth,
            &TowerColor,
            &UpgradeTrack<Primary>,
            &mut Sprite,
            &mut TowerState,
        ),
        (With<Tower>, With<BlocksNav>, Without<Enemy>),
    >,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (attacker_tf, mut attack) in &mut attackers {
        attack.cooldown.tick(time.delta());
        if !attack.cooldown.is_finished() {
            continue;
        }

        let attacker_pos = attacker_tf.translation.truncate();

        let mut best: Option<(Entity, f32)> = None;
        for (entity, tower_tf, _, _, _, _, tower_state) in &towers {
            if !tower_state.is_operational() {
                continue;
            }
            let dist = attacker_pos.distance(tower_tf.translation.truncate());
            if dist <= attack.range && best.is_none_or(|(_, d)| dist < d) {
                best = Some((entity, dist));
            }
        }

        let Some((target_entity, _)) = best else {
            continue;
        };

        attack.cooldown.reset();

        if let Ok((entity, _, mut health, tower_color, tier, mut sprite, mut tower_state)) =
            towers.get_mut(target_entity)
        {
            health.current = (health.current - attack.damage).max(0.0);

            commands.trigger(PlaySound(GameSound::EnemyBruteAttack));

            sprite.color = Color::WHITE;
            commands.entity(entity).insert(UpgradeFlash {
                timer: Timer::from_seconds(0.1, TimerMode::Once),
                target_color: degradation_color(tower_color.0, tier.tier, &health),
            });

            if health.current <= 0.0 {
                *tower_state = TowerState::Rubble;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand::rngs::SmallRng;

    // -- random_cell_jitter --

    #[test]
    fn cell_jitter_within_bounds() {
        let mut rng = SmallRng::seed_from_u64(42);
        for _ in 0..100 {
            let j = random_cell_jitter(&mut rng);
            assert!(
                (-CELL_JITTER_RANGE..=CELL_JITTER_RANGE).contains(&j.x),
                "x out of range: {}",
                j.x
            );
            assert!(
                (-CELL_JITTER_RANGE..=CELL_JITTER_RANGE).contains(&j.y),
                "y out of range: {}",
                j.y
            );
        }
    }

    #[test]
    fn cell_jitter_deterministic() {
        let mut rng1 = SmallRng::seed_from_u64(123);
        let mut rng2 = SmallRng::seed_from_u64(123);
        assert_eq!(random_cell_jitter(&mut rng1), random_cell_jitter(&mut rng2));
    }

    // -- compute_steal_amount --

    #[test]
    fn steal_amount_clamped_to_pile() {
        assert_eq!(compute_steal_amount(10, 3), 3);
    }

    #[test]
    fn steal_amount_clamped_to_loot() {
        assert_eq!(compute_steal_amount(10, 100), 10);
    }

    #[test]
    fn steal_amount_zero_pile() {
        assert_eq!(compute_steal_amount(10, 0), 0);
    }

    #[test]
    fn steal_amount_zero_loot() {
        assert_eq!(compute_steal_amount(0, 100), 0);
    }

    // -- is_at_map_edge --

    #[test]
    fn edge_center_is_not_edge() {
        assert!(!is_at_map_edge(Vec2::ZERO, 40, 32));
    }

    #[test]
    fn edge_left_boundary() {
        assert!(is_at_map_edge(Vec2::new(-380.0, 0.0), 40, 32));
        assert!(!is_at_map_edge(Vec2::new(-379.0, 0.0), 40, 32));
    }

    #[test]
    fn edge_right_boundary() {
        assert!(is_at_map_edge(Vec2::new(380.0, 0.0), 40, 32));
        assert!(!is_at_map_edge(Vec2::new(379.0, 0.0), 40, 32));
    }

    #[test]
    fn edge_top_boundary() {
        assert!(is_at_map_edge(Vec2::new(0.0, 300.0), 40, 32));
        assert!(!is_at_map_edge(Vec2::new(0.0, 299.0), 40, 32));
    }

    #[test]
    fn edge_bottom_boundary() {
        assert!(is_at_map_edge(Vec2::new(0.0, -300.0), 40, 32));
        assert!(!is_at_map_edge(Vec2::new(0.0, -299.0), 40, 32));
    }

    #[test]
    fn edge_corner() {
        assert!(is_at_map_edge(Vec2::new(-380.0, -300.0), 40, 32));
    }

    // -- wave-scaling formulas --

    #[test]
    fn health_mult_at_wave_one_is_baseline() {
        assert!((health_mult_for_wave(1) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn health_mult_grows_with_wave() {
        assert!(health_mult_for_wave(5) > health_mult_for_wave(2));
    }

    #[test]
    fn speed_mult_at_wave_one_is_baseline() {
        assert!((speed_mult_for_wave(1) - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn health_grows_faster_than_speed() {
        // Per-wave HP growth (15 %) is steeper than speed growth (5 %).
        let h = health_mult_for_wave(10);
        let s = speed_mult_for_wave(10);
        assert!(h > s);
    }
}
