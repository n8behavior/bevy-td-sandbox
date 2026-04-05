use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::Rng;

use bevy::sprite_render::MeshMaterial2d;

use crate::common::constants::{GridConfig, TILE_SIZE};
use crate::grid::systems::{grid_to_world, grid_to_world_cfg, world_to_grid};
use crate::pile::resources::{EdgeCells, PileScrap, PileState};
use crate::pile::systems::{nearest_edge_cell, nearest_pile_cell};
use crate::shader::CircleMaterial;

use crate::wave::resources::BossTrait;

use super::components::*;

/// Radians per second enemies rotate toward their travel direction.
const ENEMY_ROTATION_SPEED: f32 = 6.0;

#[derive(Event)]
pub struct EnemyDied {
    pub position: Vec2,
    pub loot_value: u32,
    /// Additional scrap the enemy had stolen from the pile.
    pub stolen_scrap: u32,
    /// Number of enemies to spawn on death (boss splitting trait).
    pub splits: u32,
}

#[derive(Component)]
pub struct HealthBar {
    pub y_offset: f32,
}

pub fn enemy_movement(
    mut query: Query<
        (
            Entity,
            &mut AgentPos,
            &NextPos,
            &mut Transform,
            &MoveSpeed,
            Option<&mut WanderOffset>,
        ),
        (Without<Dead>, Without<Dying>),
    >,
    mut commands: Commands,
    time: Res<Time>,
    config: Res<GridConfig>,
) {
    let mut rng = rand::rng();
    for (entity, mut agent_pos, next_pos, mut transform, speed, wander) in &mut query {
        let wander_vec = wander.as_deref().map_or(Vec2::ZERO, |w| w.0);
        let target_world = (grid_to_world(next_pos.0, &config) + wander_vec).extend(1.0);
        let current = transform.translation;
        let direction = target_world - current;
        let distance = direction.length();

        // Rotate toward direction of travel.
        if distance > 1.0 {
            let dir2 = direction.truncate();
            let goal = Quat::from_rotation_z(dir2.y.atan2(dir2.x));
            let dot = transform.rotation.dot(goal);
            let goal = if dot < 0.0 { -goal } else { goal };
            let angle_remaining = transform.rotation.angle_between(goal);
            let max_step = ENEMY_ROTATION_SPEED * time.delta_secs();
            let t = if angle_remaining > 0.0 {
                (max_step / angle_remaining).min(1.0)
            } else {
                1.0
            };
            transform.rotation = transform.rotation.slerp(goal, t);
        }

        if distance < 1.0 {
            agent_pos.0 = next_pos.0;
            transform.translation = target_world;
            commands.entity(entity).remove::<NextPos>();
            commands.entity(entity).insert(WanderOffset(Vec2::new(
                rng.random_range(-7.0..7.0),
                rng.random_range(-7.0..7.0),
            )));
        } else {
            let step = direction.normalize() * speed.current * time.delta_secs();
            if step.length() >= distance {
                agent_pos.0 = next_pos.0;
                transform.translation = target_world;
                commands.entity(entity).remove::<NextPos>();
                commands.entity(entity).insert(WanderOffset(Vec2::new(
                    rng.random_range(-7.0..7.0),
                    rng.random_range(-7.0..7.0),
                )));
            } else {
                transform.translation += step;
            }
        }
    }
}

