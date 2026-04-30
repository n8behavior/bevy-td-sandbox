use std::time::Duration;

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::common::constants::{MAGNET_AURA_COLOR, TOWER_HP_COST_MULT};
use crate::tower::components::*;
use crate::tower::events::TowerFired;

const FIRE_HZ: f32 = 880.0;
const FIRE_MS: u64 = 80;
const FIRE_VOLUME: f32 = 0.3;

#[derive(Component)]
pub struct ScrapGun;

#[derive(Resource)]
struct FireSound(Handle<Pitch>);

pub struct ScrapGunPlugin;

impl Plugin for ScrapGunPlugin {
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
    scrap_guns: Query<(), With<ScrapGun>>,
    sound: Res<FireSound>,
    mut commands: Commands,
) {
    if scrap_guns.contains(trigger.entity) {
        commands.spawn((
            AudioPlayer::<Pitch>(sound.0.clone()),
            PlaybackSettings::DESPAWN.with_volume(Volume::Linear(FIRE_VOLUME)),
        ));
    }
}

fn register(mut registry: ResMut<TowerRegistry>) {
    registry.blueprints.push(TowerBlueprint {
        name: "ScrapGun",
        cost: 50,
        color: Color::srgb(0.7, 0.7, 0.3),
        ui_color: Color::srgb(0.95, 0.9, 0.4),
        key: KeyCode::Digit1,
        special_label: "",
        spawn_fn: |cmds| {
            let damage = 10.0;
            let range = 80.0;
            let cooldown = 1.0;
            let aim_tolerance = 0.15;
            let color = Color::srgb(0.7, 0.7, 0.3);
            let collect_range = 30.0;
            cmds.insert((
                RangeRingConfig {
                    range,
                    color: Color::srgba(0.6, 0.6, 0.0, 0.15),
                },
                ScrapCollector {
                    range: collect_range,
                },
                ScrapGun,
                BlocksNav,
                TargetingMode::default(),
                Turret::new(Damage(damage), Range(range), cooldown, aim_tolerance),
                TowerColor(color),
                ProjectileVisuals {
                    speed: 200.0,
                    color: Color::srgb(1.0, 1.0, 0.6),
                    size: Vec2::splat(6.0),
                    trail_color: Color::srgb(1.0, 1.0, 0.4),
                    trail_interval: 0.03,
                    particle_size: 4.0,
                    particle_lifetime: 0.2,
                },
                TowerTier(0),
                TowerName("ScrapGun"),
                PanelStats::default(),
                BaseCost(50),
            ));
            let max_hp = 50.0 * TOWER_HP_COST_MULT;
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
