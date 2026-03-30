#![allow(clippy::too_many_arguments, clippy::type_complexity)]

use bevy::prelude::*;
use bevy::window::{MonitorSelection, WindowMode};
use bevy_northstar::prelude::*;

mod common;
mod economy;
mod enemy;
mod grid;
mod pathfinding;
mod projectile;
mod states;
mod tower;
mod ui;
mod wave;

use economy::EconomyPlugin;
use enemy::EnemyPlugin;
use grid::GridPlugin;
use pathfinding::PathfindingPlugin;
use projectile::ProjectilePlugin;
use states::GameState;
use tower::TowerPlugin;
use ui::UIPlugin;
use wave::WavePlugin;

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                        title: "Scrap Defence".into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(NorthstarPlugin::<CardinalNeighborhood>::default())
        .init_state::<GameState>()
        .add_sub_state::<states::PlayPhase>()
        .add_plugins((
            GridPlugin,
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
