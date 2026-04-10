use bevy::prelude::*;

use super::events::GameSound;

#[derive(Resource)]
pub(super) struct SoundAssets {
    pub(super) tower_scrapgun: Handle<Pitch>,
    pub(super) tower_explosive: Handle<Pitch>,
    pub(super) tower_railgun: Handle<Pitch>,
    pub(super) tower_chain_lightning: Handle<Pitch>,
    pub(super) tower_destroyed: Handle<Pitch>,
    pub(super) tower_repaired: Handle<Pitch>,
    pub(super) enemy_death: Handle<Pitch>,
    pub(super) enemy_brute_attack: Handle<Pitch>,
    pub(super) scrap_drop: Handle<Pitch>,
    pub(super) scrap_collected: Handle<Pitch>,
    pub(super) wave_boss_spawn: Handle<Pitch>,
    pub(super) wave_start: Handle<Pitch>,
    pub(super) game_over: Handle<Pitch>,
}

impl SoundAssets {
    pub(super) fn get(&self, sound: GameSound) -> &Handle<Pitch> {
        match sound {
            GameSound::TowerScrapgun => &self.tower_scrapgun,
            GameSound::TowerExplosive => &self.tower_explosive,
            GameSound::TowerRailgun => &self.tower_railgun,
            GameSound::TowerChainLightning => &self.tower_chain_lightning,
            GameSound::TowerDestroyed => &self.tower_destroyed,
            GameSound::TowerRepaired => &self.tower_repaired,
            GameSound::EnemyDeath => &self.enemy_death,
            GameSound::EnemyBruteAttack => &self.enemy_brute_attack,
            GameSound::ScrapDrop => &self.scrap_drop,
            GameSound::ScrapCollected => &self.scrap_collected,
            GameSound::WaveBossSpawn => &self.wave_boss_spawn,
            GameSound::WaveStart => &self.wave_start,
            GameSound::GameOver => &self.game_over,
        }
    }
}
