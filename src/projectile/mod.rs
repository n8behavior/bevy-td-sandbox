pub mod components;
pub mod systems;

use bevy::prelude::*;

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        let _ = app;
    }
}
