//! Raider — the issue-#81-style acceptance scaffold for enemies.
//!
//! Demonstrates that a new enemy with a fundamentally different goal
//! ("destroy towers, ignore the pile") can be added entirely inside
//! `src/enemy/raider/` plus one line in `enemy/mod.rs`. Raider exercises
//! every extension point of the refactor:
//!
//! - **Custom marker**: `Raider` (this module).
//! - **Custom capability**: `RaiderTarget` (this module) — tracks the
//!   tower currently being hunted. A per-enemy system updates
//!   `Pathfind` when the target dies or none is set.
//! - **Opts out of the steal-scrap lifecycle**: omits `StealsScrap`, so
//!   `enemy_reached_pile` and `enemy_escaped` skip Raider entirely.
//! - **Reuses a universal capability**: carries `AttacksTowers` so the
//!   shared `attacks_towers_system` already does the damage tick.
//! - **Custom death observer**: registers a marker-filtered observer
//!   that triggers a small extra rumble on Raider death.

pub mod components;
pub mod systems;

use bevy::prelude::*;

use crate::common::constants::{BRUTE_ATTACK_COOLDOWN, BRUTE_ATTACK_DAMAGE, BRUTE_ATTACK_RANGE};
use crate::enemy::components::*;
use crate::enemy::events::EnemyDied;
use crate::states::{GameState, PlayPhase};

use components::{Raider, RaiderTarget};

const RAIDER_COLOR: Color = Color::srgb(0.85, 0.35, 0.1);
const RAIDER_UI_COLOR: Color = Color::srgb(1.0, 0.55, 0.2);
const RAIDER_HEALTH: f32 = 80.0;
const RAIDER_SPEED: f32 = 55.0;
const RAIDER_LOOT: u32 = 25;
const RAIDER_SIZE: f32 = 16.0;

pub struct RaiderPlugin;

impl Plugin for RaiderPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, register)
            .add_observer(on_raider_died_rumble)
            .add_systems(
                FixedUpdate,
                systems::pick_raider_target
                    .run_if(in_state(GameState::Playing))
                    .run_if(in_state(PlayPhase::Defending)),
            )
            .add_systems(
                Update,
                systems::dev_spawn_raider_keypress
                    .run_if(in_state(GameState::Playing))
                    .run_if(in_state(PlayPhase::Defending)),
            );
    }
}

fn register(mut registry: ResMut<EnemyRegistry>) {
    registry.blueprints.push(EnemyBlueprint {
        name: "Raider",
        color: RAIDER_COLOR,
        ui_color: RAIDER_UI_COLOR,
        spawn_fn: |cmds| {
            cmds.insert((
                Raider,
                RaiderTarget(None),
                Health {
                    current: RAIDER_HEALTH,
                    max: RAIDER_HEALTH,
                },
                MoveSpeed {
                    base: RAIDER_SPEED,
                    current: RAIDER_SPEED,
                },
                LootValue(RAIDER_LOOT),
                Sprite::from_color(RAIDER_COLOR, Vec2::splat(RAIDER_SIZE)),
                AttacksTowers {
                    cooldown: Timer::from_seconds(BRUTE_ATTACK_COOLDOWN * 0.75, TimerMode::Once),
                    damage: BRUTE_ATTACK_DAMAGE * 1.25,
                    range: BRUTE_ATTACK_RANGE,
                },
            ));
            cmds.with_child((
                HealthBar {
                    y_offset: RAIDER_SIZE / 2.0 + 3.0,
                },
                Sprite::from_color(Color::srgb(0.2, 0.8, 0.2), Vec2::new(16.0, 2.0)),
                Transform::from_translation(Vec3::new(0.0, RAIDER_SIZE / 2.0 + 3.0, 0.1)),
            ));
        },
    });
}

/// Brief screen rumble when a Raider falls — distinct from the
/// boss-death shake. Demonstrates a per-blueprint death observer.
fn on_raider_died_rumble(
    trigger: On<EnemyDied>,
    raiders: Query<(), With<Raider>>,
    mut shake: ResMut<crate::camera::components::ScreenShake>,
) {
    if raiders.contains(trigger.entity) {
        shake.intensity = shake.intensity.max(2.0);
        shake.timer = Timer::from_seconds(0.15, TimerMode::Once);
        shake.decay = 0.05;
    }
}
