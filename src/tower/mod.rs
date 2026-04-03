pub mod components;
pub mod placement;
pub mod systems;
pub mod types;

use bevy::prelude::*;
use bevy::asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use crate::states::{GameState, PlayPhase};

use components::{CircleImage, TowerRegistry};

fn create_circle_image(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let size: u32 = 128;
    let center = size as f32 / 2.0;
    let radius = center - 0.5;
    let mut data = vec![0u8; (size * size * 4) as usize];

    for y in 0..size {
        for x in 0..size {
            let dx = x as f32 + 0.5 - center;
            let dy = y as f32 + 0.5 - center;
            let dist = (dx * dx + dy * dy).sqrt();
            let idx = ((y * size + x) * 4) as usize;
            // Soft edge: smooth over ~1.5px at boundary.
            let alpha = ((radius - dist) / 1.5).clamp(0.0, 1.0);
            let a = (alpha * 255.0) as u8;
            data[idx] = 255;
            data[idx + 1] = 255;
            data[idx + 2] = 255;
            data[idx + 3] = a;
        }
    }

    let image = Image::new(
        Extent3d { width: size, height: size, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    commands.insert_resource(CircleImage(images.add(image)));
}

pub struct TowerPlugin;

impl Plugin for TowerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, create_circle_image)
            .init_resource::<placement::SelectedTower>()
            .init_resource::<TowerRegistry>()
            .add_plugins((
                types::ScrapGunPlugin,
                types::TarPitPlugin,
                types::ExplosivePlugin,
                types::RailgunPlugin,
            ))
            .add_systems(
                Update,
                (
                    placement::handle_tower_selection,
                    placement::update_placing_tower,
                    placement::tint_placing_tower,
                    placement::confirm_tower_placement,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (systems::turret_state_machine, systems::slow_aura)
                    .run_if(in_state(GameState::Playing))
                    .run_if(in_state(PlayPhase::Defending)),
            )
            .add_systems(
                Update,
                systems::rotate_towers_to_target
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
