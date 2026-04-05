pub mod resources;
pub mod systems;

use crate::states::{GameMode, GameState, PlayPhase};
use bevy::prelude::*;

fn is_classic(mode: Res<GameMode>) -> bool {
    *mode == GameMode::Classic
}

pub struct WavePlugin;

impl Plugin for WavePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Playing),
            init_wave_manager.run_if(is_classic),
        )
        .add_systems(
            OnEnter(PlayPhase::Defending),
            (systems::start_wave, systems::play_wave_start_sound).run_if(is_classic),
        )
        .add_systems(
            FixedUpdate,
            systems::spawn_enemies
                .run_if(in_state(GameState::Playing))
                .run_if(in_state(PlayPhase::Defending))
                .run_if(is_classic),
        )
        .add_systems(
            Update,
            (
                (systems::check_game_over, systems::check_wave_complete)
                    .chain()
                    .run_if(in_state(PlayPhase::Defending)),
                systems::handle_start_wave_input.run_if(in_state(PlayPhase::Building)),
            )
                .run_if(in_state(GameState::Playing))
                .run_if(is_classic),
        );
    }
}

fn init_wave_manager(mut commands: Commands) {
    commands.insert_resource(resources::WaveManager {
        current_wave: 0,
        waves: systems::generate_waves(),
        spawn_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        enemies_remaining: 0,
        spawn_queue: Vec::new(),
    });
}
