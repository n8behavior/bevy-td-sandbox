use bevy::prelude::*;
use crate::states::GameState;

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
            parent.spawn((
                Text::new("Press SPACE to start"),
                TextColor(Color::srgb(0.6, 0.6, 0.5)),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
            ));
        });
}

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
            parent.spawn((
                Text::new("Press SPACE to restart"),
                TextColor(Color::srgb(0.6, 0.6, 0.5)),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
            ));
        });
}

pub fn handle_game_over_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        next_state.set(GameState::MainMenu);
    }
}
