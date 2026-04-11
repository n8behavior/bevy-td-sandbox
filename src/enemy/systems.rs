use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::Rng;

use bevy::sprite_render::MeshMaterial2d;

use crate::audio::{GameSound, PlaySound};
use crate::camera::components::ScreenShake;
use crate::common::constants::{GridConfig, SCRAP_COLOR, TILE_SIZE};
use crate::common::math::rotate_toward;
use crate::grid::systems::{grid_to_world_cfg, world_to_grid};
use crate::particles::systems::spawn_death_particles;
use crate::pile::resources::{EdgeCells, PileScrap, PileState};
use crate::pile::systems::{nearest_edge_cell, nearest_pile_cell};
use crate::shader::CircleMaterial;

use crate::common::constants::{BRUTE_ATTACK_COOLDOWN, BRUTE_ATTACK_DAMAGE, BRUTE_ATTACK_RANGE};
use crate::tower::components::{
    BaseStats, BlocksNav, Tower, TowerHealth, TowerState, TowerTier, UpgradeFlash,
};
use crate::tower::upgrade::degradation_color;
use crate::wave::resources::BossTrait;

use super::components::*;

/// Radians per second enemies rotate toward their travel direction.
const ENEMY_ROTATION_SPEED: f32 = 6.0;

/// Half-width of per-cell random jitter (world units per axis).
const CELL_JITTER_RANGE: f32 = 7.0;

/// Duration of the shrink+fade death animation (seconds).
const DEATH_ANIM_SECS: f32 = 0.3;

/// Random offset within a grid cell so enemies don't overlap on the same pixel path.
/// Each axis is sampled uniformly from `[-CELL_JITTER_RANGE, CELL_JITTER_RANGE]`.
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

#[derive(Event)]
pub struct EnemyDied {
    pub position: Vec2,
    pub loot_value: u32,
    /// Additional scrap the enemy had stolen from the pile.
    pub stolen_scrap: u32,
    /// Number of enemies to spawn on death (boss splitting trait).
    pub splits: u32,
    /// Type of enemy that died (for stats tracking).
    pub enemy_type: EnemyType,
    /// Sprite color at time of death (for death particles).
    pub color: Color,
}

#[derive(Event)]
pub struct EnemyEscaped {
    pub stolen_scrap: u32,
    pub enemy_type: EnemyType,
}

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
        // Each enemy walks toward a jittered point within the next grid cell
        // so that enemies on the same path don't stack on identical pixels.
        let jitter_vec = jitter.as_deref().map_or(Vec2::ZERO, |j| j.0);
        let target_world = (grid_to_world_cfg(next_pos.0, &config) + jitter_vec).extend(1.0);
        let direction = target_world - transform.translation;
        let distance = direction.length();

        // Rotate toward direction of travel.
        if distance > 1.0 {
            let to_target = direction.truncate().normalize();
            rotate_toward(
                &mut transform,
                to_target,
                ENEMY_ROTATION_SPEED,
                time.delta_secs(),
            );
        }

        // Move toward the jittered target; snap on arrival.
        let step_size = speed.current * time.delta_secs();
        let arrived = distance < 1.0 || step_size >= distance;

        if arrived {
            // Snap to cell, advance pathfinding, and roll a fresh jitter
            // offset for the next leg.
            agent_pos.0 = next_pos.0;
            transform.translation = target_world;
            commands.entity(entity).remove::<NextPos>();
            commands
                .entity(entity)
                .insert(CellJitter(random_cell_jitter(&mut rng)));
        } else {
            transform.translation += direction / distance * step_size;
            // Keep AgentPos in sync with the visual position so that
            // path recalculation (GridChanged) starts from where the
            // enemy actually is, not from a stale completed waypoint.
            // Only update to passable cells — a tower placed on the cell
            // the enemy is crossing must not poison AgentPos.
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

/// Approaching enemies that reach a pile cell steal scrap and start fleeing.
/// If the pile is empty, enemies flee to the edge with nothing.
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
        With<Enemy>,
    >,
    mut pile_scrap: ResMut<PileScrap>,
    edge_cells: Res<EdgeCells>,
) {
    for (entity, agent_pos, loot, mut state, next_pos, pathfind_req, path) in &mut enemies {
        if *state != EnemyState::Approaching {
            continue;
        }
        // Enemy is still navigating toward the pile.
        let path_active = next_pos.is_some()
            || pathfind_req.is_some()
            || path.is_some_and(|p| !p.path().is_empty());
        if path_active {
            continue;
        }

        // Enemy reached the pile — steal what we can and flee.
        let steal_amount = compute_steal_amount(loot.0, pile_scrap.amount);
        pile_scrap.amount = pile_scrap.amount.saturating_sub(steal_amount);

        let flee_target = nearest_edge_cell(agent_pos.0, &edge_cells.0);
        *state = EnemyState::Fleeing;
        commands
            .entity(entity)
            .insert(Pathfind::new(flee_target).mode(PathfindMode::AStar));

        if steal_amount > 0 {
            commands.entity(entity).insert(StolenScrap(steal_amount));
            // Visual decal: small gold square to indicate carried scrap.
            commands.entity(entity).with_child((
                ScrapCarrierDecal,
                Sprite::from_color(SCRAP_COLOR, Vec2::splat(6.0)),
                Transform::from_translation(Vec3::new(0.0, -5.0, 0.1)),
            ));
        }
    }
}

