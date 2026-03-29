use bevy::prelude::*;
use crate::states::GameState;
use crate::economy::resources::PlayerScrap;
use crate::wave::resources::WaveManager;
use crate::common::constants::*;

#[derive(Resource)]
pub struct PlayerLives(pub u32);

#[derive(Component)]
pub struct ScrapText;

#[derive(Component)]
pub struct LivesText;

#[derive(Component)]
pub struct WaveText;

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
                Text::new(format!("Scrap: {}", STARTING_SCRAP)),
                TextColor(Color::srgb(0.9, 0.8, 0.2)),
                TextFont { font_size: 18.0, ..default() },
                ScrapText,
            ));
            parent.spawn((
                Text::new("Wave: 1 / 20"),
                TextColor(Color::WHITE),
                TextFont { font_size: 18.0, ..default() },
                WaveText,
            ));
            parent.spawn((
                Text::new(format!("Lives: {}", STARTING_LIVES)),
                TextColor(Color::srgb(0.9, 0.3, 0.3)),
                TextFont { font_size: 18.0, ..default() },
                LivesText,
            ));
        });
}

pub fn update_hud(
    scrap: Res<PlayerScrap>,
    lives: Res<PlayerLives>,
    wave_mgr: Option<Res<WaveManager>>,
    mut scrap_query: Query<&mut Text, (With<ScrapText>, Without<LivesText>, Without<WaveText>)>,
    mut lives_query: Query<&mut Text, (With<LivesText>, Without<ScrapText>, Without<WaveText>)>,
    mut wave_query: Query<&mut Text, (With<WaveText>, Without<ScrapText>, Without<LivesText>)>,
) {
    for mut text in &mut scrap_query {
        **text = format!("Scrap: {}", scrap.0);
    }
    for mut text in &mut lives_query {
        **text = format!("Lives: {}", lives.0);
    }
    if let Some(wave_mgr) = wave_mgr {
        for mut text in &mut wave_query {
            **text = format!("Wave: {} / {}", wave_mgr.current_wave + 1, wave_mgr.waves.len());
        }
    }
}
