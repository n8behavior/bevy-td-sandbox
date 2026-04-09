//! Scrap drop economy: loot spawning, lifetime, and visual effects.
//!
//! When an enemy dies the economy module spawns a golden [`ScrapDrop`] entity
//! that blinks and despawns after [`SCRAP_DROP_LIFETIME`] seconds unless
//! collected first.

pub mod components;
pub mod systems;

use crate::states::GameState;
use bevy::prelude::*;

pub struct EconomyPlugin;

impl Plugin for EconomyPlugin {
    fn build(&self, app: &mut App) {
        app.add_observer(systems::on_enemy_died_spawn_drop)
            .add_observer(systems::on_enemy_died_scrap_sound)
            .add_systems(
                Update,
                (systems::scrap_drop_lifetime, systems::scrap_idle_rotation)
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
