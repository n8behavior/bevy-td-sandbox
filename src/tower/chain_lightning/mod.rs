pub mod components;
pub mod systems;

use std::time::Duration;

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::common::constants::{MAGNET_AURA_COLOR, TOWER_HP_COST_MULT};
use crate::states::{GameState, PlayPhase};
use crate::tower::components::*;
use crate::tower::events::TowerFired;
use crate::tower::upgrade::{Magnet, UpgradeTrack};

use components::{ChainCooldown, ChainLightning, ChainLightningMarker};

const FIRE_HZ: f32 = 660.0;
const FIRE_MS: u64 = 120;
const FIRE_VOLUME: f32 = 0.3;

#[derive(Resource)]
struct FireSound(Handle<Pitch>);

pub struct ChainLightningPlugin;

impl Plugin for ChainLightningPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (register, init_fire_sound))
            .add_observer(on_tower_fired)
            .add_systems(
                FixedUpdate,
                systems::chain_lightning_fire
                    .run_if(in_state(GameState::Playing))
                    .run_if(in_state(PlayPhase::Defending)),
            )
            .add_systems(
                Update,
                (
                    systems::animate_lightning_arcs,
                    systems::scale_chain_on_tier_change,
                    systems::rebuild_chain_panel_stats
                        .after(crate::tower::upgrade::rebuild_common_stats),
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
    chain_lightnings: Query<(), With<ChainLightningMarker>>,
    sound: Res<FireSound>,
    mut commands: Commands,
) {
    if chain_lightnings.contains(trigger.entity) {
        commands.spawn((
            AudioPlayer::<Pitch>(sound.0.clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(FIRE_VOLUME)),
        ));
    }
}

fn register(mut registry: ResMut<TowerRegistry>) {
    registry.blueprints.push(TowerBlueprint {
        name: "Chain Lightning",
        cost: 125,
        color: Color::srgb(0.3, 0.6, 1.0),
        ui_color: Color::srgb(0.4, 0.7, 1.0),
        key: KeyCode::Digit6,
        special_label: "Chain",
        spawn_fn: |cmds| {
            let damage = 20.0;
            let range = 90.0;
            let cooldown = 2.0;
            let arc_range = 60.0;
            let color = Color::srgb(0.3, 0.6, 1.0);
            let collect_range = 30.0;
            cmds.insert((
                RangeRingConfig {
                    range,
                    color: Color::srgba(0.3, 0.5, 1.0, 0.15),
                },
                ScrapCollector {
                    range: collect_range,
                },
                ChainLightningMarker,
                BlocksNav,
                TargetingMode::default(),
                TowerColor(color),
                ChainLightning {
                    damage: Damage(damage),
                    primary_range: Range(range),
                    arc_range,
                    damage_falloff: 0.7,
                },
                ChainCooldown::new(cooldown),
                TowerTier(0),
                TowerName("Chain Lightning"),
                PanelStats::default(),
                BaseCost(125),
            ));
            let max_hp = 125.0 * TOWER_HP_COST_MULT;
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
