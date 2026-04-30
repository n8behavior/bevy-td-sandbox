use std::time::Duration;

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::common::constants::{MAGNET_AURA_COLOR, TOWER_HP_COST_MULT};
use crate::tower::components::*;
use crate::tower::events::TowerFired;

const FIRE_HZ: f32 = 2200.0;
const FIRE_MS: u64 = 50;
const FIRE_VOLUME: f32 = 0.3;

#[derive(Component)]
pub struct Railgun;

#[derive(Resource)]
struct FireSound(Handle<Pitch>);

pub struct RailgunPlugin;

impl Plugin for RailgunPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, (register, init_fire_sound))
            .add_observer(on_tower_fired);
    }
}

fn init_fire_sound(mut commands: Commands, mut pitches: ResMut<Assets<Pitch>>) {
    let handle = pitches.add(Pitch::new(FIRE_HZ, Duration::from_millis(FIRE_MS)));
    commands.insert_resource(FireSound(handle));
}

fn on_tower_fired(
    trigger: On<TowerFired>,
    railguns: Query<(), With<Railgun>>,
    sound: Res<FireSound>,
    mut commands: Commands,
) {
    if railguns.contains(trigger.entity) {
        commands.spawn((
            AudioPlayer::<Pitch>(sound.0.clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(FIRE_VOLUME)),
        ));
    }
}

fn register(mut registry: ResMut<TowerRegistry>) {
    registry.blueprints.push(TowerBlueprint {
        name: "Railgun",
        cost: 150,
        color: Color::srgb(0.6, 0.65, 0.75),
        ui_color: Color::srgb(0.75, 0.8, 0.9),
        key: KeyCode::Digit4,
        special_label: "",
        spawn_fn: |cmds| {
            let damage = 50.0;
            let range = 160.0;
            let cooldown = 5.0;
            let aim_tolerance = 0.05;
            let color = Color::srgb(0.6, 0.65, 0.75);
            let collect_range = 30.0;
            cmds.insert((
                RangeRingConfig {
                    range,
                    color: Color::srgba(0.6, 0.6, 0.0, 0.15),
                },
                ScrapCollector {
                    range: collect_range,
                },
                Railgun,
                BlocksNav,
                TargetingMode::default(),
                Turret::new(Damage(damage), Range(range), cooldown, aim_tolerance),
                TowerColor(color),
                ProjectileVisuals {
                    speed: 2000.0,
                    color: Color::srgb(0.6, 0.8, 1.0),
                    size: Vec2::new(10.0, 4.0),
                    trail_color: Color::srgb(0.4, 0.7, 1.0),
                    trail_interval: 0.008,
                    particle_size: 6.0,
                    particle_lifetime: 0.4,
                },
                TowerTier(0),
                TowerName("Railgun"),
                PanelStats::default(),
                BaseCost(150),
            ));
            let max_hp = 150.0 * TOWER_HP_COST_MULT;
            cmds.insert(TowerHealth {
                current: max_hp,
                max: max_hp,
            });
            cmds.insert((
                MagnetTier(0),
                CollectionAuraRingConfig {
                    range: collect_range,
                    color: MAGNET_AURA_COLOR,
                },
            ));
        },
    });
}
