//! Boss — the apex enemy. Stacks `Regeneration`, `Armor`, and
//! `SplitsOnDeath` together on top of the standard `StealsScrap` lifecycle,
//! and triggers screen shake on death via a marker-filtered observer.
//!
//! In the previous design these capabilities were a `BossTrait` enum
//! choice (only one at a time, only on Boss). Making them universal
//! components lets future blueprints mix and match — e.g. an armored
//! Brute or a splitting Runner — without touching shared code.

pub mod systems;

use bevy::prelude::*;

use crate::camera::components::ScreenShake;
use crate::enemy::components::*;
use crate::enemy::events::EnemyDied;

const BOSS_COLOR: Color = Color::srgb(0.8, 0.1, 0.1);
const BOSS_UI_COLOR: Color = Color::srgb(1.0, 0.3, 0.3);
const BOSS_HEALTH: f32 = 350.0;
const BOSS_SPEED: f32 = 20.0;
const BOSS_LOOT: u32 = 150;
const BOSS_SIZE: f32 = 28.0;
const BOSS_REGEN_RATE: f32 = 5.0;
const BOSS_ARMOR_REDUCTION: f32 = 5.0;
const BOSS_SPLIT_COUNT: u32 = 3;

/// Marker for Boss enemies. Used to filter the screen-shake observer.
#[derive(Component)]
pub struct Boss;

pub struct BossPlugin;

impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register)
            .add_observer(on_boss_died_shake);
    }
}

fn register(mut registry: ResMut<EnemyRegistry>) {
    registry.blueprints.push(EnemyBlueprint {
        name: "Boss",
        color: BOSS_COLOR,
        ui_color: BOSS_UI_COLOR,
        spawn_fn: |cmds| {
            cmds.insert((
                Boss,
                StealsScrap,
                EnemyState::default(),
                Health {
                    current: BOSS_HEALTH,
                    max: BOSS_HEALTH,
                },
                MoveSpeed {
                    base: BOSS_SPEED,
                    current: BOSS_SPEED,
                },
                LootValue(BOSS_LOOT),
                Sprite::from_color(BOSS_COLOR, Vec2::splat(BOSS_SIZE)),
                Regeneration {
                    rate: BOSS_REGEN_RATE,
                },
                Armor {
                    reduction: BOSS_ARMOR_REDUCTION,
                },
                SplitsOnDeath {
                    count: BOSS_SPLIT_COUNT,
                    spawn_blueprint: "Shambler",
                },
            ));
            cmds.with_child((
                HealthBar {
                    y_offset: BOSS_SIZE / 2.0 + 3.0,
                },
                Sprite::from_color(Color::srgb(0.2, 0.8, 0.2), Vec2::new(16.0, 2.0)),
                Transform::from_translation(Vec3::new(0.0, BOSS_SIZE / 2.0 + 3.0, 0.1)),
            ));
        },
    });
}

/// Trigger a screen-shake when a Boss dies. Other enemies don't fire
/// this — the observer filters by `With<Boss>`.
fn on_boss_died_shake(
    trigger: On<EnemyDied>,
    bosses: Query<(), With<Boss>>,
    mut shake: ResMut<ScreenShake>,
) {
    if bosses.contains(trigger.entity) {
        shake.intensity = 6.0;
        shake.timer = Timer::from_seconds(0.5, TimerMode::Once);
        shake.decay = 0.03;
    }
}
