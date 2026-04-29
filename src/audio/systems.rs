use std::time::Duration;

use bevy::audio::Volume;
use bevy::prelude::*;

use super::events::{GameSound, PlaySound};
use super::resources::SoundAssets;

pub(super) fn init_sound_assets(mut commands: Commands, mut pitches: ResMut<Assets<Pitch>>) {
    let mut add = |sound: GameSound| -> Handle<Pitch> {
        pitches.add(Pitch::new(
            sound.pitch_hz(),
            Duration::from_millis(sound.duration_ms()),
        ))
    };
    commands.insert_resource(SoundAssets {
        tower_destroyed: add(GameSound::TowerDestroyed),
        tower_repaired: add(GameSound::TowerRepaired),
        enemy_death: add(GameSound::EnemyDeath),
        enemy_brute_attack: add(GameSound::EnemyBruteAttack),
        scrap_drop: add(GameSound::ScrapDrop),
        scrap_collected: add(GameSound::ScrapCollected),
        wave_boss_spawn: add(GameSound::WaveBossSpawn),
        wave_start: add(GameSound::WaveStart),
        game_over: add(GameSound::GameOver),
    });
}

pub(super) fn on_play_sound(
    trigger: On<PlaySound>,
    mut commands: Commands,
    sounds: Res<SoundAssets>,
) {
    let game_sound = trigger.0;
    let handle = sounds.get(game_sound);
    commands.spawn((
        AudioPlayer::<Pitch>(handle.clone()),
        PlaybackSettings::DESPAWN.with_volume(Volume::Linear(game_sound.volume())),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_variants_covered() {
        assert_eq!(GameSound::ALL.len(), 9);
    }

    #[test]
    fn pitch_values_positive() {
        for sound in GameSound::ALL {
            assert!(sound.pitch_hz() > 0.0, "{sound:?} has non-positive pitch");
        }
    }

    #[test]
    fn duration_values_positive() {
        for sound in GameSound::ALL {
            assert!(sound.duration_ms() > 0, "{sound:?} has zero duration");
        }
    }

    #[test]
    fn volume_values_in_range() {
        for sound in GameSound::ALL {
            let v = sound.volume();
            assert!(v > 0.0 && v <= 1.0, "{sound:?} volume {v} out of range");
        }
    }

    #[test]
    fn specific_sound_values() {
        assert_eq!(GameSound::TowerDestroyed.pitch_hz(), 82.0);
        assert_eq!(GameSound::TowerDestroyed.duration_ms(), 300);
        assert_eq!(GameSound::TowerDestroyed.volume(), 0.5);

        assert_eq!(GameSound::GameOver.pitch_hz(), 165.0);
        assert_eq!(GameSound::GameOver.duration_ms(), 400);
        assert_eq!(GameSound::GameOver.volume(), 0.6);
    }
}
