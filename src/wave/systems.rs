use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::prelude::IndexedRandom;
use rand::seq::SliceRandom;

use crate::common::constants::GridConfig;
use crate::economy::components::ScrapDrop;
use crate::enemy::components::{Dead, Enemy, EnemyType, StolenScrap};
use crate::enemy::systems::spawn_enemy;
use crate::pile::resources::{EdgeCells, PileScrap, PileState};
use crate::pile::systems::nearest_pile_cell;
use crate::states::{GameState, PlayPhase};

use super::resources::*;

pub fn start_wave(mut wave_mgr: ResMut<WaveManager>) {
    let wave_idx = wave_mgr.current_wave as usize;
    if wave_idx >= wave_mgr.waves.len() {
        return;
    }

    // Build a flat, shuffled spawn queue from the wave config.
    let wave = &wave_mgr.waves[wave_idx];
    let mut queue: Vec<SpawnEntry> = wave
        .enemies
        .iter()
        .flat_map(|we| {
            (0..we.count).map(move |_| SpawnEntry {
                enemy_type: we.enemy_type,
                health_multiplier: we.health_multiplier,
                speed_multiplier: we.speed_multiplier,
                boss_trait: we.boss_trait,
            })
        })
        .collect();

    queue.shuffle(&mut rand::rng());

    let total = queue.len() as u32;
    let interval = wave.spawn_interval;

    wave_mgr.spawn_queue = queue;
    wave_mgr.enemies_remaining = total;
    wave_mgr.spawn_timer = Timer::from_seconds(interval, TimerMode::Repeating);
}

pub fn spawn_enemies(
    mut commands: Commands,
    mut wave_mgr: ResMut<WaveManager>,
    time: Res<Time>,
    config: Res<GridConfig>,
    grid_query: Query<Entity, With<OrdinalGrid>>,
    edge_cells: Res<EdgeCells>,
    pile_state: Res<PileState>,
) {
    let Ok(grid_entity) = grid_query.single() else {
        return;
    };

    if edge_cells.0.is_empty() || pile_state.cells.is_empty() {
        return;
    }

    wave_mgr.spawn_timer.tick(time.delta());

    if !wave_mgr.spawn_timer.just_finished() {
        return;
    }

    let Some(entry) = wave_mgr.spawn_queue.pop() else {
        return;
    };

    // Pick a random edge cell for this enemy's spawn.
    let mut rng = rand::rng();
    let spawn_pos = *edge_cells.0.choose(&mut rng).unwrap();
    let goal_pos = nearest_pile_cell(spawn_pos, &pile_state);

    spawn_enemy(
        &mut commands,
        entry.enemy_type,
        spawn_pos,
        goal_pos,
        grid_entity,
        entry.health_multiplier,
        entry.speed_multiplier,
        &config,
        entry.boss_trait,
    );
}

pub fn check_wave_complete(
    mut wave_mgr: ResMut<WaveManager>,
    enemies: Query<(), (With<Enemy>, Without<Dead>)>,
    drops: Query<(), With<ScrapDrop>>,
    mut next_phase: ResMut<NextState<PlayPhase>>,
) {
    // Wave isn't over until all enemies dead AND all ground scrap settled.
    if wave_mgr.spawn_queue.is_empty() && enemies.is_empty() && drops.is_empty() {
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

/// Game over only when truly bankrupt: pile empty, no scrap on ground,
/// no fleeing enemies carrying recoverable scrap, and no living enemies
/// that could still be killed for loot.
pub fn check_game_over(
    pile_scrap: Res<PileScrap>,
    drops: Query<(), With<ScrapDrop>>,
    enemies: Query<(), (With<Enemy>, Without<Dead>)>,
    stolen: Query<&StolenScrap, (With<Enemy>, Without<Dead>)>,
    wave_mgr: Res<WaveManager>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if pile_scrap.amount > 0 {
        return;
    }
    if !drops.is_empty() {
        return;
    }
    if stolen.iter().any(|s| s.0 > 0) {
        return;
    }
    // Enemies alive or queued to spawn can still be killed for loot.
    if !enemies.is_empty() || !wave_mgr.spawn_queue.is_empty() {
        return;
    }
    next_state.set(GameState::GameOver);
}

pub fn generate_waves() -> Vec<WaveConfig> {
    let mut waves = Vec::new();
    let mut rng = rand::rng();
    let boss_traits = [
        BossTrait::Regeneration,
        BossTrait::Armor,
        BossTrait::Splitting,
    ];

    for i in 0..20 {
        let wave_num = i + 1;
        let health_mult = 1.0 + (i as f32 * 0.15);
        let speed_mult = 1.0 + (i as f32 * 0.05);

        // Boss wave every 5th wave
        if wave_num % 5 == 0 {
            let boss_trait = *boss_traits.choose(&mut rng).unwrap();
            waves.push(WaveConfig {
                enemies: vec![WaveEnemy {
                    enemy_type: EnemyType::Boss,
                    count: 1,
                    health_multiplier: health_mult,
                    speed_multiplier: speed_mult,
                    boss_trait: Some(boss_trait),
                }],
                spawn_interval: 1.0,
            });
            continue;
        }

        let base_count = 5 + i * 2;

        let mut enemies = vec![WaveEnemy {
            enemy_type: EnemyType::Shambler,
            count: base_count,
            health_multiplier: health_mult,
            speed_multiplier: speed_mult,
            boss_trait: None,
        }];

        if wave_num >= 3 {
            enemies.push(WaveEnemy {
                enemy_type: EnemyType::Runner,
                count: (base_count / 2).max(2),
                health_multiplier: health_mult,
                speed_multiplier: speed_mult,
                boss_trait: None,
            });
        }

        if wave_num >= 6 {
            enemies.push(WaveEnemy {
                enemy_type: EnemyType::Brute,
                count: (base_count / 4).max(1),
                health_multiplier: health_mult,
                speed_multiplier: speed_mult,
                boss_trait: None,
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
