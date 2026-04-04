use crate::states::GameState;
#[cfg(not(target_arch = "wasm32"))]
use bevy::ecs::message::MessageWriter;
use bevy::prelude::*;

pub fn setup_main_menu(mut commands: Commands) {
    commands.spawn((Camera2d, DespawnOnExit(GameState::MainMenu)));

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.08, 0.08, 0.06)),
            DespawnOnExit(GameState::MainMenu),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("SCRAP DEFENCE"),
                TextColor(Color::srgb(0.9, 0.7, 0.2)),
                TextFont {
                    font_size: 64.0,
                    ..default()
                },
            ));

            #[cfg(not(target_arch = "wasm32"))]
            let hint = "Press SPACE to start  |  ESC to quit";
            #[cfg(target_arch = "wasm32")]
            let hint = "Press SPACE to start";

            parent.spawn((
                Text::new(hint),
                TextColor(Color::srgb(0.6, 0.6, 0.5)),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
            ));
        });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_main_menu_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        next_state.set(GameState::Playing);
    }
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn handle_main_menu_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        next_state.set(GameState::Playing);
    }
}

pub fn setup_game_over(mut commands: Commands) {
    commands.spawn((Camera2d, DespawnOnExit(GameState::GameOver)));

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(20.0),
                ..default()
            },
            BackgroundColor(Color::srgb(0.1, 0.02, 0.02)),
            DespawnOnExit(GameState::GameOver),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("GAME OVER"),
                TextColor(Color::srgb(0.9, 0.2, 0.2)),
                TextFont {
                    font_size: 64.0,
                    ..default()
                },
            ));

            #[cfg(not(target_arch = "wasm32"))]
            let hint = "SPACE: restart  |  ESC: quit";
            #[cfg(target_arch = "wasm32")]
            let hint = "SPACE: restart";

            parent.spawn((
                Text::new(hint),
                TextColor(Color::srgb(0.6, 0.6, 0.5)),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
            ));
        });
}

#[cfg(not(target_arch = "wasm32"))]
pub fn handle_game_over_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut exit: MessageWriter<AppExit>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        next_state.set(GameState::MainMenu);
    }
    if keyboard.just_pressed(KeyCode::Escape) {
        exit.write(AppExit::Success);
    }
}

#[cfg(target_arch = "wasm32")]
pub fn handle_game_over_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        next_state.set(GameState::MainMenu);
    }
}
