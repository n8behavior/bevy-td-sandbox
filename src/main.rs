use bevy::asset::AssetMetaCheck;
use bevy::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use bevy::window::{MonitorSelection, WindowMode};
use bevy_northstar::prelude::*;

mod camera;
mod common;
mod economy;
mod enemy;
mod grid;
mod pathfinding;
mod pile;
mod projectile;
mod shader;
mod states;
mod terrain;
mod tower;
mod ui;
mod wave;

use camera::CameraPlugin;
use economy::EconomyPlugin;
use enemy::EnemyPlugin;
use grid::GridPlugin;
use pathfinding::PathfindingPlugin;
use pile::PilePlugin;
use projectile::ProjectilePlugin;
use shader::ShaderPlugin;
use states::GameState;
use terrain::TerrainPlugin;
use tower::TowerPlugin;
use ui::UIPlugin;
use wave::WavePlugin;

fn main() {
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
        .init_state::<GameState>()
        .add_sub_state::<states::PlayPhase>()
        .add_plugins((
            ShaderPlugin,
            CameraPlugin,
            GridPlugin,
            PilePlugin,
            TerrainPlugin,
            PathfindingPlugin,
            TowerPlugin,
            EnemyPlugin,
            ProjectilePlugin,
            WavePlugin,
            EconomyPlugin,
            UIPlugin,
        ))
        .run();
}
