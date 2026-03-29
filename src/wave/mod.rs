pub mod resources;
pub mod systems;

use bevy::prelude::*;
use crate::states::{GameState, PlayPhase};

pub struct WavePlugin;

impl Plugin for WavePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_systems(
                OnEnter(GameState::Playing),
                init_wave_manager,
            )
            .add_systems(OnEnter(PlayPhase::Defending), systems::start_wave)
            .add_systems(
                FixedUpdate,
                systems::spawn_enemies
                    .run_if(in_state(GameState::Playing))
                    .run_if(in_state(PlayPhase::Defending)),
            )
            .add_systems(
                Update,
                (
                    systems::check_wave_complete
                        .run_if(in_state(PlayPhase::Defending)),
                    systems::handle_start_wave_input
                        .run_if(in_state(PlayPhase::Building)),
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn init_wave_manager(mut commands: Commands) {
    commands.insert_resource(resources::WaveManager {
        current_wave: 0,
        waves: systems::generate_waves(),
        spawn_timer: Timer::from_seconds(1.0, TimerMode::Repeating),
        enemies_remaining: 0,
        enemies_spawned: 0,
    });
}
