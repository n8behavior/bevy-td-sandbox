//! Shambler — the basic, slow enemy. Steals scrap.

use bevy::prelude::*;
use serde::Deserialize;

use crate::enemy::components::*;
use crate::tuning::{Tuned, TuningData, embedded_tuning, require_positive};

const SHAMBLER_COLOR: Color = Color::srgb(0.4, 0.7, 0.3);
const SHAMBLER_UI_COLOR: Color = Color::srgb(0.5, 0.9, 0.4);
const SHAMBLER_SIZE: f32 = 14.0;

/// Marker for Shambler enemies.
#[derive(Component)]
pub struct Shambler;

/// The Shambler's tuning numbers, from `stats.ron` next to this file.
#[derive(Asset, TypePath, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ShamblerStats {
    pub health: f32,
    pub speed: f32,
    pub loot: u32,
}

impl TuningData for ShamblerStats {
    fn validate(&self) -> Result<(), String> {
        require_positive("health", self.health)?;
        require_positive("speed", self.speed)
    }
}

pub struct ShamblerPlugin;

impl Plugin for ShamblerPlugin {
    fn build(&self, app: &mut App) {
        embedded_tuning!(app, ShamblerStats, "stats.ron");
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
                Sprite::from_color(SHAMBLER_COLOR, Vec2::splat(SHAMBLER_SIZE)),
            ));
            // The stats are read when this command runs, so a reload of
            // `stats.ron` shows on the next spawn. The `EnemySpawned`
            // observers run after it and scale the values by wave.
            cmds.queue(|mut entity: EntityWorldMut| {
                let stats = entity.resource::<Tuned<ShamblerStats>>().0.clone();
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
                ));
            });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::enemy::systems::{health_mult_for_wave, speed_mult_for_wave};
    use crate::test_helpers::{assert_base_stats, enemy_app, spawn_enemy};
    use crate::tuning::parse;

    fn file_stats() -> ShamblerStats {
        parse(include_bytes!("stats.ron")).unwrap()
    }

    #[test]
    fn stats_file_parses_and_validates() {
        file_stats();
    }

    #[test]
    fn spawned_shambler_matches_its_file() {
        let mut app = enemy_app();
        let shambler = spawn_enemy(&mut app, "Shambler", 1);
        let stats = file_stats();
        assert_base_stats(&app, shambler, stats.health, stats.speed, stats.loot);
    }

    #[test]
    fn wave_scaling_applies_on_top_of_the_file() {
        let mut app = enemy_app();
        let shambler = spawn_enemy(&mut app, "Shambler", 5);
        let stats = file_stats();
        assert!(health_mult_for_wave(5) > 1.0);
        assert!(speed_mult_for_wave(5) > 1.0);

        let shambler = app.world().entity(shambler);
        let health = shambler.get::<Health>().unwrap();
        assert_eq!(health.max, stats.health * health_mult_for_wave(5));
        assert_eq!(health.current, health.max);
        let speed = shambler.get::<MoveSpeed>().unwrap();
        assert_eq!(speed.base, stats.speed * speed_mult_for_wave(5));
        assert_eq!(speed.current, speed.base);
    }
}
