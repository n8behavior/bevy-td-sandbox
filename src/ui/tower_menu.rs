use crate::enemy::components::{Enemy, EnemyRegistry};
use crate::states::{GameMode, GameState, PlayPhase};
use crate::tower::components::{TargetingMode, TowerRegistry};
use crate::tower::placement::SelectedTower;
use crate::ui::hud::{HudPanel, HudState};
use crate::wave::resources::WaveManager;
use bevy::prelude::*;

const LABEL_COLOR: Color = Color::srgb(0.95, 0.85, 0.5);
const HINT_COLOR: Color = Color::srgb(0.7, 0.65, 0.5);
const STAT_COLOR: Color = Color::srgb(0.6, 0.6, 0.55);

/// Index into the tower palette, stored on each tower-selection button.
#[derive(Component)]
pub struct TowerPaletteIndex(pub usize);

/// Marker for the wave preview panel (right side of screen).
#[derive(Component)]
pub struct WavePreviewPanel;

/// Where a UI panel is anchored on screen.
pub enum PanelAnchor {
    BottomLeft,
    BottomRight,
    TopRight,
}

/// Pixel offset below the top HUD bar (`hud::setup_hud` height = 40).
const TOP_HUD_HEIGHT_PX: f32 = 40.0;

/// Builds the common component bundle for an anchored UI panel.
pub fn panel_node(anchor: PanelAnchor) -> (Node, BackgroundColor, DespawnOnExit<GameState>) {
    let mut node = Node {
        position_type: PositionType::Absolute,
        flex_direction: FlexDirection::Column,
        row_gap: Val::Px(4.0),
        padding: UiRect::all(Val::Px(10.0)),
        ..default()
    };
    match anchor {
        PanelAnchor::BottomLeft => {
            node.bottom = Val::Px(10.0);
            node.left = Val::Px(10.0);
        }
        PanelAnchor::BottomRight => {
            node.bottom = Val::Px(10.0);
            node.right = Val::Px(10.0);
        }
        PanelAnchor::TopRight => {
            node.top = Val::Px(TOP_HUD_HEIGHT_PX + 10.0);
            node.right = Val::Px(10.0);
        }
    }
    (
        node,
        BackgroundColor(Color::srgba(0.12, 0.1, 0.08, 0.85)),
        DespawnOnExit(GameState::Playing),
    )
}

pub fn setup_tower_palette(
    mut commands: Commands,
    registry: Res<TowerRegistry>,
    game_mode: Res<GameMode>,
) {
    commands
        .spawn((panel_node(PanelAnchor::BottomLeft), HudPanel))
        .with_children(|parent| {
            parent.spawn((
                Text::new(format!("TOWERS (1-{})", registry.blueprints.len())),
                TextColor(LABEL_COLOR),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
            ));

            for (i, blueprint) in registry.blueprints.iter().enumerate() {
                let special = if blueprint.special_label.is_empty() {
                    String::new()
                } else {
                    format!("  {}", blueprint.special_label)
                };

                parent
                    .spawn((
                        Node {
                            padding: UiRect::new(
                                Val::Px(4.0),
                                Val::Px(4.0),
                                Val::Px(2.0),
                                Val::Px(2.0),
                            ),
                            flex_direction: FlexDirection::Column,
                            ..default()
                        },
                        TowerPaletteIndex(i),
                    ))
                    .with_children(|btn| {
                        btn.spawn((
                            Text::new(format!(
                                "{}: {}  {}${special}",
                                i + 1,
                                blueprint.name,
                                blueprint.cost
                            )),
                            TextColor(blueprint.ui_color),
                            TextFont {
                                font_size: 14.0,
                                ..default()
                            },
                        ));
                    });
            }

            parent.spawn((
                Text::new(format_targeting_legend()),
                TextColor(HINT_COLOR),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
            ));

            let hints = if *game_mode == GameMode::Endless {
                "\nTab: Toggle HUD\nESC: Deselect\nESC ESC: Quit"
            } else {
                "\nENTER: Start Wave\nTab: Toggle HUD\nESC: Deselect\nESC ESC: Quit"
            };
            parent.spawn((
                Text::new(hints),
                TextColor(HINT_COLOR),
                TextFont {
                    font_size: 12.0,
                    ..default()
                },
            ));
        });

    // Wave preview panel (top-right, content adapts per phase).
    // Anchored below the top HUD bar; doesn't overlap with the upgrade panel
    // anchored bottom-right.
    commands.spawn((
        panel_node(PanelAnchor::TopRight),
        Visibility::Hidden,
        HudPanel,
        WavePreviewPanel,
    ));
}

pub fn highlight_selected_tower(
    selected: Res<SelectedTower>,
    mut buttons: Query<(&TowerPaletteIndex, &mut BackgroundColor)>,
) {
    for (btn, mut bg) in &mut buttons {
        if selected.index == Some(btn.0) {
            *bg = BackgroundColor(Color::srgba(0.5, 0.45, 0.2, 0.6));
        } else {
            *bg = BackgroundColor(Color::NONE);
        }
    }
}

