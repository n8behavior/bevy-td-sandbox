pub mod components;
pub mod resources;
pub mod systems;

use bevy::prelude::*;
use crate::enemy::systems::EnemyDied;
use crate::states::GameState;

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(systems::on_enemy_died)
            .add_systems(
                Update,
                (systems::scrap_drop_lifetime, systems::click_to_collect_scrap)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
