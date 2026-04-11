//! Pathfinding subsystem — recalculates enemy paths when the grid changes.
//!
//! Listens for [`GridChanged`] events (triggered by tower placement/sale) and
//! recomputes goals for all active enemies based on their
//! [`EnemyState`](crate::enemy::components::EnemyState).

pub mod systems;

use bevy::prelude::*;

use crate::states::PlayPhase;

/// Fired when the navigation grid changes (tower placed or sold).
///
/// Triggers [`systems::on_grid_changed`] which recalculates paths for all enemies.
#[derive(Event)]
pub struct GridChanged;

pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(systems::on_grid_changed);
        app.add_systems(
            Update,
            (
                systems::log_pathfinding_failures,
                systems::check_stuck_enemies,
            )
                .run_if(in_state(PlayPhase::Defending)),
        );
    }
}
