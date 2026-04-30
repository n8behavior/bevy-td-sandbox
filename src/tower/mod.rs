//! Tower lifecycle: placement, targeting, firing, upgrades, and visuals.
//!
//! Towers progress through `TowerState`: `Placing` (preview at cursor),
//! `Active` (operational on the grid), and `Rubble` (destroyed, repairable).
//!
//! Each tower type registers a `TowerBlueprint` via its plugin. Ring visuals
//! (range, slow-aura, collection-aura) are spawned reactively when the
//! corresponding config component (`RangeRingConfig`, `SlowAuraRingConfig`,
//! `CollectionAuraRingConfig`) is added to an entity.

pub mod chain_lightning;
pub mod components;
pub mod events;
pub mod explosive;
pub mod placement;
pub mod railgun;
pub mod scrap_gun;
pub mod scrap_magnet;
pub mod systems;
pub mod tar_pit;
pub mod targeting;
pub mod upgrade;

use crate::shader::{CircleMaterial, CircleMesh};
use crate::states::{GameState, PlayPhase};
use bevy::prelude::*;
use bevy::sprite_render::MeshMaterial2d;

use components::{
    AuraVisual, CollectionAuraRingConfig, MagnetAura, RangeRing, RangeRingConfig,
    SlowAuraRingConfig, TowerRegistry,
};

/// Spawn a shader-driven ring child on an entity.
fn spawn_ring_child<M: Component>(
    commands: &mut Commands,
    entity: Entity,
    marker: M,
    mesh: &CircleMesh,
    materials: &mut Assets<CircleMaterial>,
    range: f32,
    color: Color,
    z_offset: f32,
    material_fn: fn(Color) -> CircleMaterial,
) {
    let diameter = range * 2.0;
    let mat = materials.add(material_fn(color));
    commands.entity(entity).with_child((
        marker,
        Mesh2d(mesh.0.clone()),
        MeshMaterial2d(mat),
        Transform::from_translation(Vec3::new(0.0, 0.0, z_offset))
            .with_scale(Vec3::splat(diameter)),
    ));
}

/// Reactive system: spawn a range ring when `RangeRingConfig` is added.
fn spawn_range_rings(
    mut commands: Commands,
    query: Query<(Entity, &RangeRingConfig), Added<RangeRingConfig>>,
    circle_mesh: Res<CircleMesh>,
    mut materials: ResMut<Assets<CircleMaterial>>,
) {
    for (entity, config) in &query {
        spawn_ring_child(
            &mut commands,
            entity,
            RangeRing,
            &circle_mesh,
            &mut materials,
            config.range,
            config.color,
            -0.1,
            CircleMaterial::range_indicator,
        );
    }
}

/// Reactive system: spawn a slow-aura ring when `SlowAuraRingConfig` is added.
fn spawn_aura_rings(
    mut commands: Commands,
    query: Query<(Entity, &SlowAuraRingConfig), Added<SlowAuraRingConfig>>,
    circle_mesh: Res<CircleMesh>,
    mut materials: ResMut<Assets<CircleMaterial>>,
) {
    for (entity, config) in &query {
        spawn_ring_child(
            &mut commands,
            entity,
            AuraVisual,
            &circle_mesh,
            &mut materials,
            config.range,
            config.color,
            -0.2,
            CircleMaterial::aura,
        );
    }
}

/// Reactive system: spawn a collection-aura ring when
/// `CollectionAuraRingConfig` is added.
fn spawn_collection_aura_rings(
    mut commands: Commands,
    query: Query<(Entity, &CollectionAuraRingConfig), Added<CollectionAuraRingConfig>>,
    circle_mesh: Res<CircleMesh>,
    mut materials: ResMut<Assets<CircleMaterial>>,
) {
    for (entity, config) in &query {
        spawn_ring_child(
            &mut commands,
            entity,
            MagnetAura,
            &circle_mesh,
            &mut materials,
            config.range,
            config.color,
            -0.2,
            CircleMaterial::aura,
        );
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
            .init_resource::<upgrade::PanelSections>()
            .add_message::<upgrade::UpgradeApplied<upgrade::Primary>>()
            .add_message::<upgrade::UpgradeApplied<upgrade::Magnet>>()
            .add_observer(systems::default_fire_observer)
            .add_plugins((
                scrap_gun::ScrapGunPlugin,
                tar_pit::TarPitPlugin,
                explosive::ExplosivePlugin,
                railgun::RailgunPlugin,
                scrap_magnet::ScrapMagnetPlugin,
                chain_lightning::ChainLightningPlugin,
            ))
            .add_systems(Startup, upgrade::register_default_panel_sections)
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
                    targeting::handle_targeting_button,
                    upgrade::inspect_tower,
                    upgrade::apply_track_upgrade::<upgrade::Primary>,
                    upgrade::scale_turret_on_tier,
                    upgrade::scale_aoe_on_tier,
                    upgrade::scale_slow_on_tier,
                    upgrade::scale_health_on_tier,
                    upgrade::scale_primary_visuals_on_upgrade,
                    upgrade::sync_collector_on_upgrade,
                    upgrade::apply_track_upgrade::<upgrade::Magnet>,
                    upgrade::scale_magnet_on_track,
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
                    upgrade::rebuild_common_stats,
                    upgrade::update_upgrade_panel,
                    targeting::refresh_targeting_button_colors,
                    targeting::spawn_targeting_label,
                    targeting::update_targeting_label,
                    targeting::stabilize_targeting_labels,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_systems(
                Update,
                (
                    spawn_range_rings,
                    spawn_aura_rings,
                    spawn_collection_aura_rings,
                )
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
                (
                    systems::rotate_towers_to_target,
                    systems::scrap_magnet_collect,
                    systems::on_tower_becomes_rubble,
                    systems::update_tower_degradation_visual,
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}
