//! Runner — fast, fragile, low-loot enemy. Steals scrap.

use bevy::prelude::*;
use serde::Deserialize;

use crate::enemy::components::*;
use crate::tuning::{Tuned, TuningData, embedded_tuning, require_positive};

const RUNNER_COLOR: Color = Color::srgb(0.9, 0.8, 0.2);
const RUNNER_UI_COLOR: Color = Color::srgb(1.0, 0.9, 0.3);
const RUNNER_SIZE: f32 = 10.0;

/// Marker for Runner enemies.
#[derive(Component)]
pub struct Runner;

/// The Runner's tuning numbers, from `stats.ron` next to this file.
#[derive(Asset, TypePath, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RunnerStats {
    pub health: f32,
    pub speed: f32,
    pub loot: u32,
}

impl TuningData for RunnerStats {
    fn validate(&self) -> Result<(), String> {
        require_positive("health", self.health)?;
        require_positive("speed", self.speed)
    }
}

pub struct RunnerPlugin;

impl Plugin for RunnerPlugin {
    fn build(&self, app: &mut App) {
        embedded_tuning!(app, RunnerStats, "stats.ron");
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
                Sprite::from_color(RUNNER_COLOR, Vec2::splat(RUNNER_SIZE)),
            ));
            // The stats are read when this command runs, so a reload of
            // `stats.ron` shows on the next spawn. The `EnemySpawned`
            // observers run after it and scale the values by wave.
            cmds.queue(|mut entity: EntityWorldMut| {
                let stats = entity.resource::<Tuned<RunnerStats>>().0.clone();
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
                    y_offset: RUNNER_SIZE / 2.0 + 3.0,
                },
                Sprite::from_color(Color::srgb(0.2, 0.8, 0.2), Vec2::new(16.0, 2.0)),
                Transform::from_translation(Vec3::new(0.0, RUNNER_SIZE / 2.0 + 3.0, 0.1)),
            ));
        },
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::{assert_base_stats, enemy_app, load_tuning, spawn_enemy};
    use crate::tuning::{TuningFile, parse};

    fn file_stats() -> RunnerStats {
        parse(include_bytes!("stats.ron")).unwrap()
    }

    #[test]
    fn stats_file_parses_and_validates() {
        file_stats();
    }

    #[test]
    fn spawned_runner_matches_its_file() {
        let mut app = enemy_app();
        let runner = spawn_enemy(&mut app, "Runner", 1);
        let stats = file_stats();
        assert_base_stats(&app, runner, stats.health, stats.speed, stats.loot);
    }

    #[test]
    fn reload_changes_only_runners_spawned_after_it() {
        let mut app = enemy_app();
        load_tuning::<RunnerStats>(&mut app);
        let before = spawn_enemy(&mut app, "Runner", 1);

        // A reload: the asset server puts the new values in `Assets`,
        // which sends `AssetEvent::Modified`.
        let old = file_stats();
        let new = RunnerStats {
            health: old.health * 2.0,
            speed: old.speed + 10.0,
            loot: old.loot + 5,
        };
        let file = app.world().resource::<TuningFile<RunnerStats>>();
        let handle = file.handle.clone();
        app.world_mut()
            .resource_mut::<Assets<RunnerStats>>()
            .insert(&handle, new.clone())
            .unwrap();
        app.update(); // Sends the event.
        app.update(); // TuningPlugin reads it.
        assert_eq!(app.world().resource::<Tuned<RunnerStats>>().0, new);

        let after = spawn_enemy(&mut app, "Runner", 1);
        assert_base_stats(&app, before, old.health, old.speed, old.loot);
        assert_base_stats(&app, after, new.health, new.speed, new.loot);
    }
}
