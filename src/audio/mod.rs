pub mod resources;
pub mod systems;

use bevy::prelude::*;

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, systems::init_sound_assets);
    }
}
