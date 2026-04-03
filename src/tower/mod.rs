pub mod components;
pub mod placement;
pub mod systems;
pub mod types;

use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;
use crate::shader::{CircleMaterial, CircleMesh};
use crate::states::{GameState, PlayPhase};

use components::{
    AuraRingConfig, AuraVisual, RangeRing, RangeRingConfig, TowerRegistry,
};

/// Reactive system: when a tower entity gets a `RangeRingConfig`, spawn the
/// shader-driven range ring as a child.
fn spawn_range_rings(
    mut commands: Commands,
    query: Query<(Entity, &RangeRingConfig), Added<RangeRingConfig>>,
    circle_mesh: Res<CircleMesh>,
    mut materials: ResMut<Assets<CircleMaterial>>,
) {
    for (entity, config) in &query {
        let diameter = config.range * 2.0;
        let mat = materials.add(CircleMaterial {
            color: config.color,
            softness: 0.05,
            fill_fade: 0.0,
            ripple_speed: 0.0,
            time: 0.0,
        });
        commands.entity(entity).with_child((
            RangeRing,
            Mesh2d(circle_mesh.0.clone()),
            MeshMaterial2d(mat),
            Transform::from_translation(Vec3::new(0.0, 0.0, -0.1))
                .with_scale(Vec3::splat(diameter)),
        ));
    }
}

/// Reactive system: when a tower entity gets an `AuraRingConfig`, spawn
/// gradient aura ring children.
fn spawn_aura_rings(
    mut commands: Commands,
    query: Query<(Entity, &AuraRingConfig), Added<AuraRingConfig>>,
    circle_mesh: Res<CircleMesh>,
    mut materials: ResMut<Assets<CircleMaterial>>,
) {
    for (entity, config) in &query {
        let diameter = config.range * 2.0;
        let mat = materials.add(CircleMaterial {
            color: Color::srgba(0.4, 0.15, 0.45, 0.55),
            softness: 0.05,
            fill_fade: 1.0,
            ripple_speed: 0.4,
            time: 0.0,
        });
        commands.entity(entity).with_child((
            AuraVisual,
            Mesh2d(circle_mesh.0.clone()),
            MeshMaterial2d(mat),
            Transform::from_translation(Vec3::new(0.0, 0.0, -0.2))
                .with_scale(Vec3::splat(diameter)),
        ));
    }
}

/// Sort tower blueprints by key so the menu order matches key assignments,
/// regardless of which Startup `register` system ran first.
fn sort_blueprints(mut registry: ResMut<TowerRegistry>) {
    registry.blueprints.sort_by_key(|b| match b.key {
        KeyCode::Digit1 => 1,
        KeyCode::Digit2 => 2,
        KeyCode::Digit3 => 3,
        KeyCode::Digit4 => 4,
        KeyCode::Digit5 => 5,
        KeyCode::Digit6 => 6,
        KeyCode::Digit7 => 7,
        KeyCode::Digit8 => 8,
        KeyCode::Digit9 => 9,
        _ => 99,
    });
}

pub struct TowerPlugin;

impl Plugin for TowerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<placement::SelectedTower>()
            .init_resource::<TowerRegistry>()
            .add_plugins((
                types::ScrapGunPlugin,
                types::TarPitPlugin,
                types::ExplosivePlugin,
                types::RailgunPlugin,
            ))
            .add_systems(PostStartup, sort_blueprints)
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
                Update,
                (spawn_range_rings, spawn_aura_rings)
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
