pub mod components;
pub mod resources;
pub mod systems;

use bevy::prelude::*;

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        let _ = app;
    }
}
