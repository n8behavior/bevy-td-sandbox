use bevy::prelude::*;

use crate::enemy::systems::EnemyDied;

use super::resources::RunStats;

pub fn init_stats(mut commands: Commands, time: Res<Time>) {
    commands.insert_resource(RunStats::new(time.elapsed_secs()));
}

pub fn track_survival_time(mut stats: ResMut<RunStats>, time: Res<Time>) {
    stats.survival_time_secs = time.elapsed_secs() - stats.start_time;
}

pub fn on_enemy_died_stats(trigger: On<EnemyDied>, mut stats: ResMut<RunStats>) {
    stats.record_kill(trigger.enemy_type);
}
