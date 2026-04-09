pub mod components;
pub mod systems;

use crate::states::GameState;
use bevy::prelude::*;

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(systems::on_enemy_died).add_systems(
            Update,
            (systems::scrap_drop_lifetime, systems::scrap_idle_rotation)
                .run_if(in_state(GameState::Playing)),
        );
    }
}
