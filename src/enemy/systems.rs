use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::grid::systems::grid_to_world;
use crate::grid::components::{GoalPoint, GridCell};
use crate::ui::hud::PlayerLives;

use super::components::*;

#[derive(Event)]
pub struct EnemyDied {
    pub position: Vec2,
    pub loot_value: u32,
}

pub fn enemy_movement(
    mut query: Query<(Entity, &mut AgentPos, &NextPos, &mut Transform, &MoveSpeed)>,
    mut commands: Commands,
    time: Res<Time>,
) {
    for (entity, mut agent_pos, next_pos, mut transform, speed) in &mut query {
        let target_world = grid_to_world(next_pos.0).extend(1.0);
        let current = transform.translation;
        let direction = target_world - current;
        let distance = direction.length();

        if distance < 1.0 {
            // Arrived at next cell
            agent_pos.0 = next_pos.0;
            transform.translation = target_world;
            commands.entity(entity).remove::<NextPos>();
        } else {
            let step = direction.normalize() * speed.current * time.delta_secs();
            if step.length() >= distance {
                agent_pos.0 = next_pos.0;
                transform.translation = target_world;
                commands.entity(entity).remove::<NextPos>();
            } else {
                transform.translation += step;
            }
        }
    }
}

pub fn enemy_reached_goal(
    mut commands: Commands,
    enemies: Query<(Entity, &AgentPos), With<Enemy>>,
    goals: Query<&GridCell, With<GoalPoint>>,
    mut lives: ResMut<PlayerLives>,
) {
    for (entity, agent_pos) in &enemies {
        for goal_cell in &goals {
            let goal_uvec = UVec3::new(goal_cell.coord.x as u32, goal_cell.coord.y as u32, 0);
            if agent_pos.0 == goal_uvec {
                lives.0 = lives.0.saturating_sub(1);
                commands.entity(entity).despawn();
            }
        }
    }
}

pub fn check_enemy_death(
    mut commands: Commands,
    enemies: Query<(Entity, &Health, &Transform, &LootValue), With<Enemy>>,
) {
    for (entity, health, transform, loot) in &enemies {
        if health.current <= 0.0 {
            commands.trigger(EnemyDied {
                position: transform.translation.truncate(),
                loot_value: loot.0,
            });
            commands.entity(entity).despawn();
        }
    }
}

pub fn apply_slow_effects(
    mut commands: Commands,
    mut query: Query<(Entity, &mut MoveSpeed, &mut SlowEffect)>,
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

pub fn spawn_enemy(
    commands: &mut Commands,
    enemy_type: EnemyType,
    spawn_pos: UVec3,
    goal_pos: UVec3,
    grid_entity: Entity,
    health_mult: f32,
    speed_mult: f32,
) {
    let world_pos = grid_to_world(spawn_pos);
    let size = enemy_type.size();
    let health = enemy_type.base_health() * health_mult;
    let speed = enemy_type.base_speed() * speed_mult;

    commands.spawn((
        Enemy { enemy_type },
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
        Transform::from_translation(world_pos.extend(1.0)),
        AgentPos(spawn_pos),
        AgentOfGrid(grid_entity),
        Pathfind::new(goal_pos),
    ));
}
