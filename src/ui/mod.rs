pub mod hud;
pub mod tower_menu;
pub mod game_over;

use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::ecs::message::MessageWriter;
#[cfg(not(target_arch = "wasm32"))]
use bevy::window::{MonitorSelection, WindowMode};
use crate::states::GameState;
#[cfg(not(target_arch = "wasm32"))]
use crate::common::constants::*;

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, (toggle_fullscreen, handle_quit));

        app
            // Main menu
            .add_systems(OnEnter(GameState::MainMenu), game_over::setup_main_menu)
            .add_systems(
                Update,
                game_over::handle_main_menu_input.run_if(in_state(GameState::MainMenu)),
            )
            // Playing - HUD, tower palette
            .add_systems(
                OnEnter(GameState::Playing),
                (hud::setup_hud, tower_menu::setup_tower_palette),
            )
            .add_systems(
                Update,
                (
                    hud::update_hud,
                    tower_menu::highlight_selected_tower,
                    tower_menu::update_wave_preview,
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

#[cfg(not(target_arch = "wasm32"))]
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

#[cfg(not(target_arch = "wasm32"))]
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
