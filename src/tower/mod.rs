pub mod components;
pub mod placement;
pub mod systems;
pub mod targeting;
pub mod types;
pub mod upgrade;

use crate::shader::{CircleMaterial, CircleMesh};
use crate::states::{GameState, PlayPhase};
use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;

use components::{
    AuraRingConfig, AuraVisual, MagnetAura, MagnetAuraConfig, RangeRing, RangeRingConfig,
    TowerRegistry,
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
            color: config.color,
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

/// Reactive system: when a tower entity gets a `MagnetAuraConfig`, spawn
/// a collection aura ring as a child. Separate from `AuraRingConfig` so
/// towers can have both a gameplay aura and a collection aura.
fn spawn_magnet_aura_rings(
    mut commands: Commands,
    query: Query<(Entity, &MagnetAuraConfig), Added<MagnetAuraConfig>>,
    circle_mesh: Res<CircleMesh>,
    mut materials: ResMut<Assets<CircleMaterial>>,
) {
    for (entity, config) in &query {
        let diameter = config.range * 2.0;
        let mat = materials.add(CircleMaterial {
            color: config.color,
            softness: 0.05,
            fill_fade: 1.0,
            ripple_speed: 0.4,
            time: 0.0,
        });
        commands.entity(entity).with_child((
            MagnetAura,
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
            .init_resource::<upgrade::InspectedTower>()
            .init_resource::<targeting::RadialMenuState>()
            .add_plugins((
                types::ScrapGunPlugin,
                types::TarPitPlugin,
                types::ExplosivePlugin,
                types::RailgunPlugin,
                types::ScrapMagnetPlugin,
                types::ChainLightningPlugin,
            ))
            .add_systems(PostStartup, sort_blueprints)
            .add_systems(OnEnter(GameState::Playing), upgrade::setup_upgrade_panel)
            .add_systems(
                Update,
                (
                    placement::handle_tower_selection,
                    placement::update_placing_tower,
                    placement::tint_placing_tower,
                    placement::confirm_tower_placement,
                    placement::sell_tower,
                    targeting::handle_radial_click,
                    upgrade::inspect_tower,
                    upgrade::apply_upgrade,
                    upgrade::sync_collector_on_upgrade,
                    upgrade::apply_magnet_upgrade,
                    upgrade::apply_repair,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    placement::animate_sell_text,
                    upgrade::animate_upgrade_flash,
                    upgrade::manage_selection_ring,
                    upgrade::update_upgrade_panel,
                    targeting::spawn_radial_menu,
                    targeting::highlight_radial_segments,
                    targeting::spawn_targeting_label,
                    targeting::update_targeting_label,
                    targeting::stabilize_targeting_labels,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (spawn_range_rings, spawn_aura_rings, spawn_magnet_aura_rings)
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                FixedUpdate,
                (
                    systems::turret_state_machine,
                    systems::slow_aura,
                    systems::magnetic_pull_enemies,
                    systems::chain_lightning_fire,
                )
                    .run_if(in_state(GameState::Playing))
                    .run_if(in_state(PlayPhase::Defending)),
            )
            .add_systems(
                Update,
                (
                    systems::rotate_towers_to_target,
                    systems::scrap_magnet_collect,
                    systems::animate_lightning_arcs,
                    systems::on_tower_becomes_rubble,
                    systems::update_tower_degradation_visual,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
