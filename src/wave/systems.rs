use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::prelude::IndexedRandom;
use rand::seq::SliceRandom;

use crate::audio::resources::SoundAssets;
use crate::audio::systems::play_sound;
use crate::common::constants::GridConfig;
use crate::economy::components::ScrapDrop;
use crate::enemy::components::{Dead, Enemy, EnemyType};
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
    sounds: Res<SoundAssets>,
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

    if entry.boss_trait.is_some() {
        play_sound(&mut commands, &sounds.boss_spawn, 0.6);
    }

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

pub fn play_wave_start_sound(mut commands: Commands, sounds: Res<SoundAssets>) {
    play_sound(&mut commands, &sounds.wave_start, 0.5);
}

pub fn handle_start_wave_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_phase: ResMut<NextState<PlayPhase>>,
) {
    if keyboard.just_pressed(KeyCode::Enter) {
        next_phase.set(PlayPhase::Defending);
    }
}

/// Game over when truly bankrupt: pile empty and no scrap on the ground.
/// Stolen scrap is already subtracted from the pile at steal time; if the
/// thief is killed the loot reappears as a ScrapDrop (caught by the drops
/// check). Alive enemies are irrelevant — if there's zero scrap anywhere
/// in the economy, recovery is impossible.
pub fn check_game_over(
    mut commands: Commands,
    pile_scrap: Res<PileScrap>,
    drops: Query<(), With<ScrapDrop>>,
    mut next_state: ResMut<NextState<GameState>>,
    sounds: Res<SoundAssets>,
) {
    if pile_scrap.amount > 0 {
        return;
    }
    if !drops.is_empty() {
        return;
    }
    play_sound(&mut commands, &sounds.game_over, 0.6);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_waves_produces_twenty() {
        let waves = generate_waves();
        assert_eq!(waves.len(), 20);
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
                    wave.enemies[0].enemy_type,
                    EnemyType::Boss,
                    "wave {wave_num} should be a boss wave"
                );
                assert_eq!(wave.enemies[0].count, 1, "boss wave should have 1 boss");
                assert!(
                    wave.enemies[0].boss_trait.is_some(),
                    "boss should have a trait"
                );
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
                    wave.enemies
                        .iter()
                        .any(|e| e.enemy_type == EnemyType::Shambler),
                    "wave {wave_num} should have shamblers"
                );
            }
        }
    }

    #[test]
    fn runners_appear_from_wave_3() {
        let waves = generate_waves();
        // Wave 2 (index 1) should NOT have runners (unless it's a boss wave)
        let wave2 = &waves[1];
        assert!(
            !wave2
                .enemies
                .iter()
                .any(|e| e.enemy_type == EnemyType::Runner),
            "wave 2 should not have runners"
        );
        // Wave 3 (index 2) should have runners
        let wave3 = &waves[2];
        assert!(
            wave3
                .enemies
                .iter()
                .any(|e| e.enemy_type == EnemyType::Runner),
            "wave 3 should have runners"
        );
    }

    #[test]
    fn brutes_appear_from_wave_6() {
        let waves = generate_waves();
        // Wave 4 (index 3) should NOT have brutes
        let wave4 = &waves[3];
        assert!(
            !wave4
                .enemies
                .iter()
                .any(|e| e.enemy_type == EnemyType::Brute),
            "wave 4 should not have brutes"
        );
        // Wave 6 (index 5) should have brutes
        let wave6 = &waves[5];
        assert!(
            wave6
                .enemies
                .iter()
                .any(|e| e.enemy_type == EnemyType::Brute),
            "wave 6 should have brutes"
        );
    }

    #[test]
    fn health_multiplier_scales_correctly() {
        let waves = generate_waves();
        for (i, wave) in waves.iter().enumerate() {
            let expected = 1.0 + (i as f32 * 0.15);
            for enemy in &wave.enemies {
                assert!(
                    (enemy.health_multiplier - expected).abs() < 0.001,
                    "wave {} health mult: got {}, expected {}",
                    i + 1,
                    enemy.health_multiplier,
                    expected
                );
            }
        }
    }

    #[test]
    fn speed_multiplier_scales_correctly() {
        let waves = generate_waves();
        for (i, wave) in waves.iter().enumerate() {
            let expected = 1.0 + (i as f32 * 0.05);
            for enemy in &wave.enemies {
                assert!(
                    (enemy.speed_multiplier - expected).abs() < 0.001,
                    "wave {} speed mult: got {}, expected {}",
                    i + 1,
                    enemy.speed_multiplier,
                    expected
                );
            }
        }
    }

    #[test]
    fn spawn_interval_decreases_and_clamps() {
        let waves = generate_waves();
        // First non-boss wave should have interval 1.5
        assert!((waves[0].spawn_interval - 1.5).abs() < 0.001);
        // Last non-boss wave's interval should be clamped at 0.3
        let last_non_boss = waves
            .iter()
            .enumerate()
            .rfind(|(i, _)| (i + 1) % 5 != 0)
            .unwrap()
            .1;
        assert!(
            last_non_boss.spawn_interval >= 0.3 - 0.001,
            "spawn interval should be clamped at 0.3, got {}",
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
