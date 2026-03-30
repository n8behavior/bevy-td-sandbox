use bevy::prelude::*;

#[derive(Component)]
pub struct CameraController {
    pub min_scale: f32,
    pub max_scale: f32,
    /// Multiplicative zoom factor per scroll notch.
    pub zoom_step: f32,
    /// World-space translation when zoom is at default (for reset).
    pub home_translation: Vec3,
    pub home_scale: f32,
}

#[derive(Resource, Default)]
pub struct PanState {
    pub dragging: bool,
    pub last_world_pos: Vec2,
}

#[derive(Resource, Default)]
pub struct ScreenShake {
    pub intensity: f32,
    pub timer: Timer,
    pub decay: f32,
}
