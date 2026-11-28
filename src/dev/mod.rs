//! Dev mode: tools for tuning the game, built only with the `dev` feature.
//!
//! Every dev feature lives in this module and hooks into the game from here,
//! with systems and observers on existing components. Each one adds its key
//! to [`keys::DEV_KEYS`] and reports its state through [`DevToggles`].

pub mod indicator;
pub mod keys;

pub use indicator::DevToggles;

use bevy::prelude::*;

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DevToggles>()
            .add_systems(Startup, indicator::setup_indicator)
            .add_systems(
                Update,
                (
                    indicator::toggle_key_list,
                    indicator::update_indicator.run_if(resource_changed::<DevToggles>),
                )
                    .chain(),
            );
    }
}
