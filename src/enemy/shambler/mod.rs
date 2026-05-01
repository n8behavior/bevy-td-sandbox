//! Shambler — the basic, slow enemy. Steals scrap.

use bevy::prelude::*;

use crate::enemy::components::*;

const SHAMBLER_COLOR: Color = Color::srgb(0.4, 0.7, 0.3);
const SHAMBLER_UI_COLOR: Color = Color::srgb(0.5, 0.9, 0.4);
const SHAMBLER_HEALTH: f32 = 50.0;
const SHAMBLER_SPEED: f32 = 40.0;
const SHAMBLER_LOOT: u32 = 10;
const SHAMBLER_SIZE: f32 = 14.0;

/// Marker for Shambler enemies.
#[derive(Component)]
pub struct Shambler;

pub struct ShamblerPlugin;

impl Plugin for ShamblerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register);
    }
}

fn register(mut registry: ResMut<EnemyRegistry>) {
    registry.blueprints.push(EnemyBlueprint {
        name: "Shambler",
        color: SHAMBLER_COLOR,
        ui_color: SHAMBLER_UI_COLOR,
        spawn_fn: |cmds| {
            cmds.insert((
                Shambler,
                StealsScrap,
                EnemyState::default(),
                Health {
                    current: SHAMBLER_HEALTH,
                    max: SHAMBLER_HEALTH,
                },
                MoveSpeed {
                    base: SHAMBLER_SPEED,
                    current: SHAMBLER_SPEED,
                },
                LootValue(SHAMBLER_LOOT),
                Sprite::from_color(SHAMBLER_COLOR, Vec2::splat(SHAMBLER_SIZE)),
            ));
            cmds.with_child((
                HealthBar {
                    y_offset: SHAMBLER_SIZE / 2.0 + 3.0,
                },
                Sprite::from_color(Color::srgb(0.2, 0.8, 0.2), Vec2::new(16.0, 2.0)),
                Transform::from_translation(Vec3::new(0.0, SHAMBLER_SIZE / 2.0 + 3.0, 0.1)),
            ));
        },
    });
}
