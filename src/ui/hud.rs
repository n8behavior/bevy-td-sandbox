use crate::common::constants::*;
use crate::economy::components::ScrapDrop;
use crate::endless::resources::EndlessSpawner;
use crate::enemy::components::{Enemy, StolenScrap};
use crate::pile::resources::PileScrap;
use crate::states::{GameMode, GameState, PlayPhase};
use crate::wave::resources::WaveManager;
use bevy::prelude::*;

// ---------------------------------------------------------------------------
// HUD visibility state machine
// ---------------------------------------------------------------------------

/// Whether the HUD panels are currently shown or hidden.
#[derive(Resource, Default, PartialEq, Eq, Debug, Clone, Copy)]
pub enum HudState {
    #[default]
    Visible,
    Hidden,
}

impl HudState {
    /// Returns true when panels should be rendered.
    pub fn is_visible(self) -> bool {
        self == Self::Visible
    }

    /// Returns the opposite state.
    pub fn toggled(self) -> Self {
        match self {
            Self::Visible => Self::Hidden,
            Self::Hidden => Self::Visible,
        }
    }
}

/// Marker for root UI nodes that participate in the HUD toggle.
#[derive(Component)]
pub struct HudPanel;

/// Fired to request the HUD to toggle visibility. Handled by observer.
#[derive(Event)]
pub struct ToggleHud;

/// Emits [`ToggleHud`] when Tab is pressed.
pub fn handle_hud_toggle_input(keyboard: Res<ButtonInput<KeyCode>>, mut commands: Commands) {
    if keyboard.just_pressed(KeyCode::Tab) {
        commands.trigger(ToggleHud);
    }
}

/// Toggles [`HudState`] and applies [`Visibility`] to all [`HudPanel`] entities.
pub fn on_toggle_hud(
    _trigger: On<ToggleHud>,
    mut hud_state: ResMut<HudState>,
    mut panels: Query<&mut Visibility, With<HudPanel>>,
) {
    *hud_state = hud_state.toggled();
    let vis = if hud_state.is_visible() {
        Visibility::Inherited
    } else {
        Visibility::Hidden
    };
    for mut v in &mut panels {
        *v = vis;
    }
}

// ---------------------------------------------------------------------------
// HUD text markers
// ---------------------------------------------------------------------------

#[derive(Component)]
pub struct ScrapText;

#[derive(Component)]
pub struct GroundScrapText;

#[derive(Component)]
pub struct StolenScrapText;

#[derive(Component)]
pub struct WaveText;

#[derive(Component)]
pub struct PhaseText;

pub fn setup_hud(mut commands: Commands) {
    commands
        .spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Px(40.0),
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                padding: UiRect::horizontal(Val::Px(16.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            HudPanel,
            DespawnOnExit(GameState::Playing),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new(format!("Pile: {}", STARTING_SCRAP)),
                TextColor(Color::srgb(0.9, 0.8, 0.2)),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                ScrapText,
            ));
            parent.spawn((
                Text::new("Ground: 0"),
                TextColor(Color::srgb(0.7, 0.6, 0.2)),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                GroundScrapText,
            ));
            parent.spawn((
                Text::new("Stolen: 0"),
                TextColor(Color::srgb(0.9, 0.3, 0.3)),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                StolenScrapText,
            ));
            parent.spawn((
                Text::new("BUILDING"),
                TextColor(Color::srgb(0.3, 0.9, 0.3)),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                PhaseText,
            ));
            parent.spawn((
                Text::new("Wave: 1 / 20"),
                TextColor(Color::WHITE),
                TextFont {
                    font_size: 18.0,
                    ..default()
                },
                WaveText,
            ));
        });
}

