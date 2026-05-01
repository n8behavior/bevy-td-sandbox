use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::prelude::IndexedRandom;
use rand::seq::SliceRandom;

use crate::audio::{GameSound, PlaySound};
use crate::common::constants::GridConfig;
use crate::economy::components::ScrapDrop;
use crate::enemy::components::{Enemy, EnemyRegistry};
use crate::enemy::spawn::spawn_from_blueprint;
use crate::pile::resources::{EdgeCells, PileScrap, PileState};
use crate::pile::systems::nearest_pile_cell;
use crate::states::{GameState, PlayPhase};

use super::resources::*;

const TOTAL_WAVES: u32 = 20;
const BOSS_WAVE_INTERVAL: u32 = 5;
const RUNNER_UNLOCK_WAVE: u32 = 3;
const BRUTE_UNLOCK_WAVE: u32 = 6;
const BASE_SPAWN_INTERVAL: f32 = 1.5;
const MIN_SPAWN_INTERVAL: f32 = 0.3;
const SPAWN_INTERVAL_DECREASE: f32 = 0.05;

pub fn start_wave(mut wave_mgr: ResMut<WaveManager>) {
    let wave_idx = wave_mgr.current_wave as usize;
    if wave_idx >= wave_mgr.waves.len() {
        return;
    }

    let wave = &wave_mgr.waves[wave_idx];
    let mut queue: Vec<SpawnEntry> = wave
        .enemies
        .iter()
        .flat_map(|we| {
            (0..we.count).map(move |_| SpawnEntry {
                enemy_blueprint: we.enemy_blueprint,
            })
        })
        .collect();

    queue.shuffle(&mut rand::rng());

    let interval = wave.spawn_interval;

    wave_mgr.spawn_queue = queue;
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
    registry: Res<EnemyRegistry>,
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

    let mut rng = rand::rng();
    let spawn_pos = *edge_cells.0.choose(&mut rng).unwrap();
    let goal_pos = nearest_pile_cell(spawn_pos, &pile_state);

    let Some(blueprint) = registry.lookup(entry.enemy_blueprint) else {
        warn!(
            "spawn_enemies: blueprint '{}' not found in registry",
            entry.enemy_blueprint
        );
        return;
    };

    if entry.enemy_blueprint == "Boss" {
        commands.trigger(PlaySound(GameSound::WaveBossSpawn));
    }

    let wave = wave_mgr.current_wave + 1;
    spawn_from_blueprint(
        &mut commands,
        blueprint,
        spawn_pos,
        goal_pos,
        grid_entity,
        &config,
        wave,
    );
}

#[derive(Event)]
pub struct WaveComplete;

/// Whether the wave still has unresolved activity: enemies queued, alive, or drops pending.
fn is_wave_active(queue_empty: bool, enemies_empty: bool, drops_empty: bool) -> bool {
    !queue_empty || !enemies_empty || !drops_empty
}

/// When the wave is fully resolved (all enemies dead/escaped, all drops
/// settled), trigger the `WaveComplete` event.
pub fn check_wave_complete(
    mut commands: Commands,
    wave_mgr: Res<WaveManager>,
    enemies: Query<(), With<Enemy>>,
    drops: Query<(), With<ScrapDrop>>,
) {
    if is_wave_active(
        wave_mgr.spawn_queue.is_empty(),
        enemies.is_empty(),
        drops.is_empty(),
    ) {
        return;
    }
    commands.trigger(WaveComplete);
}

/// Observer: handle wave completion — game over vs next wave.
pub fn on_wave_complete(
    _trigger: On<WaveComplete>,
    mut commands: Commands,
    mut wave_mgr: ResMut<WaveManager>,
    pile_scrap: Res<PileScrap>,
    mut next_phase: ResMut<NextState<PlayPhase>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if pile_scrap.amount == 0 {
        commands.trigger(PlaySound(GameSound::GameOver));
        next_state.set(GameState::GameOver);
    } else {
        wave_mgr.current_wave += 1;
        next_phase.set(PlayPhase::Building);
    }
}

pub fn play_wave_start_sound(mut commands: Commands) {
    commands.trigger(PlaySound(GameSound::WaveStart));
}

pub fn handle_start_wave_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_phase: ResMut<NextState<PlayPhase>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        next_phase.set(PlayPhase::Defending);
    }
}

