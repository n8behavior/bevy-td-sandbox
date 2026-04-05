pub mod resources;
pub mod systems;

use crate::states::{GameMode, GameState, PlayPhase};
use bevy::prelude::*;

fn is_endless(mode: Res<GameMode>) -> bool {
    *mode == GameMode::Endless
}

pub struct EndlessPlugin;

impl Plugin for EndlessPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Playing),
            systems::skip_building_phase.run_if(is_endless),
        )
        .add_systems(
            OnEnter(PlayPhase::Defending),
            systems::init_endless.run_if(is_endless),
        )
        .add_systems(
            FixedUpdate,
            systems::endless_spawn_enemies
                .run_if(in_state(GameState::Playing))
                .run_if(in_state(PlayPhase::Defending))
                .run_if(is_endless),
        )
        .add_systems(
            Update,
            systems::endless_check_game_over
                .run_if(in_state(GameState::Playing))
                .run_if(in_state(PlayPhase::Defending))
                .run_if(is_endless),
        );
    }
}
