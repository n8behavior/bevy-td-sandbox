//! Mortar — heavy artillery turret with a unique reload phase.
//!
//! Mortar is the issue #81 acceptance scaffold: a tower added entirely
//! inside `src/tower/mortar/` plus one line in `tower/mod.rs`. It exercises
//! every extension point introduced by the refactor:
//!
//! - **Unique behavioral phase**: `Reloading` holds the turret in `Idle`
//!   between shots via `block_mortar_during_reload`.
//! - **Unique stat**: `ReloadSecs` is the live reload duration.
//! - **Unique upgrade dimension**: `ReloadKind` (key L) shrinks the reload
//!   time via `apply_track_upgrade::<ReloadKind>` + `scale_reload_on_track`.
//! - **Custom-fire observer**: `mortar_fire_observer` runs on
//!   `TowerWantsToFire` and inserts `Reloading` after spawning the
//!   projectile. Mortars carry the `CustomFire` marker so the default fire
//!   observer skips them.
//! - **Panel section**: `render_mortar_section` adds the `[L] Reload`
//!   button to the upgrade panel.

pub mod components;
pub mod systems;

use std::time::Duration;

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::common::constants::{MAGNET_AURA_COLOR, TOWER_HP_COST_MULT};
use crate::states::{GameState, PlayPhase};
use crate::tower::components::*;
use crate::tower::events::TowerFired;
use crate::tower::upgrade::{Magnet, Primary, UpgradeApplied, UpgradeTrack, apply_track_upgrade};

use components::{Mortar, ReloadKind, ReloadSecs};

const FIRE_HZ: f32 = 165.0;
const FIRE_MS: u64 = 320;
const FIRE_VOLUME: f32 = 0.45;

#[derive(Resource)]
struct FireSound(Handle<Pitch>);

pub struct MortarPlugin;

impl Plugin for MortarPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<UpgradeApplied<ReloadKind>>()
            .add_systems(
                Startup,
                (register, init_fire_sound, systems::register_mortar_section),
            )
            .add_observer(systems::mortar_fire_observer)
            .add_observer(on_tower_fired)
            .add_systems(
                FixedUpdate,
                systems::block_mortar_during_reload
                    .before(crate::tower::systems::turret_state_machine)
                    .run_if(in_state(GameState::Playing))
                    .run_if(in_state(PlayPhase::Defending)),
            )
            .add_systems(
                Update,
                (
                    systems::tick_reload,
                    (
                        apply_track_upgrade::<ReloadKind>,
                        systems::scale_reload_on_track,
                    )
                        .chain(),
                )
                    .run_if(in_state(GameState::Playing)),
            );
    }
}

fn init_fire_sound(mut commands: Commands, mut pitches: ResMut<Assets<Pitch>>) {
    let handle = pitches.add(Pitch::new(FIRE_HZ, Duration::from_millis(FIRE_MS)));
    commands.insert_resource(FireSound(handle));
}

fn on_tower_fired(
    trigger: On<TowerFired>,
    mortars: Query<(), With<Mortar>>,
    sound: Res<FireSound>,
    mut commands: Commands,
) {
    if mortars.contains(trigger.entity) {
        commands.spawn((
            AudioPlayer::<Pitch>(sound.0.clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(FIRE_VOLUME)),
        ));
    }
}

fn register(mut registry: ResMut<TowerRegistry>) {
    registry.blueprints.push(TowerBlueprint {
        name: "Mortar",
        cost: 200,
        color: Color::srgb(0.55, 0.4, 0.25),
        ui_color: Color::srgb(0.85, 0.6, 0.35),
        key: KeyCode::Digit7,
        special_label: "RELOAD",
        spawn_fn: |cmds| {
            let damage = 35.0;
            let range = 130.0;
            let cooldown = 0.5;
            let aim_tolerance = 0.18;
            let aoe_radius = 90.0;
            let aoe_damage = 35.0;
            let reload_secs = 3.0;
            let color = Color::srgb(0.55, 0.4, 0.25);
            let collect_range = 30.0;
            cmds.insert((
                RangeRingConfig {
                    range,
                    color: Color::srgba(0.85, 0.55, 0.2, 0.15),
                },
                ScrapCollector {
                    range: collect_range,
                },
                Mortar,
                CustomFire,
                BlocksNav,
                TargetingMode::default(),
                Turret::new(Damage(damage), Range(range), cooldown, aim_tolerance),
                TowerColor(color),
                ProjectileVisuals {
                    speed: 220.0,
                    color: Color::srgb(0.95, 0.7, 0.4),
                    size: Vec2::splat(8.0),
                    trail_color: Color::srgb(0.85, 0.55, 0.25),
                    trail_interval: 0.04,
                    particle_size: 5.0,
                    particle_lifetime: 0.35,
                },
                AoEOnHit {
                    radius: aoe_radius,
                    damage: aoe_damage,
                },
                ReloadSecs(reload_secs),
            ));
            cmds.insert((
                UpgradeTrack::<Primary>::default(),
                UpgradeTrack::<ReloadKind>::default(),
                TowerName("Mortar"),
                PanelStats::default(),
                BaseCost(200),
            ));
            let max_hp = 200.0 * TOWER_HP_COST_MULT;
            cmds.insert(TowerHealth {
                current: max_hp,
                max: max_hp,
            });
            cmds.insert((
                UpgradeTrack::<Magnet>::default(),
                CollectionAuraRingConfig {
                    range: collect_range,
                    color: MAGNET_AURA_COLOR,
                },
            ));
        },
    });
}