/// Fleeing enemies that reach the map edge escape with stolen scrap.
pub fn enemy_escaped(
    mut commands: Commands,
    enemies: Query<
        (
            Entity,
            &EnemyState,
            &Transform,
            &EnemyType,
            Option<&StolenScrap>,
        ),
        With<Enemy>,
    >,
    config: Res<GridConfig>,
) {
    for (entity, state, transform, enemy_type, stolen) in &enemies {
        if *state != EnemyState::Fleeing {
            continue;
        }
        let pos = transform.translation.truncate();
        if !is_at_map_edge(pos, config.width, config.height) {
            continue;
        }
        // Stolen scrap is permanently lost — already subtracted from pile.
        commands.trigger(EnemyEscaped {
            stolen_scrap: stolen.map_or(0, |s| s.0),
            enemy_type: *enemy_type,
        });
        commands.entity(entity).remove::<Enemy>();
        commands.entity(entity).insert(DeathAnimation {
            timer: Timer::from_seconds(DEATH_ANIM_SECS, TimerMode::Once),
        });
    }
}

/// Mark enemies with zero health as dying: remove `Enemy`, trigger loot event.
pub fn check_enemy_death(
    mut commands: Commands,
    enemies: Query<
        (
            Entity,
            &Health,
            &Transform,
            &LootValue,
            &Sprite,
            &EnemyType,
            Option<&StolenScrap>,
            Option<&SplitsOnDeath>,
        ),
        With<Enemy>,
    >,
) {
    for (entity, health, transform, loot, sprite, enemy_type, stolen, splits) in &enemies {
        if health.current <= 0.0 {
            let stolen_amount = stolen.map_or(0, |s| s.0);
            let split_count = splits.map_or(0, |s| s.count);
            let position = transform.translation.truncate();
            commands.trigger(EnemyDied {
                position,
                loot_value: loot.0,
                stolen_scrap: stolen_amount,
                splits: split_count,
                enemy_type: *enemy_type,
                color: sprite.color,
            });

            commands.entity(entity).remove::<Enemy>();
            commands.entity(entity).insert(DeathAnimation {
                timer: Timer::from_seconds(DEATH_ANIM_SECS, TimerMode::Once),
            });
        }
    }
}

pub fn on_enemy_died_sound(_trigger: On<EnemyDied>, mut commands: Commands) {
    commands.trigger(PlaySound(GameSound::EnemyDeath));
}

