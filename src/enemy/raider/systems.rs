//! Raider-specific systems. The Raider's "hunt towers" behavior lives
//! entirely here — outside the shared scrap-stealer lifecycle.

use bevy::prelude::*;
use bevy_northstar::prelude::*;
use rand::prelude::IndexedRandom;

use crate::common::constants::GridConfig;
use crate::enemy::components::{Enemy, EnemyRegistry};
use crate::enemy::spawn::spawn_from_blueprint;
use crate::grid::systems::world_to_grid;
use crate::pile::resources::EdgeCells;
use crate::pile::resources::PileState;
use crate::pile::systems::nearest_pile_cell;
use crate::tower::components::{BlocksNav, Tower, TowerState};
use crate::wave::resources::WaveManager;

use super::components::{Raider, RaiderTarget};

/// Dev/playtest helper: pressing `R` during the Defending phase spawns
/// one Raider at a random map-edge cell, no scrap cost. Lets us
/// demonstrate the acceptance-scaffold without modifying `wave/`.
pub fn dev_spawn_raider_keypress(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    config: Res<GridConfig>,
    grid_query: Query<Entity, With<OrdinalGrid>>,
    edge_cells: Res<EdgeCells>,
    pile_state: Res<PileState>,
    registry: Res<EnemyRegistry>,
    wave_mgr: Option<Res<WaveManager>>,
) {
    if !keyboard.just_pressed(KeyCode::KeyR) {
        return;
    }
    let Ok(grid_entity) = grid_query.single() else {
        return;
    };
    if edge_cells.0.is_empty() {
        return;
    }
    let Some(blueprint) = registry.lookup("Raider") else {
        return;
    };
    let mut rng = rand::rng();
    let spawn_pos = *edge_cells.0.choose(&mut rng).unwrap();
    let goal_pos = nearest_pile_cell(spawn_pos, &pile_state);
    let wave = wave_mgr.as_ref().map_or(1, |m| m.current_wave + 1);
    spawn_from_blueprint(
        &mut commands,
        blueprint,
        spawn_pos,
        goal_pos,
        grid_entity,
        &config,
        wave,
    );
}

/// Pick the nearest operational tower as the Raider's pathfinding goal.
///
/// Runs each `FixedUpdate` while defending. Re-targets when the current
/// target is missing, despawned, or has become rubble. Updates the
/// Raider's `Pathfind` component so the shared movement system steers
/// toward the chosen tower.
pub fn pick_raider_target(
    mut commands: Commands,
    mut raiders: Query<(Entity, &Transform, &mut RaiderTarget), (With<Enemy>, With<Raider>)>,
    towers: Query<(Entity, &Transform, &TowerState), (With<Tower>, With<BlocksNav>)>,
    config: Res<GridConfig>,
) {
    for (raider_entity, raider_tf, mut target) in &mut raiders {
        // Validate the existing target — drop it if despawned or rubbled.
        if let Some(t) = target.0 {
            let still_valid = towers
                .get(t)
                .is_ok_and(|(_, _, state)| state.is_operational());
            if !still_valid {
                target.0 = None;
            }
        }

        if target.0.is_some() {
            continue;
        }

        // Find the nearest operational tower.
        let raider_pos = raider_tf.translation.truncate();
        let mut best: Option<(Entity, Vec2, f32)> = None;
        for (entity, tower_tf, state) in &towers {
            if !state.is_operational() {
                continue;
            }
            let tower_pos = tower_tf.translation.truncate();
            let dist = raider_pos.distance(tower_pos);
            if best.is_none_or(|(_, _, d)| dist < d) {
                best = Some((entity, tower_pos, dist));
            }
        }

        let Some((tower_entity, tower_pos, _)) = best else {
            continue;
        };

        let Some(grid_pos) = world_to_grid(tower_pos, &config) else {
            continue;
        };
        let goal = UVec3::new(grid_pos.x as u32, grid_pos.y as u32, 0);

        target.0 = Some(tower_entity);
        commands
            .entity(raider_entity)
            .insert(Pathfind::new(goal).mode(PathfindMode::AStar));
    }
}
