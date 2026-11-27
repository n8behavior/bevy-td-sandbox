//! The DEV indicator: a "DEV" label in the top-left corner, below the HUD
//! bar, with one line for each dev toggle that is on. `F1` swaps those lines
//! for the list of every dev key and its state.

use bevy::prelude::*;
use std::collections::HashMap;

use super::keys::{DEV_KEYS, KEY_LIST};
use crate::ui::tower_menu::TOP_HUD_HEIGHT_PX;

const LABEL_COLOR: Color = Color::srgb(1.0, 0.45, 0.3);
const LINE_COLOR: Color = Color::srgb(0.9, 0.85, 0.7);

/// The state of each dev toggle that is on, keyed by its [`DEV_KEYS`] key.
///
/// A dev feature sets its entry when it turns on, such as
/// `toggles.set(KeyCode::F2, "problems")`, and clears it when it turns off.
/// The DEV indicator renders it.
#[derive(Resource, Default, Debug)]
pub struct DevToggles(HashMap<KeyCode, String>);

impl DevToggles {
    /// Turns the toggle on `key` on, with `state` as its text.
    pub fn set(&mut self, key: KeyCode, state: impl Into<String>) {
        self.0.insert(key, state.into());
    }

    /// Turns the toggle on `key` off.
    pub fn clear(&mut self, key: KeyCode) {
        self.0.remove(&key);
    }

    /// The state text of the toggle on `key`, or `None` when it is off.
    pub fn get(&self, key: KeyCode) -> Option<&str> {
        self.0.get(&key).map(String::as_str)
    }
}

/// Marker for the root node of the DEV indicator.
#[derive(Component)]
pub struct DevIndicator;

/// Marker for the text with one line for each dev toggle that is on.
#[derive(Component)]
pub struct DevToggleLines;

/// Marker for the text with the `F1` list of dev keys.
#[derive(Component)]
pub struct DevKeyList;

/// Spawns the indicator as a root UI node with no `DespawnOnExit`, so it
/// shows in every state, and the HUD toggle does not hide it.
pub fn setup_indicator(mut commands: Commands) {
    let font = TextFont {
        font_size: FontSize::Px(13.0),
        ..default()
    };
    commands
        .spawn((
            DevIndicator,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(TOP_HUD_HEIGHT_PX + 10.0),
                left: Val::Px(10.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.12, 0.1, 0.08, 0.85)),
            GlobalZIndex(100),
        ))
        .with_children(|parent| {
            parent.spawn((
                Text::new("DEV"),
                TextColor(LABEL_COLOR),
                TextFont {
                    font_size: FontSize::Px(15.0),
                    ..default()
                },
            ));
            parent.spawn((
                DevToggleLines,
                Text::default(),
                TextColor(LINE_COLOR),
                font.clone(),
            ));
            parent.spawn((
                DevKeyList,
                Text::default(),
                TextColor(LINE_COLOR),
                font,
                Node {
                    display: Display::None,
                    ..default()
                },
            ));
        });
}

/// `F1` shows or hides the list of dev keys.
pub fn toggle_key_list(keyboard: Res<ButtonInput<KeyCode>>, mut toggles: ResMut<DevToggles>) {
    if !keyboard.just_pressed(KEY_LIST) {
        return;
    }
    if toggles.get(KEY_LIST).is_some() {
        toggles.clear(KEY_LIST);
    } else {
        toggles.set(KEY_LIST, "on");
    }
}

/// One "name: state" line for each dev toggle that is on, in [`DEV_KEYS`] order.
fn toggle_lines(toggles: &DevToggles) -> String {
    DEV_KEYS
        .iter()
        .filter_map(|row| {
            toggles
                .get(row.key)
                .map(|state| format!("{}: {state}", row.name))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// One line for each [`DEV_KEYS`] row: its key, name, state and description.
fn key_list(toggles: &DevToggles) -> String {
    DEV_KEYS
        .iter()
        .map(|row| {
            let state = toggles.get(row.key).unwrap_or("off");
            format!(
                "{:?}  {}: {state}  ({})",
                row.key, row.name, row.description
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Renders [`DevToggles`] into the indicator. While the key list shows, it
/// takes the place of the toggle lines, since it shows every state.
pub fn update_indicator(
    toggles: Res<DevToggles>,
    mut lines: Query<(&mut Text, &mut Node), (With<DevToggleLines>, Without<DevKeyList>)>,
    mut list: Query<(&mut Text, &mut Node), (With<DevKeyList>, Without<DevToggleLines>)>,
) {
    let list_shown = toggles.get(KEY_LIST).is_some();
    for (mut text, mut node) in &mut lines {
        text.0 = toggle_lines(&toggles);
        node.display = if list_shown || text.0.is_empty() {
            Display::None
        } else {
            Display::Flex
        };
    }
    for (mut text, mut node) in &mut list {
        text.0 = key_list(&toggles);
        node.display = if list_shown {
            Display::Flex
        } else {
            Display::None
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dev::DevPlugin;
    use crate::test_helpers::test_app;

    fn dev_app() -> App {
        let mut app = test_app();
        app.insert_resource(ButtonInput::<KeyCode>::default());
        app.add_plugins(DevPlugin);
        app.update();
        app
    }

    /// Presses and releases `key` across one frame.
    fn tap(app: &mut App, key: KeyCode) {
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(key);
        app.update();
        let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
        keys.release(key);
        keys.clear();
    }

    fn key_list_display(app: &mut App) -> Display {
        app.world_mut()
            .query_filtered::<&Node, With<DevKeyList>>()
            .single(app.world())
            .unwrap()
            .display
    }

    #[test]
    fn startup_spawns_one_dev_indicator() {
        let mut app = dev_app();
        let world = app.world_mut();
        let indicators: Vec<Entity> = world
            .query_filtered::<Entity, With<DevIndicator>>()
            .iter(world)
            .collect();
        assert_eq!(indicators.len(), 1);

        let children = world.get::<Children>(indicators[0]).unwrap();
        let has_label = children
            .iter()
            .any(|child| world.get::<Text>(child).is_some_and(|text| text.0 == "DEV"));
        assert!(has_label, "the DEV indicator has no \"DEV\" text");
    }

    #[test]
    fn f1_shows_and_hides_the_key_list() {
        let mut app = dev_app();
        assert_eq!(key_list_display(&mut app), Display::None);

        tap(&mut app, KeyCode::F1);
        assert_eq!(key_list_display(&mut app), Display::Flex);
        let list = app
            .world_mut()
            .query_filtered::<&Text, With<DevKeyList>>()
            .single(app.world())
            .unwrap()
            .0
            .clone();
        assert!(list.contains("F1  Dev keys: on"), "key list: {list:?}");

        // A frame with no press leaves it shown.
        app.update();
        assert_eq!(key_list_display(&mut app), Display::Flex);

        tap(&mut app, KeyCode::F1);
        assert_eq!(key_list_display(&mut app), Display::None);
    }

    #[test]
    fn toggle_lines_list_toggles_that_are_on() {
        let mut toggles = DevToggles::default();
        assert_eq!(toggle_lines(&toggles), "");

        toggles.set(KEY_LIST, "on");
        assert_eq!(toggle_lines(&toggles), "Dev keys: on");

        toggles.clear(KEY_LIST);
        assert_eq!(toggle_lines(&toggles), "");
    }

    #[test]
    fn key_list_shows_every_row_with_its_state() {
        let toggles = DevToggles::default();
        let list = key_list(&toggles);
        assert_eq!(list.lines().count(), DEV_KEYS.len());
        assert!(list.starts_with("F1  Dev keys: off  (Show or hide this list)"));
    }
}
