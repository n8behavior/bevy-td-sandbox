use bevy::prelude::*;

/// All game sounds, grouped by category.
///
/// Each variant encodes its pitch frequency, duration, and default playback
/// volume — these are the single source of truth used by both asset loading
/// and the playback observer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameSound {
    // Tower weapons
    TowerScrapgun,
    TowerExplosive,
    TowerRailgun,
    TowerChainLightning,
    // Tower state
    TowerDestroyed,
    TowerRepaired,
    // Enemy
    EnemyDeath,
    EnemyBruteAttack,
    // Economy
    ScrapDrop,
    ScrapCollected,
    // Wave / game
    WaveBossSpawn,
    WaveStart,
    GameOver,
}

impl GameSound {
    /// All variants, for iteration in asset init and tests.
    pub const ALL: &[GameSound] = &[
        GameSound::TowerScrapgun,
        GameSound::TowerExplosive,
        GameSound::TowerRailgun,
        GameSound::TowerChainLightning,
        GameSound::TowerDestroyed,
        GameSound::TowerRepaired,
        GameSound::EnemyDeath,
        GameSound::EnemyBruteAttack,
        GameSound::ScrapDrop,
        GameSound::ScrapCollected,
        GameSound::WaveBossSpawn,
        GameSound::WaveStart,
        GameSound::GameOver,
    ];

    /// Pitch frequency in Hz.
    pub const fn pitch_hz(&self) -> f32 {
        match self {
            GameSound::TowerScrapgun => 880.0,
            GameSound::TowerExplosive => 110.0,
            GameSound::TowerRailgun => 2200.0,
            GameSound::TowerChainLightning => 660.0,
            GameSound::TowerDestroyed => 82.0,
            GameSound::TowerRepaired => 550.0,
            GameSound::EnemyDeath => 220.0,
            GameSound::EnemyBruteAttack => 165.0,
            GameSound::ScrapDrop => 1760.0,
            GameSound::ScrapCollected => 1320.0,
            GameSound::WaveBossSpawn => 55.0,
            GameSound::WaveStart => 440.0,
            GameSound::GameOver => 165.0,
        }
    }

    /// Duration in milliseconds.
    pub const fn duration_ms(&self) -> u64 {
        match self {
            GameSound::TowerScrapgun => 80,
            GameSound::TowerExplosive => 250,
            GameSound::TowerRailgun => 50,
            GameSound::TowerChainLightning => 120,
            GameSound::TowerDestroyed => 300,
            GameSound::TowerRepaired => 150,
            GameSound::EnemyDeath => 150,
            GameSound::EnemyBruteAttack => 100,
            GameSound::ScrapDrop => 60,
            GameSound::ScrapCollected => 100,
            GameSound::WaveBossSpawn => 500,
            GameSound::WaveStart => 200,
            GameSound::GameOver => 400,
        }
    }

    /// Default playback volume (linear, 0.0..=1.0).
    pub const fn volume(&self) -> f32 {
        match self {
            GameSound::TowerScrapgun => 0.3,
            GameSound::TowerExplosive => 0.4,
            GameSound::TowerRailgun => 0.3,
            GameSound::TowerChainLightning => 0.3,
            GameSound::TowerDestroyed => 0.5,
            GameSound::TowerRepaired => 0.4,
            GameSound::EnemyDeath => 0.4,
            GameSound::EnemyBruteAttack => 0.3,
            GameSound::ScrapDrop => 0.25,
            GameSound::ScrapCollected => 0.2,
            GameSound::WaveBossSpawn => 0.6,
            GameSound::WaveStart => 0.5,
            GameSound::GameOver => 0.6,
        }
    }
}

/// Trigger this event to play a game sound.
///
/// The audio plugin's observer looks up the asset handle and spawns a
/// one-shot `AudioPlayer` entity that auto-despawns after playback.
#[derive(Event)]
pub struct PlaySound(pub GameSound);
