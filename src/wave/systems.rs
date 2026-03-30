use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::common::constants::GridConfig;
use crate::enemy::components::{Dead, Enemy, EnemyType};
use crate::enemy::systems::spawn_enemy;
use crate::grid::components::{SpawnPoint, GoalPoint, GridCell};
use crate::states::PlayPhase;

use super::resources::*;

pub fn start_wave(mut wave_mgr: ResMut<WaveManager>) {
    let wave_idx = wave_mgr.current_wave as usize;
    if wave_idx < wave_mgr.waves.len() {
        let total: u32 = wave_mgr.waves[wave_idx]
            .enemies
            .iter()
            .map(|e| e.count)
            .sum();
        wave_mgr.enemies_remaining = total;
        wave_mgr.enemies_spawned = 0;
        let interval = wave_mgr.waves[wave_idx].spawn_interval;
        wave_mgr.spawn_timer = Timer::from_seconds(interval, TimerMode::Repeating);
    }
}

pub fn spawn_enemies(
    mut commands: Commands,
    mut wave_mgr: ResMut<WaveManager>,
    time: Res<Time>,
    config: Res<GridConfig>,
    grid_query: Query<Entity, With<CardinalGrid>>,
    spawn_query: Query<&GridCell, With<SpawnPoint>>,
    goal_query: Query<&GridCell, With<GoalPoint>>,
) {
    let Ok(grid_entity) = grid_query.single() else {
        return;
    };
    let Ok(spawn_cell) = spawn_query.single() else {
        return;
    };
    let Ok(goal_cell) = goal_query.single() else {
        return;
    };

    let spawn_pos = UVec3::new(spawn_cell.coord.x as u32, spawn_cell.coord.y as u32, 0);
    let goal_pos = UVec3::new(goal_cell.coord.x as u32, goal_cell.coord.y as u32, 0);

    wave_mgr.spawn_timer.tick(time.delta());

    if !wave_mgr.spawn_timer.just_finished() {
        return;
    }

    let wave_idx = wave_mgr.current_wave as usize;
    if wave_idx >= wave_mgr.waves.len() {
        return;
    }

    // Find which enemy type to spawn next
    let mut count = 0u32;
    let spawned = wave_mgr.enemies_spawned;
    let wave = &wave_mgr.waves[wave_idx];
    for we in &wave.enemies {
        if spawned < count + we.count {
            spawn_enemy(
                &mut commands,
                we.enemy_type,
                spawn_pos,
                goal_pos,
                grid_entity,
                we.health_multiplier,
                we.speed_multiplier,
                &config,
            );
            break;
        }
        count += we.count;
    }
    wave_mgr.enemies_spawned += 1;
}

pub fn check_wave_complete(
    mut wave_mgr: ResMut<WaveManager>,
    enemies: Query<(), (With<Enemy>, Without<Dead>)>,
    mut next_phase: ResMut<NextState<PlayPhase>>,
) {
    let wave_idx = wave_mgr.current_wave as usize;
    if wave_idx >= wave_mgr.waves.len() {
        return;
    }

    let total: u32 = wave_mgr.waves[wave_idx]
        .enemies
        .iter()
        .map(|e| e.count)
        .sum();

    // All spawned and all dead
    if wave_mgr.enemies_spawned >= total && enemies.is_empty() {
        wave_mgr.current_wave += 1;
        next_phase.set(PlayPhase::Building);
    }
}

pub fn handle_start_wave_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_phase: ResMut<NextState<PlayPhase>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        next_phase.set(PlayPhase::Defending);
    }
}

pub fn check_game_over(
    lives: Res<crate::ui::hud::PlayerLives>,
    mut next_state: ResMut<NextState<crate::states::GameState>>,
) {
    if lives.0 == 0 {
        next_state.set(crate::states::GameState::GameOver);
    }
}

pub fn generate_waves() -> Vec<WaveConfig> {
    let mut waves = Vec::new();

    for i in 0..20 {
        let wave_num = i + 1;
        let health_mult = 1.0 + (i as f32 * 0.15);
        let speed_mult = 1.0 + (i as f32 * 0.05);
        let base_count = 5 + i * 2;

        let mut enemies = vec![WaveEnemy {
            enemy_type: EnemyType::Shambler,
            count: base_count,
            health_multiplier: health_mult,
            speed_multiplier: speed_mult,
        }];

        if wave_num >= 3 {
            enemies.push(WaveEnemy {
                enemy_type: EnemyType::Runner,
                count: (base_count / 2).max(2),
                health_multiplier: health_mult,
                speed_multiplier: speed_mult,
            });
        }

        if wave_num >= 6 {
            enemies.push(WaveEnemy {
                enemy_type: EnemyType::Brute,
                count: (base_count / 4).max(1),
                health_multiplier: health_mult,
                speed_multiplier: speed_mult,
            });
        }

        let spawn_interval = (1.5 - i as f32 * 0.05).max(0.3);

        waves.push(WaveConfig {
            enemies,
            spawn_interval,
        });
    }

    waves
}