/// Approaching enemies that reach a pile cell steal scrap and start fleeing.
/// If the pile is empty, enemies wander on the pile searching for scrap.
pub fn enemy_reached_pile(
    mut commands: Commands,
    enemies: Query<
        (
            Entity,
            &AgentPos,
            &LootValue,
            &EnemyPhase,
            &Transform,
            Option<&SearchWander>,
            Option<&NextPos>,
            Option<&Pathfind>,
            Option<&Path>,
        ),
        (With<Enemy>, Without<Dead>, Without<Dying>),
    >,
    mut pile_scrap: ResMut<PileScrap>,
    edge_cells: Res<EdgeCells>,
) {
    let mut rng = rand::rng();
    for (entity, agent_pos, loot, phase, transform, search, next_pos, pathfind_req, path) in
        &enemies
    {
        if *phase != EnemyPhase::Approaching {
            continue;
        }
        // Enemy is still navigating toward the pile.
        let path_active = next_pos.is_some()
            || pathfind_req.is_some()
            || path.is_some_and(|p| !p.path().is_empty());
        if path_active {
            continue;
        }

        // Nothing to steal — start wandering if not already.
        if pile_scrap.amount == 0 {
            if search.is_none() {
                let pos = transform.translation.truncate();
                commands.entity(entity).insert(SearchWander {
                    target: random_wander_target(&mut rng, pos),
                    timer: Timer::from_seconds(rng.random_range(1.0..2.5), TimerMode::Once),
                });
            }
            continue;
        }

        let steal_amount = loot.0.min(pile_scrap.amount);
        pile_scrap.amount = pile_scrap.amount.saturating_sub(steal_amount);

        let flee_target = nearest_edge_cell(agent_pos.0, &edge_cells.0);

        commands.entity(entity).insert((
            EnemyPhase::Fleeing,
            StolenScrap(steal_amount),
            Pathfind::new(flee_target).mode(PathfindMode::Waypoints),
        ));
        commands.entity(entity).remove::<SearchWander>();

        // Visual decal: small gold square on the enemy to indicate carried scrap.
        commands.entity(entity).with_child((
            ScrapCarrierDecal,
            Sprite::from_color(Color::srgb(1.0, 0.85, 0.1), Vec2::splat(6.0)),
            Transform::from_translation(Vec3::new(0.0, -5.0, 0.1)),
        ));
    }
}

/// Wander movement for enemies searching an empty pile.
pub fn search_wander_movement(
    mut query: Query<
        (&mut Transform, &MoveSpeed, &mut SearchWander),
        (With<Enemy>, Without<Dead>, Without<Dying>),
    >,
    time: Res<Time>,
) {
    let mut rng = rand::rng();
    for (mut transform, speed, mut search) in &mut query {
        search.timer.tick(time.delta());

        let pos = transform.translation.truncate();
        let dir = search.target - pos;
        let dist = dir.length();

        // Drift toward wander target at half speed.
        if dist > 1.0 {
            let goal = Quat::from_rotation_z(dir.y.atan2(dir.x));
            let dot = transform.rotation.dot(goal);
            let goal = if dot < 0.0 { -goal } else { goal };
            let angle_remaining = transform.rotation.angle_between(goal);
            let max_step = ENEMY_ROTATION_SPEED * time.delta_secs();
            let t = if angle_remaining > 0.0 {
                (max_step / angle_remaining).min(1.0)
            } else {
                1.0
            };
            transform.rotation = transform.rotation.slerp(goal, t);

            let step = dir.normalize() * speed.current * 0.4 * time.delta_secs();
            transform.translation += step.extend(0.0);
        }

        // Pick a new wander target when timer expires or close enough.
        if search.timer.is_finished() || dist < 2.0 {
            search.target = random_wander_target(&mut rng, pos);
            search.timer = Timer::from_seconds(rng.random_range(1.0..2.5), TimerMode::Once);
        }
    }
}

fn random_wander_target(rng: &mut impl Rng, center: Vec2) -> Vec2 {
    let radius = 30.0;
    Vec2::new(
        center.x + rng.random_range(-radius..radius),
        center.y + rng.random_range(-radius..radius),
    )
}

