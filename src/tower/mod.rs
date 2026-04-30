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
pub mod mortar;
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

// ---------------------------------------------------------------------------
// Per-capability lifecycle observers (TowerBecameRubble / TowerRepaired)
//
// Each ring class owns its own pair: an observer that despawns its visual
// children when the tower becomes rubble, and an observer that re-inserts
// the matching `*RingConfig` component when the tower is repaired (which
// re-triggers the reactive `Added<...RingConfig>` spawners above).
// ---------------------------------------------------------------------------

/// On `TowerBecameRubble`, despawn any `RangeRing` children of the tower so
/// the visual disappears while the tower is dead.
fn despawn_range_ring_on_rubble(
    trigger: On<events::TowerBecameRubble>,
    children_query: Query<&Children>,
    range_rings: Query<Entity, With<RangeRing>>,
    mut commands: Commands,
) {
    let Ok(children) = children_query.get(trigger.entity) else {
        return;
    };
    for child in children.iter() {
        if range_rings.contains(child) {
            commands.entity(child).despawn();
        }
    }
}

/// On `TowerBecameRubble`, despawn any slow-aura `AuraVisual` children.
fn despawn_aura_visual_on_rubble(
    trigger: On<events::TowerBecameRubble>,
    children_query: Query<&Children>,
    aura_visuals: Query<Entity, With<AuraVisual>>,
    mut commands: Commands,
) {
    let Ok(children) = children_query.get(trigger.entity) else {
        return;
    };
    for child in children.iter() {
        if aura_visuals.contains(child) {
            commands.entity(child).despawn();
        }
    }
}

/// On `TowerBecameRubble`, despawn any collection-aura `MagnetAura` children.
fn despawn_magnet_aura_on_rubble(
    trigger: On<events::TowerBecameRubble>,
    children_query: Query<&Children>,
    magnet_auras: Query<Entity, With<MagnetAura>>,
    mut commands: Commands,
) {
    let Ok(children) = children_query.get(trigger.entity) else {
        return;
    };
    for child in children.iter() {
        if magnet_auras.contains(child) {
            commands.entity(child).despawn();
        }
    }
}

/// On `TowerRepaired`, re-insert the tower's `RangeRingConfig` to retrigger
/// the reactive `spawn_range_rings` system.
fn reinsert_range_ring_on_repair(
    trigger: On<events::TowerRepaired>,
    configs: Query<&RangeRingConfig>,
    mut commands: Commands,
) {
    let Ok(config) = configs.get(trigger.entity) else {
        return;
    };
    let new = RangeRingConfig {
        range: config.range,
        color: config.color,
    };
    commands.entity(trigger.entity).remove::<RangeRingConfig>();
    commands.entity(trigger.entity).insert(new);
}

/// On `TowerRepaired`, re-insert the tower's `SlowAuraRingConfig` to
/// retrigger the reactive `spawn_aura_rings` system.
fn reinsert_aura_ring_on_repair(
    trigger: On<events::TowerRepaired>,
    configs: Query<&SlowAuraRingConfig>,
    mut commands: Commands,
) {
    let Ok(config) = configs.get(trigger.entity) else {
        return;
    };
    let new = SlowAuraRingConfig {
        range: config.range,
        color: config.color,
    };
    commands
        .entity(trigger.entity)
        .remove::<SlowAuraRingConfig>();
    commands.entity(trigger.entity).insert(new);
}

/// On `TowerRepaired`, re-insert the tower's `CollectionAuraRingConfig` to
/// retrigger the reactive `spawn_collection_aura_rings` system.
fn reinsert_collection_aura_on_repair(
    trigger: On<events::TowerRepaired>,
    configs: Query<&CollectionAuraRingConfig>,
    mut commands: Commands,
) {
    let Ok(config) = configs.get(trigger.entity) else {
        return;
    };
    let new = CollectionAuraRingConfig {
        range: config.range,
        color: config.color,
    };
    commands
        .entity(trigger.entity)
        .remove::<CollectionAuraRingConfig>();
    commands.entity(trigger.entity).insert(new);
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
            .add_observer(despawn_range_ring_on_rubble)
            .add_observer(despawn_aura_visual_on_rubble)
            .add_observer(despawn_magnet_aura_on_rubble)
            .add_observer(reinsert_range_ring_on_repair)
            .add_observer(reinsert_aura_ring_on_repair)
            .add_observer(reinsert_collection_aura_on_repair)
            .add_plugins((
                scrap_gun::ScrapGunPlugin,
                tar_pit::TarPitPlugin,
                explosive::ExplosivePlugin,
                railgun::RailgunPlugin,
                scrap_magnet::ScrapMagnetPlugin,
                chain_lightning::ChainLightningPlugin,
                mortar::MortarPlugin,
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
