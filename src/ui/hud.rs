use crate::common::constants::*;
use crate::economy::components::ScrapDrop;
use crate::endless::resources::EndlessSpawner;
use crate::enemy::components::{Enemy, EnemyState, StolenScrap};
use crate::pile::resources::PileScrap;
use crate::states::{GameMode, GameState, PlayPhase};
use crate::wave::resources::WaveManager;
use bevy::prelude::*;

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
    stolen: Query<(&StolenScrap, &EnemyState), With<Enemy>>,
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
        **text = format!("Pile: {}", pile_scrap.amount);
    }

    let ground_total: u32 = drops.iter().map(|d| d.value).sum();
    for mut text in &mut ground_query {
        **text = format!("Ground: {}", ground_total);
    }

    let stolen_total: u32 = stolen
        .iter()
        .filter(|(_, state)| state.is_alive())
        .map(|(s, _)| s.0)
        .sum();
    for mut text in &mut stolen_query {
        **text = format!("Stolen: {}", stolen_total);
    }

    match *game_mode {
        GameMode::Endless => {
            if let Some(spawner) = &endless_spawner {
                let mins = (spawner.elapsed_time / 60.0) as u32;
                let secs = (spawner.elapsed_time % 60.0) as u32;
                for mut text in &mut wave_query {
                    **text = format!("Time: {:02}:{:02}", mins, secs);
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
                    **text = format!(
                        "Wave: {} / {}",
                        wave_mgr.current_wave + 1,
                        wave_mgr.waves.len()
                    );
                }
            }
            if let Some(phase) = phase {
                for (mut text, mut color) in &mut phase_query {
                    match phase.get() {
                        PlayPhase::Building => {
                            **text = "BUILDING [Enter]".into();
                            *color = TextColor(Color::srgb(0.3, 0.9, 0.3));
                        }
                        PlayPhase::Defending => {
                            **text = "DEFENDING".into();
                            *color = TextColor(Color::srgb(0.9, 0.3, 0.3));
                        }
                    }
                }
            }
        }
    }
}
