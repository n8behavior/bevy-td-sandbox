use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::enemy::components::{Dead, Enemy, EnemyPhase};
use crate::pile::resources::{EdgeCells, PileState};
use crate::pile::systems::{nearest_edge_cell, nearest_pile_cell};

pub fn recalculate_enemy_paths(
    mut commands: Commands,
    enemies: Query<(Entity, &AgentPos, &EnemyPhase), (With<Enemy>, Without<Dead>)>,
    pile_state: Res<PileState>,
    edge_cells: Res<EdgeCells>,
) {
    for (entity, agent_pos, phase) in &enemies {
        let goal = match phase {
            EnemyPhase::Approaching => nearest_pile_cell(agent_pos.0, &pile_state),
            EnemyPhase::Fleeing => nearest_edge_cell(agent_pos.0, &edge_cells.0),
        };
        commands
            .entity(entity)
            .insert(Pathfind::new(goal).mode(PathfindMode::Waypoints));
    }
}