pub fn update_hud(
    pile_scrap: Res<PileScrap>,
    game_mode: Res<GameMode>,
    wave_mgr: Option<Res<WaveManager>>,
    endless_spawner: Option<Res<EndlessSpawner>>,
    phase: Option<Res<State<PlayPhase>>>,
    drops: Query<&ScrapDrop>,
    stolen: Query<&StolenScrap, With<Enemy>>,
    mut scrap_query: Query<
        &mut Text,
        (
            With<ScrapText>,
            Without<WaveText>,
            Without<PhaseText>,
            Without<GroundScrapText>,
            Without<StolenScrapText>,
        ),
    >,
    mut ground_query: Query<
        &mut Text,
        (
            With<GroundScrapText>,
            Without<ScrapText>,
            Without<WaveText>,
            Without<PhaseText>,
            Without<StolenScrapText>,
        ),
    >,
    mut stolen_query: Query<
        &mut Text,
        (
            With<StolenScrapText>,
            Without<ScrapText>,
            Without<WaveText>,
            Without<PhaseText>,
            Without<GroundScrapText>,
        ),
    >,
    mut wave_query: Query<
        &mut Text,
        (
            With<WaveText>,
            Without<ScrapText>,
            Without<PhaseText>,
            Without<GroundScrapText>,
            Without<StolenScrapText>,
        ),
    >,
    mut phase_query: Query<
        (&mut Text, &mut TextColor),
        (
            With<PhaseText>,
            Without<ScrapText>,
            Without<WaveText>,
            Without<GroundScrapText>,
            Without<StolenScrapText>,
        ),
    >,
) {
    for mut text in &mut scrap_query {
        **text = format_pile_text(pile_scrap.amount);
    }

    let ground_total: u32 = drops.iter().map(|d| d.value).sum();
    for mut text in &mut ground_query {
        **text = format_ground_text(ground_total);
    }

    let stolen_total: u32 = stolen.iter().map(|s| s.0).sum();
    for mut text in &mut stolen_query {
        **text = format_stolen_text(stolen_total);
    }

    match *game_mode {
        GameMode::Endless => {
            if let Some(spawner) = &endless_spawner {
                for mut text in &mut wave_query {
                    **text = format_wave_text_endless(spawner.elapsed_time);
                }
            }
            for (mut text, mut color) in &mut phase_query {
                **text = "ENDLESS".into();
                *color = TextColor(Color::srgb(0.9, 0.6, 0.2));
            }
        }
        GameMode::Classic => {
            if let Some(wave_mgr) = wave_mgr {
                for mut text in &mut wave_query {
                    **text = format_wave_text_classic(wave_mgr.current_wave, wave_mgr.waves.len());
                }
            }
            if let Some(phase) = phase {
                let (label, label_color) = format_phase_text(phase.get());
                for (mut text, mut color) in &mut phase_query {
                    **text = label.into();
                    *color = TextColor(label_color);
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Pure formatting helpers (testable without ECS)
// ---------------------------------------------------------------------------

/// Formats the pile scrap display text (e.g. `"Pile: 250"`).
pub(crate) fn format_pile_text(amount: u32) -> String {
    format!("Pile: {amount}")
}

/// Formats the ground scrap total display text (e.g. `"Ground: 15"`).
pub(crate) fn format_ground_text(total: u32) -> String {
    format!("Ground: {total}")
}

/// Formats the stolen scrap total display text (e.g. `"Stolen: 7"`).
pub(crate) fn format_stolen_text(total: u32) -> String {
    format!("Stolen: {total}")
}

/// Formats the wave counter for Classic mode.
///
/// Converts the 0-indexed `current_wave` to 1-indexed display
/// (e.g. wave 0 of 20 → `"Wave: 1 / 20"`).
pub(crate) fn format_wave_text_classic(current_wave: u32, total_waves: usize) -> String {
    format!("Wave: {} / {}", current_wave + 1, total_waves)
}

/// Formats the elapsed-time display for Endless mode (e.g. `"Time: 01:05"`).
pub(crate) fn format_wave_text_endless(elapsed_time: f32) -> String {
    let mins = (elapsed_time / 60.0) as u32;
    let secs = (elapsed_time % 60.0) as u32;
    format!("Time: {mins:02}:{secs:02}")
}

/// Returns the phase label and color for Classic mode.
///
/// - Building → `"BUILDING [Enter]"` (green)
/// - Defending → `"DEFENDING"` (red)
pub(crate) fn format_phase_text(phase: &PlayPhase) -> (&'static str, Color) {
    match phase {
        PlayPhase::Building => ("BUILDING [Enter]", Color::srgb(0.3, 0.9, 0.3)),
        PlayPhase::Defending => ("DEFENDING", Color::srgb(0.9, 0.3, 0.3)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hud_state_toggled() {
        assert_eq!(HudState::Visible.toggled(), HudState::Hidden);
        assert_eq!(HudState::Hidden.toggled(), HudState::Visible);
    }

    #[test]
    fn hud_state_is_visible() {
        assert!(HudState::Visible.is_visible());
        assert!(!HudState::Hidden.is_visible());
    }

    #[test]
    fn pile_text_formats_amount() {
        assert_eq!(format_pile_text(0), "Pile: 0");
        assert_eq!(format_pile_text(42), "Pile: 42");
        assert_eq!(format_pile_text(999_999), "Pile: 999999");
    }

    #[test]
    fn ground_text_formats_total() {
        assert_eq!(format_ground_text(0), "Ground: 0");
        assert_eq!(format_ground_text(15), "Ground: 15");
    }

    #[test]
    fn stolen_text_formats_total() {
        assert_eq!(format_stolen_text(0), "Stolen: 0");
        assert_eq!(format_stolen_text(7), "Stolen: 7");
    }

    #[test]
    fn wave_text_classic_one_indexed() {
        assert_eq!(format_wave_text_classic(0, 20), "Wave: 1 / 20");
        assert_eq!(format_wave_text_classic(19, 20), "Wave: 20 / 20");
    }

    #[test]
    fn wave_text_endless_formats_minutes_seconds() {
        assert_eq!(format_wave_text_endless(0.0), "Time: 00:00");
        assert_eq!(format_wave_text_endless(65.5), "Time: 01:05");
        assert_eq!(format_wave_text_endless(3599.0), "Time: 59:59");
    }

    #[test]
    fn phase_text_building() {
        let (label, color) = format_phase_text(&PlayPhase::Building);
        assert_eq!(label, "BUILDING [Enter]");
        assert_eq!(color, Color::srgb(0.3, 0.9, 0.3));
    }

    #[test]
    fn phase_text_defending() {
        let (label, color) = format_phase_text(&PlayPhase::Defending);
        assert_eq!(label, "DEFENDING");
        assert_eq!(color, Color::srgb(0.9, 0.3, 0.3));
    }
}