// ---------------------------------------------------------------------------
// Pure formatting helpers (testable without ECS)
// ---------------------------------------------------------------------------

/// Formats the targeting mode legend for the tower palette.
///
/// Uses [`TargetingMode::label()`] so the legend stays in sync with the
/// single-letter codes shown on tower sprites.
pub(crate) fn format_targeting_legend() -> String {
    format!(
        "\nTARGETING\n{}: Closest    {}: High HP\n{}: Low HP     {}: Furthest",
        TargetingMode::Closest.label(),
        TargetingMode::HighestHp.label(),
        TargetingMode::LowestHp.label(),
        TargetingMode::FurthestAlongPath.label(),
    )
}

/// Formats the condensed enemy status shown during the Defending phase.
pub(crate) fn format_defend_status(alive: usize, queued: usize) -> String {
    format!("Enemies: {alive} alive, {queued} queued")
}

pub fn update_wave_preview(
    mut commands: Commands,
    game_mode: Res<GameMode>,
    wave_mgr: Option<Res<WaveManager>>,
    phase: Option<Res<State<PlayPhase>>>,
    hud_state: Res<HudState>,
    enemies: Query<(), With<Enemy>>,
    registry: Res<EnemyRegistry>,
    mut panel_query: Query<(Entity, &mut Visibility), With<WavePreviewPanel>>,
) {
    let Ok((panel_entity, mut vis)) = panel_query.single_mut() else {
        return;
    };

    // Respect HUD toggle.
    if !hud_state.is_visible() {
        return;
    }

    // No wave preview in Endless mode.
    if *game_mode == GameMode::Endless {
        *vis = Visibility::Hidden;
        return;
    }

    let Some(wave_mgr) = wave_mgr else {
        *vis = Visibility::Hidden;
        return;
    };

    let is_building = phase.is_some_and(|p| *p.get() == PlayPhase::Building);

    *vis = Visibility::Inherited;

    // Clear old children and rebuild
    commands.entity(panel_entity).despawn_related::<Children>();

    let wave_idx = wave_mgr.current_wave as usize;

    if !is_building {
        // Defending phase: show condensed enemy status.
        let alive = enemies.iter().count();
        let queued = wave_mgr.spawn_queue.len();
        commands.entity(panel_entity).with_children(|parent| {
            parent.spawn((
                Text::new(format!(
                    "== WAVE {}/{} ==",
                    wave_idx + 1,
                    wave_mgr.waves.len()
                )),
                TextColor(LABEL_COLOR),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
            ));
            parent.spawn((
                Text::new(format_defend_status(alive, queued)),
                TextColor(STAT_COLOR),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        });
        return;
    }

    if wave_idx >= wave_mgr.waves.len() {
        commands.entity(panel_entity).with_children(|parent| {
            parent.spawn((
                Text::new("ALL WAVES COMPLETE!"),
                TextColor(LABEL_COLOR),
                TextFont {
                    font_size: 15.0,
                    ..default()
                },
            ));
        });
        return;
    }

    let wave = &wave_mgr.waves[wave_idx];

    commands.entity(panel_entity).with_children(|parent| {
        parent.spawn((
            Text::new(format!(
                "== NEXT WAVE ({}/{}) ==",
                wave_idx + 1,
                wave_mgr.waves.len()
            )),
            TextColor(LABEL_COLOR),
            TextFont {
                font_size: 15.0,
                ..default()
            },
        ));

        for we in &wave.enemies {
            // Look up presentation metadata from the registry; fall back
            // to a neutral label if a blueprint isn't registered yet.
            let (color, label) = match registry.lookup(we.enemy_blueprint) {
                Some(bp) => (bp.ui_color, bp.name),
                None => (STAT_COLOR, we.enemy_blueprint),
            };

            parent.spawn((
                Text::new(format!(" {:>2}x {label}", we.count)),
                TextColor(color),
                TextFont {
                    font_size: 13.0,
                    ..default()
                },
            ));
        }

        parent.spawn((
            Text::new(format!("\nSpawn interval: {:.1}s", wave.spawn_interval)),
            TextColor(STAT_COLOR),
            TextFont {
                font_size: 11.0,
                ..default()
            },
        ));

        parent.spawn((
            Text::new("[ENTER to start]"),
            TextColor(HINT_COLOR),
            TextFont {
                font_size: 12.0,
                ..default()
            },
        ));
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targeting_legend_contains_all_modes() {
        let legend = format_targeting_legend();
        for mode in TargetingMode::ALL {
            assert!(
                legend.contains(mode.label()),
                "legend missing label for {:?}",
                mode,
            );
        }
        assert!(legend.contains("Closest"));
        assert!(legend.contains("High HP"));
        assert!(legend.contains("Low HP"));
        assert!(legend.contains("Furthest"));
    }

    #[test]
    fn defend_status_formats_counts() {
        assert_eq!(format_defend_status(5, 3), "Enemies: 5 alive, 3 queued");
        assert_eq!(format_defend_status(0, 0), "Enemies: 0 alive, 0 queued");
    }
}
