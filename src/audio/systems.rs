use std::time::Duration;

use bevy::audio::Volume;
use bevy::prelude::*;

use super::resources::SoundAssets;

pub fn init_sound_assets(mut commands: Commands, mut pitches: ResMut<Assets<Pitch>>) {
    commands.insert_resource(SoundAssets {
        tower_scrapgun: pitches.add(Pitch::new(880.0, Duration::from_millis(80))),
        tower_explosive: pitches.add(Pitch::new(110.0, Duration::from_millis(250))),
        tower_railgun: pitches.add(Pitch::new(2200.0, Duration::from_millis(50))),
        tower_chain_lightning: pitches.add(Pitch::new(660.0, Duration::from_millis(120))),
        enemy_death: pitches.add(Pitch::new(220.0, Duration::from_millis(150))),
        scrap_drop: pitches.add(Pitch::new(1760.0, Duration::from_millis(60))),
        scrap_collected: pitches.add(Pitch::new(1320.0, Duration::from_millis(100))),
        boss_spawn: pitches.add(Pitch::new(55.0, Duration::from_millis(500))),
        wave_start: pitches.add(Pitch::new(440.0, Duration::from_millis(200))),
        game_over: pitches.add(Pitch::new(165.0, Duration::from_millis(400))),
        brute_attack: pitches.add(Pitch::new(165.0, Duration::from_millis(100))),
        tower_destroyed: pitches.add(Pitch::new(82.0, Duration::from_millis(300))),
        tower_repaired: pitches.add(Pitch::new(550.0, Duration::from_millis(150))),
    });
}

pub fn play_sound(commands: &mut Commands, sound: &Handle<Pitch>, volume: f32) {
    commands.spawn((
        AudioPlayer::<Pitch>(sound.clone()),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume)),
    ));
}
