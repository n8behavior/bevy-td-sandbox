use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::window::{MonitorSelection, WindowMode};
use bevy_northstar::prelude::*;

pub mod audio;
pub mod camera;
pub mod common;
pub mod economy;
pub mod endless;
pub mod enemy;
pub mod grid;
pub mod particles;
pub mod pathfinding;
pub mod pile;
pub mod projectile;
pub mod shader;
pub mod states;
pub mod stats;
pub mod terrain;
pub mod tower;
pub mod ui;
pub mod wave;

pub mod test_helpers;

use audio::GameAudioPlugin;
use camera::CameraPlugin;
use economy::EconomyPlugin;
use endless::EndlessPlugin;
use enemy::EnemyPlugin;
use grid::GridPlugin;
use particles::ParticlesPlugin;
use pathfinding::PathfindingPlugin;
use pile::PilePlugin;
use projectile::ProjectilePlugin;
use shader::ShaderPlugin;
use states::{GameMode, GameState};
use stats::StatsPlugin;
use terrain::TerrainPlugin;
use tower::TowerPlugin;
use ui::UIPlugin;
use wave::WavePlugin;

pub fn run() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Scrap Defence".into(),
                        #[cfg(not(target_arch = "wasm32"))]
                        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(NorthstarPlugin::<OrdinalNeighborhood>::default())
        .init_resource::<GameMode>()
        .init_state::<GameState>()
        .add_sub_state::<states::PlayPhase>()
        .add_plugins((
            ShaderPlugin,
            GameAudioPlugin,
            CameraPlugin,
            GridPlugin,
            PilePlugin,
            TerrainPlugin,
            PathfindingPlugin,
            TowerPlugin,
            EnemyPlugin,
            ProjectilePlugin,
            WavePlugin,
            EndlessPlugin,
            EconomyPlugin,
            StatsPlugin,
            UIPlugin,
        ))
        .add_plugins(ParticlesPlugin)
        .run();
}
