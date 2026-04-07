pub mod systems;

use crate::enemy::components::{Enemy, EnemyState};
use crate::pile::resources::{EdgeCells, PileState};
use bevy::prelude::*;
use bevy_northstar::prelude::AgentPos;

#[derive(Event)]
pub struct GridChanged;

pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(
            |_trigger: On<GridChanged>,
             commands: Commands,
             enemies: Query<(Entity, &AgentPos, &EnemyState), With<Enemy>>,
             pile_state: Res<PileState>,
             edge_cells: Res<EdgeCells>| {
                systems::recalculate_enemy_paths(commands, enemies, pile_state, edge_cells);
            },
        );
    }
}
