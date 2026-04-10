use bevy::prelude::*;
use bevy_northstar::prelude::*;

use super::GridChanged;
use crate::enemy::components::{Enemy, EnemyState};
use crate::pile::resources::{EdgeCells, PileState};
use crate::pile::systems::{nearest_edge_cell, nearest_pile_cell};

/// Observer handler for [`GridChanged`]: recalculates paths for all enemies.
pub fn on_grid_changed(
    _trigger: On<GridChanged>,
    commands: Commands,
    enemies: Query<(Entity, &AgentPos, &EnemyState), With<Enemy>>,
    pile_state: Res<PileState>,
    edge_cells: Res<EdgeCells>,
) {
    recalculate_enemy_paths(commands, enemies, pile_state, edge_cells);
}

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
            // Waypoints mode gives smoother any-angle movement instead of cell-by-cell steps.
            .insert(Pathfind::new(goal).mode(PathfindMode::Waypoints));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::insert_pile;
    use crate::test_helpers::test_app;

    // Note: nearest_pile_cell / nearest_edge_cell edge cases (empty pile, empty
    // edges) are already covered by unit tests in src/pile/systems.rs.

    // -----------------------------------------------------------------------
    // recalculate_enemy_paths
    // -----------------------------------------------------------------------

    #[test]
    fn recalculate_no_enemies_is_noop() {
        let mut app = test_app();
        insert_pile(&mut app, 200);
        app.add_systems(Update, recalculate_enemy_paths);
        app.update();
        // No panic, no entities — just verifying the system handles empty queries.
    }

    #[test]
    fn recalculate_approaching_enemy_gets_pile_goal() {
        let mut app = test_app();
        insert_pile(&mut app, 200);

        let enemy = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyState::Approaching,
                AgentPos(UVec3::new(5, 5, 0)),
            ))
            .id();

        app.add_systems(Update, recalculate_enemy_paths);
        app.update();

        let pf = app
            .world()
            .get::<Pathfind>(enemy)
            .expect("should have Pathfind");
        let pile_state = app.world().resource::<PileState>();
        assert!(
            pile_state.cells.contains(&pf.goal),
            "goal {:?} should be a pile cell",
            pf.goal
        );
        assert_eq!(pf.mode, Some(PathfindMode::Waypoints));
    }

    #[test]
    fn recalculate_fleeing_enemy_gets_edge_goal() {
        let mut app = test_app();
        insert_pile(&mut app, 200);
        // insert_pile inserts empty EdgeCells; provide actual edges for this test.
        app.insert_resource(EdgeCells(vec![UVec3::new(0, 0, 0), UVec3::new(39, 0, 0)]));

        let enemy = app
            .world_mut()
            .spawn((Enemy, EnemyState::Fleeing, AgentPos(UVec3::new(38, 1, 0))))
            .id();

        app.add_systems(Update, recalculate_enemy_paths);
        app.update();

        let pf = app
            .world()
            .get::<Pathfind>(enemy)
            .expect("should have Pathfind");
        assert_eq!(pf.goal, UVec3::new(39, 0, 0));
        assert_eq!(pf.mode, Some(PathfindMode::Waypoints));
    }

    #[test]
    fn recalculate_clears_stale_next_pos_and_path() {
        let mut app = test_app();
        insert_pile(&mut app, 200);

        let enemy = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyState::Approaching,
                AgentPos(UVec3::new(5, 5, 0)),
                NextPos(UVec3::new(6, 5, 0)),
                Path::new(vec![UVec3::new(6, 5, 0), UVec3::new(10, 5, 0)], 5),
            ))
            .id();

        app.add_systems(Update, recalculate_enemy_paths);
        app.update();

        assert!(
            app.world().get::<NextPos>(enemy).is_none(),
            "stale NextPos should be removed"
        );
        assert!(
            app.world().get::<Path>(enemy).is_none(),
            "stale Path should be removed"
        );
        assert!(
            app.world().get::<Pathfind>(enemy).is_some(),
            "new Pathfind should be inserted"
        );
    }

    #[test]
    fn recalculate_multiple_enemies_different_states() {
        let mut app = test_app();
        insert_pile(&mut app, 200);
        app.insert_resource(EdgeCells(vec![UVec3::new(0, 0, 0)]));

        let approaching = app
            .world_mut()
            .spawn((
                Enemy,
                EnemyState::Approaching,
                AgentPos(UVec3::new(5, 5, 0)),
            ))
            .id();
        let fleeing = app
            .world_mut()
            .spawn((Enemy, EnemyState::Fleeing, AgentPos(UVec3::new(1, 1, 0))))
            .id();

        app.add_systems(Update, recalculate_enemy_paths);
        app.update();

        let pile_state = app.world().resource::<PileState>();
        let pf_a = app
            .world()
            .get::<Pathfind>(approaching)
            .expect("approaching should have Pathfind");
        assert!(
            pile_state.cells.contains(&pf_a.goal),
            "approaching goal {:?} should be a pile cell",
            pf_a.goal
        );

        let pf_f = app
            .world()
            .get::<Pathfind>(fleeing)
            .expect("fleeing should have Pathfind");
        assert_eq!(
            pf_f.goal,
            UVec3::new(0, 0, 0),
            "fleeing goal should be nearest edge cell"
        );
    }
}
