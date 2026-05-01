//! Runner — fast, fragile, low-loot enemy. Steals scrap.

use bevy::prelude::*;

use crate::enemy::components::*;

const RUNNER_COLOR: Color = Color::srgb(0.9, 0.8, 0.2);
const RUNNER_UI_COLOR: Color = Color::srgb(1.0, 0.9, 0.3);
const RUNNER_HEALTH: f32 = 30.0;
const RUNNER_SPEED: f32 = 80.0;
const RUNNER_LOOT: u32 = 15;
const RUNNER_SIZE: f32 = 10.0;

/// Marker for Runner enemies.
#[derive(Component)]
pub struct Runner;

pub struct RunnerPlugin;

impl Plugin for RunnerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register);
    }
}

fn register(mut registry: ResMut<EnemyRegistry>) {
    registry.blueprints.push(EnemyBlueprint {
        name: "Runner",
        color: RUNNER_COLOR,
        ui_color: RUNNER_UI_COLOR,
        spawn_fn: |cmds| {
            cmds.insert((
                Runner,
                StealsScrap,
                EnemyState::default(),
                Health {
                    current: RUNNER_HEALTH,
                    max: RUNNER_HEALTH,
                },
                MoveSpeed {
                    base: RUNNER_SPEED,
                    current: RUNNER_SPEED,
                },
                LootValue(RUNNER_LOOT),
                Sprite::from_color(RUNNER_COLOR, Vec2::splat(RUNNER_SIZE)),
            ));
            cmds.with_child((
                HealthBar {
                    y_offset: RUNNER_SIZE / 2.0 + 3.0,
                },
                Sprite::from_color(Color::srgb(0.2, 0.8, 0.2), Vec2::new(16.0, 2.0)),
                Transform::from_translation(Vec3::new(0.0, RUNNER_SIZE / 2.0 + 3.0, 0.1)),
            ));
        },
    });
}