pub fn generate_waves() -> Vec<WaveConfig> {
    let mut waves = Vec::new();

    for i in 0..TOTAL_WAVES {
        let wave_num = i + 1;

        if wave_num % BOSS_WAVE_INTERVAL == 0 {
            waves.push(WaveConfig {
                enemies: vec![WaveEnemy {
                    enemy_blueprint: "Boss",
                    count: 1,
                }],
                spawn_interval: 1.0,
            });
            continue;
        }

        let base_count = 5 + i * 2;

        let mut enemies = vec![WaveEnemy {
            enemy_blueprint: "Shambler",
            count: base_count,
        }];

        if wave_num >= RUNNER_UNLOCK_WAVE {
            enemies.push(WaveEnemy {
                enemy_blueprint: "Runner",
                count: (base_count / 2).max(2),
            });
        }

        if wave_num >= BRUTE_UNLOCK_WAVE {
            enemies.push(WaveEnemy {
                enemy_blueprint: "Brute",
                count: (base_count / 4).max(1),
            });
        }

        let spawn_interval =
            (BASE_SPAWN_INTERVAL - i as f32 * SPAWN_INTERVAL_DECREASE).max(MIN_SPAWN_INTERVAL);

        waves.push(WaveConfig {
            enemies,
            spawn_interval,
        });
    }

    waves
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave_active_with_nonempty_queue() {
        assert!(is_wave_active(false, true, true));
    }

    #[test]
    fn wave_active_with_alive_enemies() {
        assert!(is_wave_active(true, false, true));
    }

    #[test]
    fn wave_active_with_pending_drops() {
        assert!(is_wave_active(true, true, false));
    }

    #[test]
    fn wave_inactive_when_all_empty() {
        assert!(!is_wave_active(true, true, true));
    }

    #[test]
    fn generate_waves_produces_twenty() {
        let waves = generate_waves();
        assert_eq!(waves.len(), TOTAL_WAVES as usize);
    }

    #[test]
    fn boss_wave_every_fifth() {
        let waves = generate_waves();
        for (i, wave) in waves.iter().enumerate() {
            let wave_num = i + 1;
            if wave_num % 5 == 0 {
                assert_eq!(
                    wave.enemies.len(),
                    1,
                    "wave {wave_num} should have 1 enemy group"
                );
                assert_eq!(
                    wave.enemies[0].enemy_blueprint, "Boss",
                    "wave {wave_num} should be a boss wave"
                );
                assert_eq!(wave.enemies[0].count, 1, "boss wave should have 1 boss");
            }
        }
    }

    #[test]
    fn non_boss_waves_have_shamblers() {
        let waves = generate_waves();
        for (i, wave) in waves.iter().enumerate() {
            let wave_num = i + 1;
            if wave_num % 5 != 0 {
                assert!(
                    wave.enemies.iter().any(|e| e.enemy_blueprint == "Shambler"),
                    "wave {wave_num} should have shamblers"
                );
            }
        }
    }

    #[test]
    fn runners_appear_from_wave_3() {
        let waves = generate_waves();
        let wave2 = &waves[1];
        assert!(
            !wave2.enemies.iter().any(|e| e.enemy_blueprint == "Runner"),
            "wave 2 should not have runners"
        );
        let wave3 = &waves[2];
        assert!(
            wave3.enemies.iter().any(|e| e.enemy_blueprint == "Runner"),
            "wave 3 should have runners"
        );
    }

    #[test]
    fn brutes_appear_from_wave_6() {
        let waves = generate_waves();
        let wave4 = &waves[3];
        assert!(
            !wave4.enemies.iter().any(|e| e.enemy_blueprint == "Brute"),
            "wave 4 should not have brutes"
        );
        let wave6 = &waves[5];
        assert!(
            wave6.enemies.iter().any(|e| e.enemy_blueprint == "Brute"),
            "wave 6 should have brutes"
        );
    }

    #[test]
    fn spawn_interval_decreases_and_clamps() {
        let waves = generate_waves();
        assert!((waves[0].spawn_interval - BASE_SPAWN_INTERVAL).abs() < 0.001);
        let last_non_boss = waves
            .iter()
            .enumerate()
            .rfind(|(i, _)| (i + 1) % 5 != 0)
            .unwrap()
            .1;
        assert!(
            last_non_boss.spawn_interval >= MIN_SPAWN_INTERVAL - 0.001,
            "spawn interval should be clamped at {MIN_SPAWN_INTERVAL}, got {}",
            last_non_boss.spawn_interval
        );
    }

    #[test]
    fn all_waves_have_nonzero_enemies() {
        let waves = generate_waves();
        for (i, wave) in waves.iter().enumerate() {
            let total: u32 = wave.enemies.iter().map(|e| e.count).sum();
            assert!(total > 0, "wave {} has no enemies", i + 1);
        }
    }
}
