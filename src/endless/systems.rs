use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::Rng;
use rand::prelude::IndexedRandom;

use crate::common::constants::GridConfig;
use crate::economy::components::ScrapDrop;
use crate::enemy::components::{Enemy, EnemyState, EnemyType, StolenScrap};
use crate::enemy::systems::spawn_enemy;
use crate::pile::resources::{EdgeCells, PileScrap, PileState};
use crate::pile::systems::nearest_pile_cell;
use crate::states::{GameState, PlayPhase};
use crate::wave::resources::BossTrait;

use super::resources::EndlessSpawner;

/// Skip the Building phase and go straight to Defending in Endless mode.
pub fn skip_building_phase(mut next_phase: ResMut<NextState<PlayPhase>>) {
    next_phase.set(PlayPhase::Defending);
}

/// Initialize the endless spawner resource when Defending phase begins.
pub fn init_endless(mut commands: Commands) {
    commands.insert_resource(EndlessSpawner {
        elapsed_time: 0.0,
        spawn_timer: Timer::from_seconds(1.5, TimerMode::Repeating),
        enemies_spawned: 0,
    });
}

/// Continuously spawn enemies with time-based difficulty scaling.
pub fn endless_spawn_enemies(
    mut commands: Commands,
    mut spawner: ResMut<EndlessSpawner>,
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

    spawner.elapsed_time += time.delta_secs();
    let elapsed = spawner.elapsed_time;

    // Tighten spawn interval over time: -0.02s per 10s elapsed, min 0.25s.
    let new_interval = (1.5 - (elapsed / 10.0) * 0.02).max(0.25);
    let current_duration = spawner.spawn_timer.duration().as_secs_f32();
    if (new_interval - current_duration).abs() > 0.01 {
        spawner
            .spawn_timer
            .set_duration(std::time::Duration::from_secs_f32(new_interval));
    }

    spawner.spawn_timer.tick(time.delta());
    if !spawner.spawn_timer.just_finished() {
        return;
    }

    let mut rng = rand::rng();

    // Pick enemy type based on elapsed time.
    let (enemy_type, boss_trait) = pick_enemy_type(elapsed, &mut rng);

    // Difficulty scaling: +15% HP per minute, +5% speed per minute.
    let minutes = elapsed / 60.0;
    let health_mult = 1.0 + minutes * 0.15;
    let speed_mult = 1.0 + minutes * 0.05;

    let spawn_pos = *edge_cells.0.choose(&mut rng).unwrap();
    let goal_pos = nearest_pile_cell(spawn_pos, &pile_state);

    spawn_enemy(
        &mut commands,
        enemy_type,
        spawn_pos,
        goal_pos,
        grid_entity,
        health_mult,
        speed_mult,
        &config,
        boss_trait,
    );

    spawner.enemies_spawned += 1;
}

/// Pick an enemy type based on elapsed time with escalating mix.
fn pick_enemy_type(elapsed: f32, rng: &mut impl Rng) -> (EnemyType, Option<BossTrait>) {
    let boss_traits = [
        BossTrait::Regeneration,
        BossTrait::Armor,
        BossTrait::Splitting,
    ];

    // Boss: appears after 300s, starts at 2% chance, increases 0.5% per minute beyond 300s.
    if elapsed >= 300.0 {
        let boss_chance = 0.02 + (elapsed - 300.0) / 60.0 * 0.005;
        if rng.random_range(0.0..1.0) < boss_chance {
            let trait_val = *boss_traits.choose(rng).unwrap();
            return (EnemyType::Boss, Some(trait_val));
        }
    }

    // Brute: appears after 150s, starts at 15% chance, caps at 25%.
    if elapsed >= 150.0 {
        let brute_chance = (0.15 + (elapsed - 150.0) / 600.0 * 0.10).min(0.25);
        if rng.random_range(0.0..1.0) < brute_chance {
            return (EnemyType::Brute, None);
        }
    }

    // Runner: appears after 60s, starts at 30% chance, caps at 40%.
    if elapsed >= 60.0 {
        let runner_chance = (0.30 + (elapsed - 60.0) / 300.0 * 0.10).min(0.40);
        if rng.random_range(0.0..1.0) < runner_chance {
            return (EnemyType::Runner, None);
        }
    }

    (EnemyType::Shambler, None)
}

/// Game over in endless mode: truly bankrupt with no recovery possible.
/// Unlike classic, there is no "spawn queue empty" condition — spawning is infinite,
/// so game over triggers when pile is empty AND no enemies/drops can restore scrap.
pub fn endless_check_game_over(
    pile_scrap: Res<PileScrap>,
    drops: Query<(), With<ScrapDrop>>,
    enemies: Query<(&EnemyState, Option<&StolenScrap>), With<Enemy>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if pile_scrap.amount > 0 {
        return;
    }
    if !drops.is_empty() {
        return;
    }
    for (state, stolen) in &enemies {
        if stolen.is_some_and(|s| s.0 > 0) {
            return;
        }
        if state.is_alive() {
            return;
        }
    }
    next_state.set(GameState::GameOver);
}
