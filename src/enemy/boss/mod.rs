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
use serde::Deserialize;

use crate::camera::components::ScreenShake;
use crate::enemy::components::*;
use crate::enemy::events::EnemyDied;
use crate::tuning::{Tuned, TuningData, embedded_tuning, require_positive};

const BOSS_COLOR: Color = Color::srgb(0.8, 0.1, 0.1);
const BOSS_UI_COLOR: Color = Color::srgb(1.0, 0.3, 0.3);
const BOSS_SIZE: f32 = 28.0;

/// Marker for Boss enemies. Used to filter the screen-shake observer.
#[derive(Component)]
pub struct Boss;

/// The Boss's tuning numbers, from `stats.ron` next to this file.
#[derive(Asset, TypePath, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BossStats {
    pub health: f32,
    pub speed: f32,
    pub loot: u32,
    /// For `Regeneration::rate`: hit points per second.
    pub regen_rate: f32,
    /// For `Armor::reduction`: damage taken off each hit.
    pub armor_reduction: f32,
    /// For `SplitsOnDeath::count`: Shamblers spawned on death.
    pub split_count: u32,
}

impl TuningData for BossStats {
    fn validate(&self) -> Result<(), String> {
        require_positive("health", self.health)?;
        require_positive("speed", self.speed)
    }
}

pub struct BossPlugin;

impl Plugin for BossPlugin {
    fn build(&self, app: &mut App) {
        embedded_tuning!(app, BossStats, "stats.ron");
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
                Sprite::from_color(BOSS_COLOR, Vec2::splat(BOSS_SIZE)),
            ));
            // The stats are read when this command runs, so a reload of
            // `stats.ron` shows on the next spawn. The `EnemySpawned`
            // observers run after it and scale the values by wave.
            cmds.queue(|mut entity: EntityWorldMut| {
                let stats = entity.resource::<Tuned<BossStats>>().0.clone();
                entity.insert((
                    Health {
                        current: stats.health,
                        max: stats.health,
                    },
                    MoveSpeed {
                        base: stats.speed,
                        current: stats.speed,
                    },
                    LootValue(stats.loot),
                    Regeneration {
                        rate: stats.regen_rate,
                    },
                    Armor {
                        reduction: stats.armor_reduction,
                    },
                    SplitsOnDeath {
                        count: stats.split_count,
                        spawn_blueprint: "Shambler",
                    },
                ));
            });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{assert_base_stats, enemy_app, spawn_enemy};
    use crate::tuning::parse;

    fn file_stats() -> BossStats {
        parse(include_bytes!("stats.ron")).unwrap()
    }

    #[test]
    fn stats_file_parses_and_validates() {
        file_stats();
    }

    #[test]
    fn spawned_boss_matches_its_file() {
        let mut app = enemy_app();
        let boss = spawn_enemy(&mut app, "Boss", 1);
        let stats = file_stats();
        assert_base_stats(&app, boss, stats.health, stats.speed, stats.loot);

        let boss = app.world().entity(boss);
        let regen = boss.get::<Regeneration>().unwrap();
        assert_eq!(regen.rate, stats.regen_rate);
        let armor = boss.get::<Armor>().unwrap();
        assert_eq!(armor.reduction, stats.armor_reduction);
        let splits = boss.get::<SplitsOnDeath>().unwrap();
        assert_eq!(splits.count, stats.split_count);
        assert_eq!(splits.spawn_blueprint, "Shambler");
    }
}