pub fn on_enemy_died_particles(trigger: On<EnemyDied>, mut commands: Commands) {
    spawn_death_particles(
        &mut commands,
        trigger.position,
        trigger.color,
        &mut rand::rng(),
    );
}

pub fn on_enemy_died_shake(trigger: On<EnemyDied>, mut shake: ResMut<ScreenShake>) {
    if matches!(trigger.enemy_type, EnemyType::Boss) {
        shake.intensity = 6.0;
        shake.timer = Timer::from_seconds(0.5, TimerMode::Once);
        shake.decay = 0.03;
    }
}

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
                // Counter-rotate position and orientation so the bar stays
                // centered above the enemy regardless of its rotation.
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
        // Ease-out cubic: 1 - (1-t)^3
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
        // Shrink and fade out.
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

pub fn spawn_enemy(
    commands: &mut Commands,
    enemy_type: EnemyType,
    spawn_pos: UVec3,
    goal_pos: UVec3,
    grid_entity: Entity,
    health_mult: f32,
    speed_mult: f32,
    config: &GridConfig,
    boss_trait: Option<BossTrait>,
) {
    debug!("enemy spawned at {spawn_pos:?} → goal {goal_pos:?}");
    let world_pos = grid_to_world_cfg(spawn_pos, config);
    let size = enemy_type.size();
    let health = enemy_type.base_health() * health_mult;
    let speed = enemy_type.base_speed() * speed_mult;
    let mut rng = rand::rng();

    let entity_id = commands
        .spawn((
            Enemy,
            enemy_type,
            EnemyState::Approaching,
            Health {
                current: health,
                max: health,
            },
            MoveSpeed {
                base: speed,
                current: speed,
            },
            LootValue(enemy_type.loot_value()),
            Sprite::from_color(enemy_type.color(), Vec2::splat(size)),
            Transform::from_translation(world_pos.extend(1.0)).with_scale(Vec3::ZERO),
            SpawnAnimation {
                timer: Timer::from_seconds(0.25, TimerMode::Once),
            },
            CellJitter(random_cell_jitter(&mut rng)),
            AgentPos(spawn_pos),
            AgentOfGrid(grid_entity),
            Pathfind::new(goal_pos).mode(PathfindMode::AStar),
        ))
        .with_child((
            HealthBar {
                y_offset: size / 2.0 + 3.0,
            },
            Sprite::from_color(Color::srgb(0.2, 0.8, 0.2), Vec2::new(16.0, 2.0)),
            Transform::from_translation(Vec3::new(0.0, size / 2.0 + 3.0, 0.1)),
        ))
        .id();

    if matches!(enemy_type, EnemyType::Brute) {
        commands.entity(entity_id).insert(BruteAttack {
            cooldown: Timer::from_seconds(BRUTE_ATTACK_COOLDOWN, TimerMode::Once),
            damage: BRUTE_ATTACK_DAMAGE,
        });
    }

    match boss_trait {
        Some(BossTrait::Regeneration) => {
            commands
                .entity(entity_id)
                .insert(Regeneration { rate: 5.0 });
        }
        Some(BossTrait::Armor) => {
            commands.entity(entity_id).insert(Armor { reduction: 5.0 });
        }
        Some(BossTrait::Splitting) => {
            commands
                .entity(entity_id)
                .insert(SplitsOnDeath { count: 3 });
        }
        None => {}
    }
}

/// Boss regeneration: heal over time.
pub fn boss_regeneration(
    mut query: Query<(&Regeneration, &mut Health), With<Enemy>>,
    time: Res<Time>,
) {
    for (regen, mut health) in &mut query {
        health.current = (health.current + regen.rate * time.delta_secs()).min(health.max);
    }
}

