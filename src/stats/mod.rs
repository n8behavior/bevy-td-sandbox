pub mod resources;
pub mod systems;

use crate::states::GameState;
use bevy::prelude::*;

pub struct StatsPlugin;

impl Plugin for StatsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Playing), systems::init_stats)
            .add_systems(
                Update,
                systems::track_survival_time.run_if(in_state(GameState::Playing)),
            )
            .add_observer(systems::on_enemy_died_stats);
    }
}
