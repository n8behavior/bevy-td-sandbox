pub mod hud;
pub mod tower_menu;
pub mod game_over;

use bevy::prelude::*;
use bevy::ecs::message::MessageWriter;
use bevy::window::{MonitorSelection, WindowMode};
use crate::states::GameState;
use crate::common::constants::*;
use crate::economy::resources::PlayerScrap;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        app
            // Global (all states)
            .add_systems(Update, toggle_fullscreen)
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
                (
                    hud::update_hud,
                    tower_menu::highlight_selected_tower,
                    tower_menu::update_wave_preview,
                    handle_quit,
                )
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

/// ESC deselects tower first, then quits if nothing was selected.
/// Runs AFTER tower_selection so `selected` is already updated this frame.
fn handle_quit(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: MessageWriter<AppExit>,
    mut last_esc: Local<bool>,
) {
    if keyboard.just_pressed(KeyCode::Escape) {
        if *last_esc {
            // Second ESC in a row → quit
            exit.write(AppExit::Success);
        }
        *last_esc = true;
    } else if keyboard.get_just_pressed().count() > 0 {
        *last_esc = false;
    }
}

fn toggle_fullscreen(keyboard: Res<ButtonInput<KeyCode>>, mut windows: Query<&mut Window>) {
    if keyboard.just_pressed(KeyCode::F11) {
        let Ok(mut window) = windows.single_mut() else {
            return;
        };
        window.mode = match window.mode {
            WindowMode::BorderlessFullscreen(_) => {
                window.resolution.set(WINDOWED_WIDTH, WINDOWED_HEIGHT);
                WindowMode::Windowed
            }
            _ => WindowMode::BorderlessFullscreen(MonitorSelection::Current),
        };
    }
}
