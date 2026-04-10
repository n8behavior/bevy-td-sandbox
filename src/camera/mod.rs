//! Camera controller systems: scroll-wheel zoom ([`systems::camera_zoom`]),
//! middle-mouse pan ([`systems::camera_pan`]), Home-key reset
//! ([`systems::camera_reset`]), and screen shake ([`systems::apply_screen_shake`]).
//!
//! Per-camera settings live in [`components::CameraController`] on the camera
//! entity. [`components::PanState`] and [`components::ScreenShake`] are global
//! resources.

pub mod components;
pub mod systems;

use crate::states::GameState;
use bevy::prelude::*;

use components::{PanState, ScreenShake};

pub struct CameraPlugin;

impl Plugin for CameraPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PanState>()
            .init_resource::<ScreenShake>()
            .add_systems(
                Update,
                (
                    systems::camera_zoom,
                    systems::camera_pan,
                    systems::camera_reset,
                    systems::apply_screen_shake,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
