use bevy::prelude::*;

#[derive(States, Default, PartialEq, Eq, Hash, Debug, Clone)]
pub enum GameState {
    #[default]
    MainMenu,
    Playing,
    /// The run has ended, won or lost. A Classic run is won when its last
    /// wave is beaten (`WaveManager::all_waves_cleared`); the end screen
    /// shows the outcome.
    GameOver,
}

#[derive(SubStates, Clone, PartialEq, Eq, Hash, Debug, Default)]
#[source(GameState = GameState::Playing)]
pub enum PlayPhase {
    #[default]
    Building,
    Defending,
}

#[derive(Resource, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameMode {
    #[default]
    Classic,
    Endless,
}