/// Fleeing enemies that reach the map edge escape with stolen scrap.
pub fn enemy_escaped(
    mut commands: Commands,
    enemies: Query<(Entity, &EnemyPhase, &Transform), (With<Enemy>, Without<Dead>, Without<Dying>)>,
    config: Res<GridConfig>,
) {
    for (entity, phase, transform) in &enemies {
        if *phase != EnemyPhase::Fleeing {
            continue;
        }
        let pos = transform.translation.truncate();
        let half_w = config.width as f32 * TILE_SIZE / 2.0;
        let half_h = config.height as f32 * TILE_SIZE / 2.0;
        let margin = TILE_SIZE;
        let is_edge = pos.x <= -half_w + margin
            || pos.x >= half_w - margin
            || pos.y <= -half_h + margin
            || pos.y >= half_h - margin;
        if !is_edge {
            continue;
        }
        // Stolen scrap is permanently lost — already subtracted from pile.
        commands.entity(entity).insert((
            Dying,
            DeathAnimation {
                timer: Timer::from_seconds(0.3, TimerMode::Once),
            },
        ));
    }
}

/// Mark enemies with zero health as Dying, trigger loot event.
pub fn check_enemy_death(
    mut commands: Commands,
    enemies: Query<
        (
            Entity,
            &Health,
            &Transform,
            &LootValue,
            Option<&StolenScrap>,
            Option<&SplitsOnDeath>,
        ),
        (With<Enemy>, Without<Dead>, Without<Dying>),
    >,
) {
    for (entity, health, transform, loot, stolen, splits) in &enemies {
        if health.current <= 0.0 {
            let stolen_amount = stolen.map_or(0, |s| s.0);
            let split_count = splits.map_or(0, |s| s.count);
            commands.trigger(EnemyDied {
                position: transform.translation.truncate(),
                loot_value: loot.0,
                stolen_scrap: stolen_amount,
                splits: split_count,
            });
            commands.entity(entity).insert((
                Dying,
                DeathAnimation {
                    timer: Timer::from_seconds(0.3, TimerMode::Once),
                },
            ));
        }
    }
}

pub fn apply_slow_effects(
    mut commands: Commands,
    mut query: Query<(Entity, &mut MoveSpeed, &mut SlowEffect), Without<Dead>>,
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
    enemies: Query<(&Health, &Transform, &Children), (With<Enemy>, Without<Dead>)>,
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

/// Actually despawn all Dead entities. Runs last in the frame.
pub fn cleanup_dead(mut commands: Commands, dead: Query<Entity, With<Dead>>) {
    for entity in &dead {
        commands.entity(entity).despawn();
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

/// Shrink + fade death animation. Inserts `Dead` when complete.
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
            commands.entity(entity).insert(Dead);
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
    let world_pos = grid_to_world_cfg(spawn_pos, config);
    let size = enemy_type.size();
    let health = enemy_type.base_health() * health_mult;
    let speed = enemy_type.base_speed() * speed_mult;
    let mut rng = rand::rng();

    let entity_id = commands
        .spawn((
            Enemy,
            EnemyPhase::Approaching,
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
            WanderOffset(Vec2::new(
                rng.random_range(-7.0..7.0),
                rng.random_range(-7.0..7.0),
            )),
            AgentPos(spawn_pos),
            AgentOfGrid(grid_entity),
            Pathfind::new(goal_pos).mode(PathfindMode::Waypoints),
        ))
        .with_child((
            HealthBar {
                y_offset: size / 2.0 + 3.0,
            },
            Sprite::from_color(Color::srgb(0.2, 0.8, 0.2), Vec2::new(16.0, 2.0)),
            Transform::from_translation(Vec3::new(0.0, size / 2.0 + 3.0, 0.1)),
        ))
        .id();

    match boss_trait {
        Some(BossTrait::Regeneration) => {
            commands.entity(entity_id).insert(Regeneration { rate: 5.0 });
        }
        Some(BossTrait::Armor) => {
            commands.entity(entity_id).insert(Armor { reduction: 10.0 });
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
    mut query: Query<(&Regeneration, &mut Health), (With<Enemy>, Without<Dead>, Without<Dying>)>,
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
