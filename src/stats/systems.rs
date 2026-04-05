use bevy::prelude::*;

use crate::enemy::components::EnemyType;
use crate::enemy::systems::EnemyDied;

use super::resources::RunStats;

pub fn init_stats(mut commands: Commands, time: Res<Time>) {
    commands.insert_resource(RunStats {
        start_time: time.elapsed_secs(),
        survival_time_secs: 0.0,
        kills_shambler: 0,
        kills_runner: 0,
        kills_brute: 0,
        kills_boss: 0,
        scrap_collected: 0,
        scrap_spent: 0,
        towers_placed: 0,
        towers_sold: 0,
    });
}

pub fn track_survival_time(mut stats: ResMut<RunStats>, time: Res<Time>) {
    stats.survival_time_secs = time.elapsed_secs() - stats.start_time;
}

pub fn on_enemy_died_stats(trigger: On<EnemyDied>, mut stats: Option<ResMut<RunStats>>) {
    let Some(stats) = stats.as_mut() else {
        return;
    };
    match trigger.enemy_type {
        EnemyType::Shambler => stats.kills_shambler += 1,
        EnemyType::Runner => stats.kills_runner += 1,
        EnemyType::Brute => stats.kills_brute += 1,
        EnemyType::Boss => stats.kills_boss += 1,
    }
}
