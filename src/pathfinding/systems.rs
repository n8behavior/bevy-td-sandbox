use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::enemy::components::{Enemy, EnemyState};
use crate::pile::resources::{EdgeCells, PileState};
use crate::pile::systems::{nearest_edge_cell, nearest_pile_cell};

pub fn recalculate_enemy_paths(
    mut commands: Commands,
    enemies: Query<(Entity, &AgentPos, &EnemyState), With<Enemy>>,
    pile_state: Res<PileState>,
    edge_cells: Res<EdgeCells>,
) {
    for (entity, agent_pos, state) in &enemies {
        let goal = match state {
            EnemyState::Approaching => nearest_pile_cell(agent_pos.0, &pile_state),
            EnemyState::Fleeing => nearest_edge_cell(agent_pos.0, &edge_cells.0),
        };
        commands
            .entity(entity)
            .remove::<(NextPos, Path)>()
            .insert(Pathfind::new(goal).mode(PathfindMode::Waypoints));
    }
}
