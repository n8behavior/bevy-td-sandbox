pub mod game_over;
pub mod hud;
pub mod tower_menu;

#[cfg(not(target_arch = "wasm32"))]
use crate::common::constants::*;
use crate::states::GameState;
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::window::{MonitorSelection, WindowMode};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut App) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            app.init_resource::<QuitConfirmation>();
            app.add_systems(Startup, setup_quit_banner);
            app.add_systems(Update, (toggle_fullscreen, handle_quit));
        }

        app
            // HUD toggle (event-driven state machine)
            .init_resource::<hud::HudState>()
            .add_observer(hud::on_toggle_hud)
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
                    hud::handle_hud_toggle_input,
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

// ---------------------------------------------------------------------------
// Quit confirmation (desktop only)
// ---------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
const QUIT_CONFIRM_SECS: f32 = 2.0;

/// Tracks the timed window for double-ESC quit confirmation.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Resource, Default)]
struct QuitConfirmation {
    timer: Option<Timer>,
}

/// Marker for the "Press ESC again to quit" banner (root UI node).
#[cfg(not(target_arch = "wasm32"))]
#[derive(Component)]
struct QuitBanner;

/// Result of evaluating quit input against the confirmation timer.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum QuitAction {
    /// No action needed.
    None,
    /// First ESC pressed — confirmation window started.
    Armed,
    /// Second ESC pressed within window — quit the app.
    Quit,
}

/// Evaluate ESC input against the confirmation timer.
///
/// - No timer + ESC → `Armed` (starts 2s window)
/// - Timer active + ESC → `Quit`
/// - Timer expires → cleared to `None`
/// - No ESC → `None`
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn quit_logic(
    esc_pressed: bool,
    timer: &mut Option<Timer>,
    dt: std::time::Duration,
) -> QuitAction {
    if let Some(t) = timer.as_mut() {
        t.tick(dt);
        if t.is_finished() {
            *timer = None;
        }
    }

    if esc_pressed {
        if timer.is_some() {
            *timer = None;
            return QuitAction::Quit;
        }
        *timer = Some(Timer::from_seconds(QUIT_CONFIRM_SECS, TimerMode::Once));
        return QuitAction::Armed;
    }

    QuitAction::None
}

#[cfg(not(target_arch = "wasm32"))]
fn handle_quit(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut exit: bevy::ecs::message::MessageWriter<AppExit>,
    mut confirm: ResMut<QuitConfirmation>,
    time: Res<Time>,
    mut banner: Query<&mut Visibility, With<QuitBanner>>,
) {
    let esc = keyboard.just_pressed(KeyCode::Escape);
    let action = quit_logic(esc, &mut confirm.timer, time.delta());

    if action == QuitAction::Quit {
        exit.write(AppExit::Success);
    }

    let should_show = confirm.timer.is_some();
    for mut vis in &mut banner {
        *vis = if should_show {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// Spawns the quit-confirmation banner as a root UI node (no `DespawnOnExit`).
#[cfg(not(target_arch = "wasm32"))]
fn setup_quit_banner(mut commands: Commands) {
    commands
        .spawn((
            QuitBanner,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(60.0),
                width: Val::Percent(100.0),
                justify_content: JustifyContent::Center,
                ..default()
            },
            GlobalZIndex(100),
            Visibility::Hidden,
        ))
        .with_children(|parent| {
            parent
                .spawn((
                    Node {
                        padding: UiRect::new(
                            Val::Px(16.0),
                            Val::Px(16.0),
                            Val::Px(8.0),
                            Val::Px(8.0),
                        ),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.12, 0.1, 0.08, 0.85)),
                ))
                .with_children(|inner| {
                    inner.spawn((
                        Text::new("Press ESC again to quit"),
                        TextColor(Color::srgb(0.9, 0.7, 0.2)),
                        TextFont {
                            font_size: 18.0,
                            ..default()
                        },
                    ));
                });
        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn single_esc_arms_timer() {
        let mut timer = None;
        let action = quit_logic(true, &mut timer, Duration::ZERO);
        assert_eq!(action, QuitAction::Armed);
        assert!(timer.is_some());
    }

    #[test]
    fn double_esc_quits() {
        let mut timer = None;
        quit_logic(true, &mut timer, Duration::ZERO);
        let action = quit_logic(true, &mut timer, Duration::from_millis(500));
        assert_eq!(action, QuitAction::Quit);
        assert!(timer.is_none());
    }

    #[test]
    fn timer_expires_resets() {
        let mut timer = None;
        quit_logic(true, &mut timer, Duration::ZERO);
        assert!(timer.is_some());
        // Tick past the 2s window
        let action = quit_logic(false, &mut timer, Duration::from_secs(3));
        assert_eq!(action, QuitAction::None);
        assert!(timer.is_none());
    }

    #[test]
    fn esc_after_expiry_rearms() {
        let mut timer = None;
        quit_logic(true, &mut timer, Duration::ZERO);
        // Expire
        quit_logic(false, &mut timer, Duration::from_secs(3));
        assert!(timer.is_none());
        // New ESC re-arms
        let action = quit_logic(true, &mut timer, Duration::ZERO);
        assert_eq!(action, QuitAction::Armed);
        assert!(timer.is_some());
    }

    #[test]
    fn no_input_is_noop() {
        let mut timer = None;
        let action = quit_logic(false, &mut timer, Duration::from_millis(100));
        assert_eq!(action, QuitAction::None);
        assert!(timer.is_none());
    }
}
