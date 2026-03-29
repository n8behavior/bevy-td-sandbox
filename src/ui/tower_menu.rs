use bevy::prelude::*;
use crate::tower::components::TowerType;
use crate::tower::placement::SelectedTower;
use crate::states::GameState;

#[derive(Component)]
pub struct TowerButton(pub TowerType);

#[derive(Component)]
pub struct BuildHint;

pub fn setup_tower_palette(mut commands: Commands) {
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(10.0),
                left: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
            DespawnOnExit(GameState::Playing),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("TOWERS (1-4)"),
                TextColor(Color::srgb(0.8, 0.7, 0.4)),
                TextFont {
                    font_size: 14.0,
                    ..default()
                },
            ));

            let towers = [
                (TowerType::ScrapGun, "1: Scrap Gun", 50),
                (TowerType::TarPit, "2: Tar Pit", 75),
                (TowerType::Explosive, "3: Explosive", 125),
                (TowerType::Railgun, "4: Railgun", 150),
            ];

            for (tower_type, label, cost) in towers {
                parent
                    .spawn((
                        Node {
                            padding: UiRect::all(Val::Px(4.0)),
                            ..default()
                        },
                        TowerButton(tower_type),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(format!("{label} ({cost})  ")),
                            TextColor(tower_type.color()),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                        ));
                    });
            }

            parent.spawn((
                Text::new("\nENTER: Start Wave\nESC: Deselect (x2: Quit)\nR-Click: Collect Scrap"),
                TextColor(Color::srgb(0.5, 0.5, 0.4)),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
                BuildHint,
            ));
        });
}

pub fn highlight_selected_tower(
    selected: Res<SelectedTower>,
    mut buttons: Query<(&TowerButton, &mut BackgroundColor)>,
) {
    for (btn, mut bg) in &mut buttons {
        if selected.0 == Some(btn.0) {
            *bg = BackgroundColor(Color::srgba(0.4, 0.4, 0.2, 0.5));
        } else {
            *bg = BackgroundColor(Color::NONE);
        }
    }
}
