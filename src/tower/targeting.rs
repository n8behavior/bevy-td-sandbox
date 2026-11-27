use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use bevy::ui_widgets::Activate;

use super::components::*;

// ---------------------------------------------------------------------------
// Targeting mode label on tower (in-world single letter)
// ---------------------------------------------------------------------------

/// Spawn a Text2d label on towers that just received a TargetingMode component.
pub fn spawn_targeting_label(
    mut commands: Commands,
    query: Query<(Entity, &TargetingMode), Added<TargetingMode>>,
) {
    for (entity, mode) in &query {
        commands.entity(entity).with_child((
            TargetingModeLabel,
            Text2d::new(mode.label()),
            TextFont {
                font_size: FontSize::Px(10.0),
                ..default()
            },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        ));
    }
}

/// Update the label text when targeting mode changes.
pub fn update_targeting_label(
    towers: Query<(&TargetingMode, &Children), Changed<TargetingMode>>,
    mut labels: Query<&mut Text2d, With<TargetingModeLabel>>,
) {
    for (mode, children) in &towers {
        for child in children.iter() {
            if let Ok(mut text) = labels.get_mut(child) {
                **text = mode.label().to_string();
            }
        }
    }
}

/// Counter-rotate targeting labels so they stay upright regardless of turret rotation.
pub fn stabilize_targeting_labels(
    towers: Query<(&Transform, &Children), (With<Tower>, With<TargetingMode>)>,
    mut labels: Query<&mut Transform, (With<TargetingModeLabel>, Without<Tower>)>,
) {
    for (tower_tf, children) in &towers {
        for child in children.iter() {
            if let Ok(mut label_tf) = labels.get_mut(child) {
                label_tf.rotation = tower_tf.rotation.inverse();
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Inline targeting buttons (in the upgrade panel)
// ---------------------------------------------------------------------------

/// Marker on each targeting-mode button inside the upgrade panel. Click events
/// look up this component to know which mode to apply.
#[derive(Component)]
pub struct TargetingButton {
    pub tower: Entity,
    pub mode: TargetingMode,
}

const BUTTON_ACTIVE: Color = Color::srgba(0.3, 0.7, 0.3, 0.8);
const BUTTON_INACTIVE: Color = Color::srgba(0.2, 0.2, 0.2, 0.7);
const BUTTON_HOVER: Color = Color::srgba(0.5, 0.5, 0.2, 0.8);

/// Background color for a targeting button based on hover/active state.
pub fn targeting_button_color(hovered: bool, is_active: bool) -> Color {
    if is_active {
        BUTTON_ACTIVE
    } else if hovered {
        BUTTON_HOVER
    } else {
        BUTTON_INACTIVE
    }
}

/// Observer: when a `TargetingButton` is activated, apply its mode to the
/// bound tower entity. The buttons carry `ActivateOnPress`, so this fires on
/// press, during input handling, before `Update` runs.
pub fn on_targeting_button_activate(
    activate: On<Activate>,
    buttons: Query<&TargetingButton>,
    mut towers: Query<&mut TargetingMode, With<Tower>>,
) {
    let Ok(button) = buttons.get(activate.entity) else {
        return;
    };
    if let Ok(mut mode) = towers.get_mut(button.tower) {
        *mode = button.mode;
    }
}

/// Refresh button background colors on hover/state changes. A pressed
/// button's mode is already applied by `on_targeting_button_activate`, so it
/// shows as active on the same frame.
pub fn refresh_targeting_button_colors(
    mut buttons: Query<(&Hovered, &TargetingButton, &mut BackgroundColor)>,
    towers: Query<&TargetingMode, With<Tower>>,
) {
    for (hovered, button, mut bg) in &mut buttons {
        let is_active = towers
            .get(button.tower)
            .map(|m| *m == button.mode)
            .unwrap_or(false);
        *bg = BackgroundColor(targeting_button_color(hovered.get(), is_active));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_helpers::test_app;

    #[test]
    fn active_color_when_active() {
        assert_eq!(targeting_button_color(false, true), BUTTON_ACTIVE);
    }

    #[test]
    fn hover_color_only_when_inactive() {
        assert_eq!(targeting_button_color(true, false), BUTTON_HOVER);
        assert_eq!(targeting_button_color(true, true), BUTTON_ACTIVE);
    }

    #[test]
    fn inactive_color_when_not_hovered() {
        assert_eq!(targeting_button_color(false, false), BUTTON_INACTIVE);
    }

    #[test]
    fn activating_button_sets_tower_mode() {
        let mut app = test_app();
        app.add_observer(on_targeting_button_activate);
        let tower = app.world_mut().spawn((Tower, TargetingMode::Closest)).id();
        let button = app
            .world_mut()
            .spawn(TargetingButton {
                tower,
                mode: TargetingMode::FurthestAlongPath,
            })
            .id();

        app.world_mut().trigger(Activate { entity: button });

        assert_eq!(
            *app.world().get::<TargetingMode>(tower).unwrap(),
            TargetingMode::FurthestAlongPath
        );
    }
}