/// On boss death with splitting trait, spawn smaller enemies at the death position.
pub fn on_boss_split(
    trigger: On<EnemyDied>,
    mut commands: Commands,
    config: Res<GridConfig>,
    grid_query: Query<Entity, With<OrdinalGrid>>,
    pile_state: Res<PileState>,
) {
    let event = &*trigger;
    if event.splits == 0 {
        return;
    }

    let Ok(grid_entity) = grid_query.single() else {
        return;
    };

    let Some(grid_pos) = world_to_grid(event.position, &config) else {
        return;
    };
    let spawn_pos = UVec3::new(grid_pos.x as u32, grid_pos.y as u32, 0);
    let goal_pos = nearest_pile_cell(spawn_pos, &pile_state);

    for _ in 0..event.splits {
        spawn_enemy(
            &mut commands,
            EnemyType::Shambler,
            spawn_pos,
            goal_pos,
            grid_entity,
            1.0,
            1.0,
            &config,
            None,
        );
    }
}

// ---------------------------------------------------------------------------
// Brute tower attack
// ---------------------------------------------------------------------------

/// Brutes attack adjacent impassable towers, dealing damage over time.
pub fn brute_attack_towers(
    mut brutes: Query<(&Transform, &mut BruteAttack), (With<Enemy>, Without<Tower>)>,
    mut towers: Query<
        (
            Entity,
            &Transform,
            &mut TowerHealth,
            &BaseStats,
            &TowerTier,
            &mut Sprite,
            &mut TowerState,
        ),
        (With<Tower>, With<BlocksNav>, Without<Enemy>),
    >,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (brute_tf, mut attack) in &mut brutes {
        attack.cooldown.tick(time.delta());
        if !attack.cooldown.is_finished() {
            continue;
        }

        let brute_pos = brute_tf.translation.truncate();

        // Find nearest tower in attack range.
        let mut best: Option<(Entity, f32)> = None;
        for (entity, tower_tf, _, _, _, _, tower_state) in &towers {
            if !tower_state.is_operational() {
                continue;
            }
            let dist = brute_pos.distance(tower_tf.translation.truncate());
            if dist <= BRUTE_ATTACK_RANGE && best.is_none_or(|(_, d)| dist < d) {
                best = Some((entity, dist));
            }
        }

        let Some((target_entity, _)) = best else {
            continue;
        };

        attack.cooldown.reset();

        if let Ok((entity, _, mut health, base, tier, mut sprite, mut tower_state)) =
            towers.get_mut(target_entity)
        {
            health.current = (health.current - attack.damage).max(0.0);

            commands.trigger(PlaySound(GameSound::EnemyBruteAttack));

            // Flash white then restore to degradation color.
            sprite.color = Color::WHITE;
            commands.entity(entity).insert(UpgradeFlash {
                timer: Timer::from_seconds(0.1, TimerMode::Once),
                target_color: degradation_color(base.color, tier.0, &health),
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
        // half_w = 40 * 20 / 2 = 400; margin = 20; threshold = -380
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
        // half_h = 32 * 20 / 2 = 320; margin = 20; threshold = 300
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

    // -- spawn_enemy integration tests --

    use crate::test_helpers::test_grid_config;
    use crate::wave::resources::BossTrait;

    /// Helper: spawn a single enemy via a startup system, return the App.
    fn spawn_test_app(
        enemy_type: EnemyType,
        health_mult: f32,
        speed_mult: f32,
        boss_trait: Option<BossTrait>,
    ) -> App {
        let mut app = crate::test_helpers::test_app();
        let config = test_grid_config();
        let et = enemy_type;
        let bt = boss_trait;
        app.add_systems(Startup, move |mut commands: Commands| {
            let grid_entity = commands.spawn_empty().id();
            spawn_enemy(
                &mut commands,
                et,
                UVec3::ZERO,
                UVec3::new(20, 16, 0),
                grid_entity,
                health_mult,
                speed_mult,
                &config,
                bt,
            );
        });
        app.update();
        app
    }

    #[test]
    fn spawn_shambler_has_core_components() {
        let mut app = spawn_test_app(EnemyType::Shambler, 1.0, 1.0, None);
        let mut query = app.world_mut().query_filtered::<Entity, With<Enemy>>();
        let entities: Vec<Entity> = query.iter(app.world()).collect();
        assert_eq!(entities.len(), 1, "should spawn exactly one enemy");

        let e = entities[0];
        let world = app.world();
        assert!(world.get::<Enemy>(e).is_some());
        assert_eq!(*world.get::<EnemyType>(e).unwrap(), EnemyType::Shambler);
        assert!(world.get::<Health>(e).is_some());
        assert!(world.get::<MoveSpeed>(e).is_some());
        assert!(world.get::<LootValue>(e).is_some());
        assert!(world.get::<CellJitter>(e).is_some());
        assert!(world.get::<SpawnAnimation>(e).is_some());
        assert_eq!(
            *world.get::<EnemyState>(e).unwrap(),
            EnemyState::Approaching
        );
    }

    #[test]
    fn spawn_health_scales_with_multiplier() {
        let mut app = spawn_test_app(EnemyType::Shambler, 2.0, 1.0, None);
        let mut query = app.world_mut().query::<&Health>();
        let health = query.single(app.world()).unwrap();
        let expected = EnemyType::Shambler.base_health() * 2.0;
        assert!((health.max - expected).abs() < f32::EPSILON);
        assert!((health.current - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn spawn_speed_scales_with_multiplier() {
        let mut app = spawn_test_app(EnemyType::Runner, 1.0, 1.5, None);
        let mut query = app.world_mut().query::<&MoveSpeed>();
        let speed = query.single(app.world()).unwrap();
        let expected = EnemyType::Runner.base_speed() * 1.5;
        assert!((speed.base - expected).abs() < f32::EPSILON);
    }

    #[test]
    fn spawn_brute_has_brute_attack() {
        let mut app = spawn_test_app(EnemyType::Brute, 1.0, 1.0, None);
        let mut query = app.world_mut().query_filtered::<Entity, With<Enemy>>();
        let e = query.single(app.world()).unwrap();
        assert!(app.world().get::<BruteAttack>(e).is_some());
    }

    #[test]
    fn spawn_non_brute_no_brute_attack() {
        let mut app = spawn_test_app(EnemyType::Shambler, 1.0, 1.0, None);
        let mut query = app.world_mut().query_filtered::<Entity, With<Enemy>>();
        let e = query.single(app.world()).unwrap();
        assert!(app.world().get::<BruteAttack>(e).is_none());
    }

    #[test]
    fn spawn_boss_regeneration_trait() {
        let mut app = spawn_test_app(EnemyType::Boss, 1.0, 1.0, Some(BossTrait::Regeneration));
        let mut query = app.world_mut().query_filtered::<Entity, With<Enemy>>();
        let e = query.single(app.world()).unwrap();
        assert!(app.world().get::<Regeneration>(e).is_some());
        assert!(app.world().get::<Armor>(e).is_none());
        assert!(app.world().get::<SplitsOnDeath>(e).is_none());
    }

    #[test]
    fn spawn_boss_armor_trait() {
        let mut app = spawn_test_app(EnemyType::Boss, 1.0, 1.0, Some(BossTrait::Armor));
        let mut query = app.world_mut().query_filtered::<Entity, With<Enemy>>();
        let e = query.single(app.world()).unwrap();
        assert!(app.world().get::<Armor>(e).is_some());
    }

    #[test]
    fn spawn_boss_splitting_trait() {
        let mut app = spawn_test_app(EnemyType::Boss, 1.0, 1.0, Some(BossTrait::Splitting));
        let mut query = app.world_mut().query_filtered::<Entity, With<Enemy>>();
        let e = query.single(app.world()).unwrap();
        assert!(app.world().get::<SplitsOnDeath>(e).is_some());
    }

    #[test]
    fn spawn_loot_matches_type() {
        let mut app = spawn_test_app(EnemyType::Runner, 1.0, 1.0, None);
        let mut query = app.world_mut().query::<&LootValue>();
        let loot = query.single(app.world()).unwrap();
        assert_eq!(loot.0, EnemyType::Runner.loot_value());
    }
}
