//! Generic enemy spawn dispatch.
//!
//! `spawn_from_blueprint` inserts the bare scaffolding every enemy needs
//! (the `Enemy` marker, `EnemyName`, transform, pathfinding agent
//! components, spawn animation, jitter), then runs the blueprint's
//! `spawn_fn` to insert all gameplay components, then triggers
//! `EnemySpawned` so per-capability observers can apply wave scaling.
//!
//! No `Sprite`, `Health`, `MoveSpeed`, or `LootValue` are inserted by the
//! shared helper — those are blueprint concerns. A "Ghost" enemy can omit
//! `Health`; a stationary enemy can omit `MoveSpeed`. Per-capability
//! observers gracefully skip entities that lack the components they scale.

use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::Rng;

use crate::common::constants::GridConfig;
use crate::grid::systems::grid_to_world_cfg;

use super::components::{CellJitter, Enemy, EnemyBlueprint, EnemyName, SpawnAnimation};
use super::events::EnemySpawned;
use super::systems::random_cell_jitter;

/// Spawn an enemy from a blueprint at `spawn_pos`, pathing toward
/// `goal_pos` on the given grid. Returns the new entity.
///
/// `wave` is forwarded to per-capability scaling observers via the
/// `EnemySpawned` event so each capability can apply its own scaling
/// formula (Health uses `health_mult_for_wave`, MoveSpeed uses
/// `speed_mult_for_wave`, future capabilities define their own).
pub fn spawn_from_blueprint(
    commands: &mut Commands,
    blueprint: &EnemyBlueprint,
    spawn_pos: UVec3,
    goal_pos: UVec3,
    grid_entity: Entity,
    config: &GridConfig,
    wave: u32,
) -> Entity {
    let world_pos = grid_to_world_cfg(spawn_pos, config);
    let mut rng = rand::rng();

    let entity = commands
        .spawn((
            Enemy,
            EnemyName(blueprint.name),
            Transform::from_translation(world_pos.extend(1.0)).with_scale(Vec3::ZERO),
            SpawnAnimation {
                timer: Timer::from_seconds(0.25, TimerMode::Once),
            },
            CellJitter(random_cell_jitter(&mut rng)),
            AgentPos(spawn_pos),
            AgentOfGrid(grid_entity),
            Pathfind::new(goal_pos).mode(PathfindMode::AStar),
        ))
        .id();

    (blueprint.spawn_fn)(&mut commands.entity(entity));
    commands.trigger(EnemySpawned { entity, wave });

    entity
}

/// Random offset within a grid cell so enemies don't overlap on the same
/// pixel path. Re-exported here for callers that build entities outside
/// `spawn_from_blueprint` (e.g., test helpers).
pub fn random_jitter<R: Rng>(rng: &mut R) -> Vec2 {
    random_cell_jitter(rng)
}
