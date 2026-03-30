use bevy::prelude::*;
use bevy_northstar::prelude::*;

use crate::enemy::components::{Dead, Enemy};
use crate::grid::components::{GoalPoint, GridCell};

pub fn recalculate_enemy_paths(
    mut commands: Commands,
    enemies: Query<Entity, (With<Enemy>, Without<Dead>)>,
    goal_query: Query<&GridCell, With<GoalPoint>>,
) {
    let Ok(goal_cell) = goal_query.single() else {
        return;
    };
    let goal_pos = UVec3::new(goal_cell.coord.x as u32, goal_cell.coord.y as u32, 0);

    for entity in &enemies {
        commands
            .entity(entity)
            .insert(Pathfind::new(goal_pos));
    }
}
