use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::Rng;

use crate::common::constants::{GridConfig, TILE_SIZE};
use crate::grid::systems::{grid_to_world, grid_to_world_cfg};
use crate::pile::resources::{EdgeCells, PileScrap, PileState};
use crate::pile::systems::nearest_edge_cell;

use super::components::*;

#[derive(Event)]
pub struct EnemyDied {
    pub position: Vec2,
    pub loot_value: u32,
    /// Additional scrap the enemy had stolen from the pile.
    pub stolen_scrap: u32,
}

#[derive(Component)]
pub struct HealthBar;

pub fn enemy_movement(
    mut query: Query<(Entity, &mut AgentPos, &NextPos, &mut Transform, &MoveSpeed, Option<&mut WanderOffset>), (Without<Dead>, Without<Dying>)>,
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

        if distance < 1.0 {
            agent_pos.0 = next_pos.0;
            transform.translation = target_world;
            commands.entity(entity).remove::<NextPos>();
            // Re-roll wander offset for next cell.
            commands.entity(entity).insert(WanderOffset(Vec2::new(
                rng.random_range(-3.0..3.0),
                rng.random_range(-3.0..3.0),
            )));
        } else {
            let step = direction.normalize() * speed.current * time.delta_secs();
            if step.length() >= distance {
                agent_pos.0 = next_pos.0;
                transform.translation = target_world;
                commands.entity(entity).remove::<NextPos>();
                commands.entity(entity).insert(WanderOffset(Vec2::new(
                    rng.random_range(-3.0..3.0),
                    rng.random_range(-3.0..3.0),
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
    enemies: Query<(Entity, &AgentPos, &LootValue, &EnemyPhase, &Transform, Option<&SearchWander>), (With<Enemy>, Without<Dead>, Without<Dying>)>,
    pile_state: Res<PileState>,
    mut pile_scrap: ResMut<PileScrap>,
    edge_cells: Res<EdgeCells>,
) {
    let mut rng = rand::rng();
    for (entity, agent_pos, loot, phase, transform, search) in &enemies {
        if *phase != EnemyPhase::Approaching {
            continue;
        }
        if !pile_state.cells.contains(&agent_pos.0) {
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
            Pathfind::new(flee_target),
        ));
        commands.entity(entity).remove::<SearchWander>();

        // Visual decal: small gold square on the enemy to indicate carried scrap.
        commands.entity(entity).with_child((
            ScrapCarrierDecal,
            Sprite::from_color(
                Color::srgb(1.0, 0.85, 0.1),
                Vec2::splat(6.0),
            ),
            Transform::from_translation(Vec3::new(0.0, -5.0, 0.1)),
        ));
    }
}

/// Wander movement for enemies searching an empty pile.
pub fn search_wander_movement(
    mut query: Query<(&mut Transform, &MoveSpeed, &mut SearchWander), (With<Enemy>, Without<Dead>, Without<Dying>)>,
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
    let radius = TILE_SIZE * 1.5;
    Vec2::new(
        center.x + rng.random_range(-radius..radius),
        center.y + rng.random_range(-radius..radius),
    )
}

/// Fleeing enemies that reach the map edge escape with stolen scrap.
pub fn enemy_escaped(
    mut commands: Commands,
    enemies: Query<(Entity, &AgentPos, &EnemyPhase), (With<Enemy>, Without<Dead>, Without<Dying>)>,
    config: Res<GridConfig>,
) {
    for (entity, agent_pos, phase) in &enemies {
        if *phase != EnemyPhase::Fleeing {
            continue;
        }
        let p = agent_pos.0;
        let is_edge = p.x == 0 || p.x == config.width - 1 || p.y == 0 || p.y == config.height - 1;
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
    enemies: Query<(Entity, &Health, &Transform, &LootValue, Option<&StolenScrap>), (With<Enemy>, Without<Dead>, Without<Dying>)>,
) {
    for (entity, health, transform, loot, stolen) in &enemies {
        if health.current <= 0.0 {
            let stolen_amount = stolen.map_or(0, |s| s.0);
            commands.trigger(EnemyDied {
                position: transform.translation.truncate(),
                loot_value: loot.0,
                stolen_scrap: stolen_amount,
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
    mut bars: Query<(&mut Sprite, &mut Transform), (With<HealthBar>, Without<Enemy>)>,
) {
    for (health, _enemy_tf, children) in &enemies {
        for child in children.iter() {
            if let Ok((mut sprite, mut bar_tf)) = bars.get_mut(child) {
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
                bar_tf.translation.x = -bar_width * (1.0 - frac) / 2.0;
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

/// Expand and fade AoE burst visuals.
pub fn animate_aoe_burst(
    mut commands: Commands,
    mut query: Query<(Entity, &mut AoEBurst, &mut Sprite, &mut Transform)>,
    time: Res<Time>,
) {
    for (entity, mut burst, mut sprite, mut tf) in &mut query {
        burst.timer.tick(time.delta());
        let t = burst.timer.fraction();
        let size = burst.max_radius * t;
        sprite.custom_size = Some(Vec2::splat(size));
        sprite.color = sprite.color.with_alpha(0.4 * (1.0 - t));
        tf.scale = Vec3::ONE; // size is driven by custom_size, not scale
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
) {
    let world_pos = grid_to_world_cfg(spawn_pos, config);
    let size = enemy_type.size();
    let health = enemy_type.base_health() * health_mult;
    let speed = enemy_type.base_speed() * speed_mult;
    let mut rng = rand::rng();

    commands
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
            Transform::from_translation(world_pos.extend(1.0))
                .with_scale(Vec3::ZERO),
            SpawnAnimation {
                timer: Timer::from_seconds(0.25, TimerMode::Once),
            },
            WanderOffset(Vec2::new(
                rng.random_range(-3.0..3.0),
                rng.random_range(-3.0..3.0),
            )),
            AgentPos(spawn_pos),
            AgentOfGrid(grid_entity),
            Pathfind::new(goal_pos),
        ))
        .with_child((
            HealthBar,
            Sprite::from_color(Color::srgb(0.2, 0.8, 0.2), Vec2::new(16.0, 2.0)),
            Transform::from_translation(Vec3::new(0.0, size / 2.0 + 3.0, 0.1)),
        ));
}
