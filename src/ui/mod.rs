pub mod hud;
pub mod tower_menu;
pub mod game_over;

use bevy::prelude::*;
use crate::states::GameState;
use crate::common::constants::*;
use crate::economy::resources::PlayerScrap;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
            // Main menu
            .add_systems(OnEnter(GameState::MainMenu), game_over::setup_main_menu)
            .add_systems(
                Update,
                game_over::handle_main_menu_input.run_if(in_state(GameState::MainMenu)),
            )
            // Playing - init resources, HUD, tower palette
            .add_systems(
                OnEnter(GameState::Playing),
                (init_resources, hud::setup_hud, tower_menu::setup_tower_palette),
            )
            .add_systems(
                Update,
                (hud::update_hud, tower_menu::highlight_selected_tower)
                    .run_if(in_state(GameState::Playing)),
            )
            // Game over
            .add_systems(OnEnter(GameState::GameOver), game_over::setup_game_over)
            .add_systems(
                Update,
                game_over::handle_game_over_input.run_if(in_state(GameState::GameOver)),
            );
    }
}

fn init_resources(mut commands: Commands) {
    commands.insert_resource(PlayerScrap(STARTING_SCRAP));
    commands.insert_resource(hud::PlayerLives(STARTING_LIVES));
}
