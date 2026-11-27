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
                    font_size: FontSize::Px(64.0),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("[SPACE] Classic Mode  |  [E] Endless Mode"),
                TextColor(Color::srgb(0.6, 0.6, 0.5)),
                TextFont {
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
            ));

            #[cfg(not(target_arch = "wasm32"))]
            parent.spawn((
                Text::new("ESC ESC: Quit"),
                TextColor(Color::srgb(0.4, 0.4, 0.35)),
                TextFont {
                    font_size: FontSize::Px(16.0),
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

/// Whether the run that just ended was won: only a Classic run can be,
/// by beating its last wave. Endless runs always end in a loss.
pub(crate) fn run_won(game_mode: GameMode, wave_mgr: Option<&WaveManager>) -> bool {
    game_mode == GameMode::Classic && wave_mgr.is_some_and(WaveManager::all_waves_cleared)
}

/// End-of-run screen, for both outcomes. A win and a loss share the layout
/// and the stats; only the title and colors differ.
pub fn setup_game_over(
    mut commands: Commands,
    stats: Option<Res<RunStats>>,
    game_mode: Res<GameMode>,
    wave_mgr: Option<Res<WaveManager>>,
) {
    commands.spawn((Camera2d, DespawnOnExit(GameState::GameOver)));

    let (title, title_color, background) = if run_won(*game_mode, wave_mgr.as_deref()) {
        (
            "YOU SURVIVED",
            Color::srgb(0.45, 0.9, 0.35),
            Color::srgb(0.02, 0.08, 0.03),
        )
    } else {
        (
            "GAME OVER",
            Color::srgb(0.9, 0.2, 0.2),
            Color::srgb(0.1, 0.02, 0.02),
        )
    };

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
            BackgroundColor(background),
            DespawnOnExit(GameState::GameOver),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(title),
                TextColor(title_color),
                TextFont {
                    font_size: FontSize::Px(64.0),
                    ..default()
                },
            ));

            let stat_color = Color::srgb(0.75, 0.7, 0.55);
            let stat_font = TextFont {
                font_size: FontSize::Px(18.0),
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
                                font_size: FontSize::Px(28.0),
                                ..default()
                            },
                        ));
                    }
                    GameMode::Classic => {
                        let (survived, total) = wave_mgr
                            .as_ref()
                            .map_or((0, 0), |w| (w.current_wave, w.waves.len()));
                        parent.spawn((
                            Text::new(format!("Waves Survived: {survived} / {total}")),
                            TextColor(Color::srgb(0.9, 0.6, 0.2)),
                            TextFont {
                                font_size: FontSize::Px(28.0),
                                ..default()
                            },
                        ));
                    }
                }

                // Kill breakdown — iterate the per-blueprint counter so
                // adding a new enemy type adds a row automatically.
                let mut entries: Vec<(&&'static str, &u32)> = stats.kills.iter().collect();
                entries.sort_by_key(|(name, _)| *name);
                let breakdown = entries
                    .iter()
                    .map(|(name, count)| format!("{name}: {count}"))
                    .collect::<Vec<_>>()
                    .join("  ");
                let total_line = if breakdown.is_empty() {
                    format!("Kills: {}", stats.total_kills())
                } else {
                    format!("Kills: {}  ({breakdown})", stats.total_kills())
                };
                parent.spawn((
                    Text::new(total_line),
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
                    font_size: FontSize::Px(24.0),
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

    fn wave_manager_at(current_wave: u32) -> WaveManager {
        WaveManager {
            current_wave,
            waves: crate::wave::systems::generate_waves(),
            ..crate::test_helpers::test_wave_manager()
        }
    }

    #[test]
    fn run_won_when_classic_clears_every_wave() {
        let total = wave_manager_at(0).waves.len() as u32;
        assert!(run_won(GameMode::Classic, Some(&wave_manager_at(total))));
    }

    #[test]
    fn run_lost_when_classic_ends_before_the_last_wave() {
        let total = wave_manager_at(0).waves.len() as u32;
        assert!(!run_won(
            GameMode::Classic,
            Some(&wave_manager_at(total - 1))
        ));
    }

    #[test]
    fn endless_run_is_never_won() {
        // A finished Classic run leaves its WaveManager behind; a later
        // Endless run must not read it as a win.
        let total = wave_manager_at(0).waves.len() as u32;
        assert!(!run_won(GameMode::Endless, Some(&wave_manager_at(total))));
    }

    #[test]
    fn run_lost_without_wave_manager() {
        assert!(!run_won(GameMode::Classic, None));
    }

    #[test]
    fn game_over_no_key_is_noop() {
        let keys = ButtonInput::<KeyCode>::default();
        let mut next_state = NextState::<GameState>::default();

        apply_game_over_keys(&keys, &mut next_state);

        assert!(matches!(next_state, NextState::Unchanged));
    }
}
