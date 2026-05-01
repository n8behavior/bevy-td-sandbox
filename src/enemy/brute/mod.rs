//! Brute — heavy, slow enemy that steals scrap *and* attacks adjacent
//! towers. Carries both the `StealsScrap` and `AttacksTowers` capability
//! markers, demonstrating how a single enemy can combine multiple goals.

use bevy::prelude::*;

use crate::common::constants::{BRUTE_ATTACK_COOLDOWN, BRUTE_ATTACK_DAMAGE, BRUTE_ATTACK_RANGE};
use crate::enemy::components::*;

const BRUTE_COLOR: Color = Color::srgb(0.6, 0.2, 0.5);
const BRUTE_UI_COLOR: Color = Color::srgb(0.8, 0.4, 0.7);
const BRUTE_HEALTH: f32 = 150.0;
const BRUTE_SPEED: f32 = 25.0;
const BRUTE_LOOT: u32 = 30;
const BRUTE_SIZE: f32 = 18.0;

/// Marker for Brute enemies.
#[derive(Component)]
pub struct Brute;

pub struct BrutePlugin;

impl Plugin for BrutePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register);
    }
}

fn register(mut registry: ResMut<EnemyRegistry>) {
    registry.blueprints.push(EnemyBlueprint {
        name: "Brute",
        color: BRUTE_COLOR,
        ui_color: BRUTE_UI_COLOR,
        spawn_fn: |cmds| {
            cmds.insert((
                Brute,
                StealsScrap,
                EnemyState::default(),
                Health {
                    current: BRUTE_HEALTH,
                    max: BRUTE_HEALTH,
                },
                MoveSpeed {
                    base: BRUTE_SPEED,
                    current: BRUTE_SPEED,
                },
                LootValue(BRUTE_LOOT),
                Sprite::from_color(BRUTE_COLOR, Vec2::splat(BRUTE_SIZE)),
                AttacksTowers {
                    cooldown: Timer::from_seconds(BRUTE_ATTACK_COOLDOWN, TimerMode::Once),
                    damage: BRUTE_ATTACK_DAMAGE,
                    range: BRUTE_ATTACK_RANGE,
                },
            ));
            cmds.with_child((
                HealthBar {
                    y_offset: BRUTE_SIZE / 2.0 + 3.0,
                },
                Sprite::from_color(Color::srgb(0.2, 0.8, 0.2), Vec2::new(16.0, 2.0)),
                Transform::from_translation(Vec3::new(0.0, BRUTE_SIZE / 2.0 + 3.0, 0.1)),
            ));
        },
    });
}
