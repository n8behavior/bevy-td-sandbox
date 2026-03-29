pub mod systems;

use bevy::prelude::*;

pub struct PathfindingPlugin;

impl Plugin for PathfindingPlugin {
    fn build(&self, app: &mut App) {
        let _ = app;
    }
}
