pub mod systems;

use bevy::prelude::*;
use crate::enemy::components::{Dead, Enemy};

#[derive(Event)]
pub struct GridChanged;

pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(
            |_trigger: On<GridChanged>,
             commands: Commands,
             enemies: Query<Entity, (With<Enemy>, Without<Dead>)>,
             goal_query: Query<
                 &crate::grid::components::GridCell,
                 With<crate::grid::components::GoalPoint>,
             >| {
                systems::recalculate_enemy_paths(commands, enemies, goal_query);
            },
        );
    }
}
