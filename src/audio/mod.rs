//! Audio plugin — owns all sound behavior.
//!
//! [`GameAudioPlugin`] loads procedural pitch assets at startup and handles
//! [`PlaySound`] events via an observer. Game systems trigger sounds with
//! `commands.trigger(PlaySound(GameSound::X))` and never touch asset handles
//! directly.
//!
//! Tests that don't register this plugin can safely trigger `PlaySound` events —
//! unhandled observer triggers are benign no-ops in Bevy.

pub mod events;
mod resources;
mod systems;

use bevy::prelude::*;

pub use events::{GameSound, PlaySound};

pub struct GameAudioPlugin;

impl Plugin for GameAudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, systems::init_sound_assets)
            .add_observer(systems::on_play_sound);
    }
}
