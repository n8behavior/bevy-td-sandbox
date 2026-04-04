pub mod systems;

use crate::enemy::components::{Dead, Enemy, EnemyPhase};
use crate::pile::resources::{EdgeCells, PileState};
use bevy::prelude::*;

#[derive(Event)]
pub struct GridChanged;

pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(
            |_trigger: On<GridChanged>,
             commands: Commands,
             enemies: Query<(Entity, &AgentPos, &EnemyPhase), (With<Enemy>, Without<Dead>)>,
             pile_state: Res<PileState>,
             edge_cells: Res<EdgeCells>| {
                systems::recalculate_enemy_paths(commands, enemies, pile_state, edge_cells);
            },
        );
    }
}

use bevy_northstar::prelude::AgentPos;
