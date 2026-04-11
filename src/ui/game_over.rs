use crate::states::{GameMode, GameState};
use crate::stats::resources::RunStats;
use crate::wave::resources::WaveManager;
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

            parent.spawn((
                Text::new("[SPACE] Classic Mode  |  [E] Endless Mode"),
                TextColor(Color::srgb(0.6, 0.6, 0.5)),
                TextFont {
                    font_size: 24.0,
                    ..default()
                },
            ));

            #[cfg(not(target_arch = "wasm32"))]
            parent.spawn((
                Text::new("ESC ESC: Quit"),
                TextColor(Color::srgb(0.4, 0.4, 0.35)),
                TextFont {
                    font_size: 16.0,
                    ..default()
                },
            ));
        });
}

/// Shared main-menu key handling: Space → Classic, E → Endless.
fn apply_main_menu_keys(
    keyboard: &ButtonInput<KeyCode>,
    next_state: &mut NextState<GameState>,
    game_mode: &mut GameMode,
) {
    if keyboard.just_pressed(KeyCode::Space) {
        *game_mode = GameMode::Classic;
        next_state.set(GameState::Playing);
    }
    if keyboard.just_pressed(KeyCode::KeyE) {
        *game_mode = GameMode::Endless;
        next_state.set(GameState::Playing);
    }
}

pub fn handle_main_menu_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
    mut game_mode: ResMut<GameMode>,
) {
    apply_main_menu_keys(&keyboard, &mut next_state, &mut game_mode);
}

pub fn setup_game_over(
    mut commands: Commands,
    stats: Option<Res<RunStats>>,
    game_mode: Res<GameMode>,
    wave_mgr: Option<Res<WaveManager>>,
) {
    commands.spawn((Camera2d, DespawnOnExit(GameState::GameOver)));

    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(12.0),
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

            let stat_color = Color::srgb(0.75, 0.7, 0.55);
            let stat_font = TextFont {
                font_size: 18.0,
                ..default()
            };

            if let Some(stats) = &stats {
                // Mode-specific headline.
                match *game_mode {
                    GameMode::Endless => {
                        let mins = (stats.survival_time_secs / 60.0) as u32;
                        let secs = (stats.survival_time_secs % 60.0) as u32;
                        parent.spawn((
                            Text::new(format!("Survival Time: {:02}:{:02}", mins, secs)),
                            TextColor(Color::srgb(0.9, 0.6, 0.2)),
                            TextFont {
                                font_size: 28.0,
                                ..default()
                            },
                        ));
                    }
                    GameMode::Classic => {
                        let waves = wave_mgr.as_ref().map_or(0, |w| w.current_wave);
                        parent.spawn((
                            Text::new(format!("Waves Survived: {} / 20", waves)),
                            TextColor(Color::srgb(0.9, 0.6, 0.2)),
                            TextFont {
                                font_size: 28.0,
                                ..default()
                            },
                        ));
                    }
                }

                // Kill breakdown.
                parent.spawn((
                    Text::new(format!(
                        "Kills: {}  (Shambler: {}  Runner: {}  Brute: {}  Boss: {})",
                        stats.total_kills(),
                        stats.kills_shambler,
                        stats.kills_runner,
                        stats.kills_brute,
                        stats.kills_boss,
                    )),
                    TextColor(stat_color),
                    stat_font.clone(),
                ));

                // Economy.
                parent.spawn((
                    Text::new(format!(
                        "Scrap Collected: {}  |  Scrap Spent: {}",
                        stats.scrap_collected, stats.scrap_spent,
                    )),
                    TextColor(stat_color),
                    stat_font.clone(),
                ));

                // Towers.
                parent.spawn((
                    Text::new(format!(
                        "Towers Placed: {}  |  Towers Sold: {}",
                        stats.towers_placed, stats.towers_sold,
                    )),
                    TextColor(stat_color),
                    stat_font,
                ));
            }

            #[cfg(not(target_arch = "wasm32"))]
            let hint = "SPACE: restart  |  ESC ESC: Quit";
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

/// Shared game-over key handling: Space → MainMenu.
fn apply_game_over_keys(keyboard: &ButtonInput<KeyCode>, next_state: &mut NextState<GameState>) {
    if keyboard.just_pressed(KeyCode::Space) {
        next_state.set(GameState::MainMenu);
    }
}

pub fn handle_game_over_input(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    apply_game_over_keys(&keyboard, &mut next_state);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_menu_space_selects_classic() {
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::Space);
        let mut next_state = NextState::<GameState>::default();
        let mut game_mode = GameMode::default();

        apply_main_menu_keys(&keys, &mut next_state, &mut game_mode);

        assert_eq!(game_mode, GameMode::Classic);
        assert!(matches!(next_state, NextState::Pending(GameState::Playing)));
    }

    #[test]
    fn main_menu_e_selects_endless() {
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::KeyE);
        let mut next_state = NextState::<GameState>::default();
        let mut game_mode = GameMode::default();

        apply_main_menu_keys(&keys, &mut next_state, &mut game_mode);

        assert_eq!(game_mode, GameMode::Endless);
        assert!(matches!(next_state, NextState::Pending(GameState::Playing)));
    }

    #[test]
    fn main_menu_no_key_is_noop() {
        let keys = ButtonInput::<KeyCode>::default();
        let mut next_state = NextState::<GameState>::default();
        let mut game_mode = GameMode::default();

        apply_main_menu_keys(&keys, &mut next_state, &mut game_mode);

        assert_eq!(game_mode, GameMode::Classic); // unchanged default
        assert!(matches!(next_state, NextState::Unchanged));
    }

    #[test]
    fn game_over_space_returns_to_menu() {
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::Space);
        let mut next_state = NextState::<GameState>::default();

        apply_game_over_keys(&keys, &mut next_state);

        assert!(matches!(
            next_state,
            NextState::Pending(GameState::MainMenu)
        ));
    }

    #[test]
    fn game_over_no_key_is_noop() {
        let keys = ButtonInput::<KeyCode>::default();
        let mut next_state = NextState::<GameState>::default();

        apply_game_over_keys(&keys, &mut next_state);

        assert!(matches!(next_state, NextState::Unchanged));
    }
}
