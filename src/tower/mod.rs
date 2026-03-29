pub mod components;
pub mod placement;
pub mod systems;

use bevy::prelude::*;

pub struct TowerPlugin;

impl Plugin for TowerPlugin {
    fn build(&self, app: &mut App) {
        let _ = app;
    }
}
