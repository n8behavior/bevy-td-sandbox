use bevy::prelude::*;

use crate::enemy::components::EnemyName;
use crate::enemy::events::EnemyDied;

use super::resources::RunStats;

pub fn init_stats(mut commands: Commands, time: Res<Time>) {
    commands.insert_resource(RunStats::new(time.elapsed_secs()));
}

pub fn track_survival_time(mut stats: ResMut<RunStats>, time: Res<Time>) {
    stats.survival_time_secs = time.elapsed_secs() - stats.start_time;
}

/// Record a kill keyed by the dying enemy's blueprint name.
pub fn on_enemy_died_stats(
    trigger: On<EnemyDied>,
    names: Query<&EnemyName>,
    mut stats: ResMut<RunStats>,
) {
    if let Ok(name) = names.get(trigger.entity) {
        stats.record_kill(name.0);
    }
}
