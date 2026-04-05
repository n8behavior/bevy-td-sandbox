use bevy::prelude::*;

#[derive(Resource)]
pub struct SoundAssets {
    pub tower_scrapgun: Handle<Pitch>,
    pub tower_explosive: Handle<Pitch>,
    pub tower_railgun: Handle<Pitch>,
    pub tower_chain_lightning: Handle<Pitch>,
    pub enemy_death: Handle<Pitch>,
    pub scrap_drop: Handle<Pitch>,
    pub scrap_collected: Handle<Pitch>,
    pub boss_spawn: Handle<Pitch>,
    pub wave_start: Handle<Pitch>,
    pub game_over: Handle<Pitch>,
}
