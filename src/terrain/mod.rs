pub mod components;
pub mod systems;

use crate::states::{GameState, PlayPhase};
use bevy::prelude::*;

pub struct TerrainPlugin;

impl Plugin for TerrainPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<components::TerrainMap>()
            .add_systems(
                OnEnter(GameState::Playing),
                systems::generate_terrain.after(crate::pile::init_pile),
            )
            .add_systems(
                FixedUpdate,
                (
                    systems::apply_puddle_slow,
                    systems::apply_radioactive_damage,
                )
                    .after(crate::enemy::systems::apply_slow_effects)
                    .before(crate::enemy::systems::enemy_movement)
                    .before(crate::enemy::systems::search_wander_movement)
                    .run_if(in_state(GameState::Playing))
                    .run_if(in_state(PlayPhase::Defending)),
            );
    }
}
