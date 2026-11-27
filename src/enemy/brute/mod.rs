//! Brute — heavy, slow enemy that steals scrap *and* attacks adjacent
//! towers. Carries both the `StealsScrap` and `AttacksTowers` capability
//! markers, demonstrating how a single enemy can combine multiple goals.

use bevy::prelude::*;
use serde::Deserialize;

use crate::enemy::components::*;
use crate::tuning::{Tuned, TuningData, embedded_tuning, require_positive};

const BRUTE_COLOR: Color = Color::srgb(0.6, 0.2, 0.5);
const BRUTE_UI_COLOR: Color = Color::srgb(0.8, 0.4, 0.7);
const BRUTE_SIZE: f32 = 18.0;

/// Marker for Brute enemies.
#[derive(Component)]
pub struct Brute;

/// The Brute's tuning numbers, from `stats.ron` next to this file.
#[derive(Asset, TypePath, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BruteStats {
    pub health: f32,
    pub speed: f32,
    pub loot: u32,
    pub attack: BruteAttack,
}

/// The Brute's attack on adjacent towers, for its `AttacksTowers`.
#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct BruteAttack {
    pub damage: f32,
    /// Seconds between hits.
    pub cooldown: f32,
    /// Max world distance to a tower it hits.
    pub range: f32,
}

impl TuningData for BruteStats {
    fn validate(&self) -> Result<(), String> {
        require_positive("health", self.health)?;
        require_positive("speed", self.speed)?;
        require_positive("attack.cooldown", self.attack.cooldown)?;
        require_positive("attack.range", self.attack.range)
    }
}

pub struct BrutePlugin;

impl Plugin for BrutePlugin {
    fn build(&self, app: &mut App) {
        embedded_tuning!(app, BruteStats, "stats.ron");
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
                Sprite::from_color(BRUTE_COLOR, Vec2::splat(BRUTE_SIZE)),
            ));
            // The stats are read when this command runs, so a reload of
            // `stats.ron` shows on the next spawn. The `EnemySpawned`
            // observers run after it and scale the values by wave.
            cmds.queue(|mut entity: EntityWorldMut| {
                let stats = entity.resource::<Tuned<BruteStats>>().0.clone();
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
                    AttacksTowers {
                        cooldown: Timer::from_seconds(stats.attack.cooldown, TimerMode::Once),
                        damage: stats.attack.damage,
                        range: stats.attack.range,
                    },
                ));
            });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{assert_base_stats, enemy_app, spawn_enemy};
    use crate::tuning::parse;
    use std::time::Duration;

    fn file_stats() -> BruteStats {
        parse(include_bytes!("stats.ron")).unwrap()
    }

    #[test]
    fn stats_file_parses_and_validates() {
        file_stats();
    }

    #[test]
    fn spawned_brute_matches_its_file() {
        let mut app = enemy_app();
        let brute = spawn_enemy(&mut app, "Brute", 1);
        let stats = file_stats();
        assert_base_stats(&app, brute, stats.health, stats.speed, stats.loot);

        let attack = app.world().get::<AttacksTowers>(brute).unwrap();
        assert_eq!(attack.damage, stats.attack.damage);
        assert_eq!(attack.range, stats.attack.range);
        let cooldown = Duration::from_secs_f32(stats.attack.cooldown);
        assert_eq!(attack.cooldown.duration(), cooldown);
    }

    #[test]
    fn zero_cooldown_is_rejected() {
        let file = "(
            health: 150.0,
            speed: 25.0,
            loot: 30,
            attack: (damage: 20.0, cooldown: 0.0, range: 30.0),
        )";
        let err = parse::<BruteStats>(file.as_bytes()).unwrap_err();
        assert!(err.to_string().contains("`attack.cooldown`"), "{err}");
    }
}
