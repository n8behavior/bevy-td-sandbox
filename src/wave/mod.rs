pub mod resources;
pub mod systems;

use bevy::prelude::*;

pub struct WavePlugin;

impl Plugin for WavePlugin {
    fn build(&self, app: &mut App) {
        let _ = app;
    }
}
