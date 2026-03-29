use bevy::prelude::*;
use crate::tower::components::TowerType;
use crate::tower::placement::SelectedTower;
use crate::states::{GameState, PlayPhase};
use crate::wave::resources::WaveManager;


const LABEL_COLOR: Color = Color::srgb(0.95, 0.85, 0.5);
const HINT_COLOR: Color = Color::srgb(0.7, 0.65, 0.5);
const STAT_COLOR: Color = Color::srgb(0.6, 0.6, 0.55);

#[derive(Component)]
pub struct TowerButton(pub TowerType);

#[derive(Component)]
pub struct WavePreviewPanel;

pub fn setup_tower_palette(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.12, 0.1, 0.08, 0.85)),
            DespawnOnExit(GameState::Playing),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("TOWERS (1-4)"),
                TextColor(LABEL_COLOR),
                TextFont { font_size: 15.0, ..default() },
            ));

            let towers = [
                TowerType::ScrapGun,
                TowerType::TarPit,
                TowerType::Explosive,
                TowerType::Railgun,
            ];

            for (i, tower_type) in towers.iter().enumerate() {
                let stats = tower_type.stats();
                let cost = tower_type.cost();
                let special = match tower_type {
                    TowerType::TarPit => "  SLOW",
                    TowerType::Explosive => "  AOE",
                    _ => "",
                };

                parent
                    .spawn((
                        Node {
                            padding: UiRect::new(Val::Px(4.0), Val::Px(4.0), Val::Px(2.0), Val::Px(2.0)),
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        TowerButton(*tower_type),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(format!("{}: {:?}  {}$", i + 1, tower_type, cost)),
                            TextColor(tower_type.ui_color()),
                            TextFont { font_size: 14.0, ..default() },
                        ));
                        btn.spawn((
                            Text::new(format!(
                                "  DMG:{:.0}  RNG:{:.0}  SPD:{:.1}{special}",
                                stats.damage, stats.range, stats.fire_rate,
                            )),
                            TextColor(STAT_COLOR),
                            TextFont { font_size: 11.0, ..default() },
                        ));
                    });
            }

            parent.spawn((
                Text::new("\nENTER: Start Wave\nESC: Deselect\nESC ESC: Quit\nR-Click: Collect Scrap"),
                TextColor(HINT_COLOR),
                TextFont { font_size: 12.0, ..default() },
            ));
        });

    // Wave preview panel (right side, shown during Building phase)
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                right: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.12, 0.1, 0.08, 0.85)),
            Visibility::Hidden,
            WavePreviewPanel,
            DespawnOnExit(GameState::Playing),
        ));
}

pub fn highlight_selected_tower(
    selected: Res<SelectedTower>,
    mut buttons: Query<(&TowerButton, &mut BackgroundColor)>,
) {
    for (btn, mut bg) in &mut buttons {
        if selected.0 == Some(btn.0) {
            *bg = BackgroundColor(Color::srgba(0.5, 0.45, 0.2, 0.6));
        } else {
            *bg = BackgroundColor(Color::NONE);
        }
    }
}

pub fn update_wave_preview(
    mut commands: Commands,
    wave_mgr: Option<Res<WaveManager>>,
    phase: Option<Res<State<PlayPhase>>>,
    mut panel_query: Query<(Entity, &mut Visibility), With<WavePreviewPanel>>,
) {
    let Ok((panel_entity, mut vis)) = panel_query.single_mut() else {
        return;
    };

    let Some(wave_mgr) = wave_mgr else {
        *vis = Visibility::Hidden;
        return;
    };

    let is_building = phase.is_some_and(|p| *p.get() == PlayPhase::Building);
    if !is_building {
        *vis = Visibility::Hidden;
        return;
    }

    *vis = Visibility::Inherited;

    // Clear old children and rebuild
    commands.entity(panel_entity).despawn_related::<Children>();

    let wave_idx = wave_mgr.current_wave as usize;
    if wave_idx >= wave_mgr.waves.len() {
        commands.entity(panel_entity).with_children(|parent| {
            parent.spawn((
                Text::new("ALL WAVES COMPLETE!"),
                TextColor(LABEL_COLOR),
                TextFont { font_size: 15.0, ..default() },
            ));
        });
        return;
    }

    let wave = &wave_mgr.waves[wave_idx];

    commands.entity(panel_entity).with_children(|parent| {
        parent.spawn((
            Text::new(format!("== NEXT WAVE ({}/{}) ==", wave_idx + 1, wave_mgr.waves.len())),
            TextColor(LABEL_COLOR),
            TextFont { font_size: 15.0, ..default() },
        ));

        for we in &wave.enemies {
            let base_hp = we.enemy_type.base_health() * we.health_multiplier;
            let base_spd = we.enemy_type.base_speed() * we.speed_multiplier;
            let color = we.enemy_type.ui_color();

            parent.spawn((
                Text::new(format!(
                    " {:>2}x {:?}  HP:{:.0}  SPD:{:.0}",
                    we.count, we.enemy_type, base_hp, base_spd,
                )),
                TextColor(color),
                TextFont { font_size: 13.0, ..default() },
            ));
        }

        parent.spawn((
            Text::new(format!("\nSpawn interval: {:.1}s", wave.spawn_interval)),
            TextColor(STAT_COLOR),
            TextFont { font_size: 11.0, ..default() },
        ));

        parent.spawn((
            Text::new("[ENTER to start]"),
            TextColor(HINT_COLOR),
            TextFont { font_size: 12.0, ..default() },
        ));
    });
}
