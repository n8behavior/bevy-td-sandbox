use bevy::prelude::*;

use super::components::*;

// ---------------------------------------------------------------------------
// Resources & Components
// ---------------------------------------------------------------------------

/// Tracks whether the radial targeting menu is open and for which tower.
#[derive(Resource, Default)]
pub struct RadialMenuState {
    pub tower: Option<Entity>,
}

/// Marker for a radial menu segment entity (world-space, NOT a tower child).
#[derive(Component)]
pub struct RadialSegment {
    pub mode: TargetingMode,
}

// ---------------------------------------------------------------------------
// Radial menu interaction
// ---------------------------------------------------------------------------

/// Cardinal offsets from tower center for each segment (matches TargetingMode::ALL order).
const SEGMENT_OFFSETS: [Vec2; 4] = [
    Vec2::new(0.0, 22.0),  // top: Closest
    Vec2::new(22.0, 0.0),  // right: HighestHp
    Vec2::new(0.0, -22.0), // bottom: FurthestAlongPath
    Vec2::new(-22.0, 0.0), // left: LowestHp
];

const SEGMENT_HIT_RADIUS: f32 = 12.0;
const SEGMENT_SIZE: f32 = 18.0;

const ACTIVE_COLOR: Color = Color::srgba(0.3, 0.7, 0.3, 0.8);
const INACTIVE_COLOR: Color = Color::srgba(0.15, 0.15, 0.15, 0.7);
const HOVER_COLOR: Color = Color::srgba(0.5, 0.5, 0.2, 0.8);

/// Process clicks when the radial menu is open. Runs BEFORE inspect_tower in
/// the chained Update set so it can consume the click.
pub fn handle_radial_click(
    mut radial_menu: ResMut<RadialMenuState>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    segments: Query<(&Transform, &RadialSegment)>,
    mut towers: Query<&mut TargetingMode, With<Tower>>,
) {
    let Some(tower_entity) = radial_menu.tower else {
        return;
    };

    if !mouse.just_pressed(MouseButton::Left) {
        return;
    }

    let Ok(window) = windows.single() else { return };
    let Ok((camera, cam_transform)) = cameras.single() else {
        return;
    };
    let Some(cursor_pos) = window.cursor_position() else {
        radial_menu.tower = None;
        return;
    };
    let Ok(world_pos) = camera.viewport_to_world_2d(cam_transform, cursor_pos) else {
        radial_menu.tower = None;
        return;
    };

    // Check if any segment was clicked.
    let mut hit_mode = None;
    for (tf, segment) in &segments {
        let seg_pos = tf.translation.truncate();
        if world_pos.distance(seg_pos) <= SEGMENT_HIT_RADIUS {
            hit_mode = Some(segment.mode);
            break;
        }
    }

    if let Some(mode) = hit_mode
        && let Ok(mut targeting) = towers.get_mut(tower_entity)
    {
        *targeting = mode;
    }

    // Close menu whether we hit a segment or not.
    radial_menu.tower = None;
}

// ---------------------------------------------------------------------------
// Radial menu visuals
// ---------------------------------------------------------------------------

/// Spawn / despawn radial menu segments when RadialMenuState changes.
pub fn spawn_radial_menu(
    mut commands: Commands,
    radial_menu: Res<RadialMenuState>,
    towers: Query<(&Transform, &TargetingMode), With<Tower>>,
    existing: Query<Entity, With<RadialSegment>>,
) {
    if !radial_menu.is_changed() {
        return;
    }

    // Despawn old segments.
    for e in &existing {
        commands.entity(e).despawn();
    }

    let Some(tower_entity) = radial_menu.tower else {
        return;
    };
    let Ok((tower_tf, current_mode)) = towers.get(tower_entity) else {
        return;
    };
    let center = tower_tf.translation.truncate();

    for (i, &mode) in TargetingMode::ALL.iter().enumerate() {
        let pos = center + SEGMENT_OFFSETS[i];
        let bg_color = if mode == *current_mode {
            ACTIVE_COLOR
        } else {
            INACTIVE_COLOR
        };

        commands
            .spawn((
                RadialSegment { mode },
                Sprite::from_color(bg_color, Vec2::splat(SEGMENT_SIZE)),
                Transform::from_translation(pos.extend(10.0)),
            ))
            .with_child((
                Text2d::new(mode.label()),
                TextFont {
                    font_size: 10.0,
                    ..default()
                },
                TextColor(Color::WHITE),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.1)),
            ));
    }
}

/// Highlight segments on mouse hover.
pub fn highlight_radial_segments(
    radial_menu: Res<RadialMenuState>,
    windows: Query<&Window>,
    cameras: Query<(&Camera, &GlobalTransform)>,
    towers: Query<&TargetingMode, With<Tower>>,
    mut segments: Query<(&Transform, &RadialSegment, &mut Sprite)>,
) {
    if radial_menu.tower.is_none() {
        return;
    }

    let current_mode = radial_menu
        .tower
        .and_then(|e| towers.get(e).ok())
        .copied()
        .unwrap_or_default();

    let cursor_world = (|| {
        let window = windows.single().ok()?;
        let (camera, cam_tf) = cameras.single().ok()?;
        let cursor = window.cursor_position()?;
        camera.viewport_to_world_2d(cam_tf, cursor).ok()
    })();

    for (tf, segment, mut sprite) in &mut segments {
        let is_active = segment.mode == current_mode;
        let is_hovered = cursor_world
            .is_some_and(|wp| wp.distance(tf.translation.truncate()) <= SEGMENT_HIT_RADIUS);

        sprite.color = if is_hovered && !is_active {
            HOVER_COLOR
        } else if is_active {
            ACTIVE_COLOR
        } else {
            INACTIVE_COLOR
        };
    }
}

// ---------------------------------------------------------------------------
// Targeting mode label on tower
// ---------------------------------------------------------------------------

/// Spawn a Text2d label on towers that just received a TargetingMode component.
pub fn spawn_targeting_label(
    mut commands: Commands,
    query: Query<(Entity, &TargetingMode), Added<TargetingMode>>,
) {
    for (entity, mode) in &query {
        commands.entity(entity).with_child((
            TargetingModeLabel,
            Text2d::new(mode.label()),
            TextFont {
                font_size: 10.0,
                ..default()
            },
            TextColor(Color::srgba(1.0, 1.0, 1.0, 0.8)),
            Transform::from_translation(Vec3::new(0.0, 0.0, 1.0)),
        ));
    }
}

/// Update the label text when targeting mode changes.
pub fn update_targeting_label(
    towers: Query<(&TargetingMode, &Children), Changed<TargetingMode>>,
    mut labels: Query<&mut Text2d, With<TargetingModeLabel>>,
) {
    for (mode, children) in &towers {
        for child in children.iter() {
            if let Ok(mut text) = labels.get_mut(child) {
                **text = mode.label().to_string();
            }
        }
    }
}

/// Counter-rotate targeting labels so they stay upright regardless of turret rotation.
pub fn stabilize_targeting_labels(
    towers: Query<(&Transform, &Children), (With<Tower>, With<TargetingMode>)>,
    mut labels: Query<&mut Transform, (With<TargetingModeLabel>, Without<Tower>)>,
) {
    for (tower_tf, children) in &towers {
        for child in children.iter() {
            if let Ok(mut label_tf) = labels.get_mut(child) {
                label_tf.rotation = tower_tf.rotation.inverse();
            }
        }
    }
}
