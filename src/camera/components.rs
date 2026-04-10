use bevy::prelude::*;

/// Per-camera settings for zoom, pan, and reset behaviour.
#[derive(Component)]
pub struct CameraController {
    /// Minimum orthographic scale (most zoomed-in). Lower values = closer view.
    pub min_scale: f32,
    /// Maximum orthographic scale (most zoomed-out). Higher values = wider view.
    pub max_scale: f32,
    /// Multiplicative zoom factor per scroll notch (must be >1.0).
    /// Scroll-up divides by this (zoom in), scroll-down multiplies (zoom out).
    pub zoom_step: f32,
    /// World-space position restored by the Home-key reset.
    pub home_translation: Vec3,
    /// Orthographic scale restored by the Home-key reset.
    pub home_scale: f32,
}

/// Tracks middle-mouse drag state for camera panning.
#[derive(Resource, Default)]
pub struct PanState {
    /// Whether a pan drag is currently active.
    pub dragging: bool,
    /// Cursor world position from the previous frame (for delta calculation).
    pub last_world_pos: Vec2,
    /// Set when the cursor leaves the window; prevents a jump on return.
    pub cursor_absent: bool,
}

/// Global screen-shake state with exponential decay.
#[derive(Resource, Default)]
pub struct ScreenShake {
    /// Current shake magnitude in world units. Zero means inactive.
    pub intensity: f32,
    /// Remaining shake duration.
    pub timer: Timer,
    /// Exponential decay base applied per second (`intensity *= decay.powf(dt)`).
    pub decay: f32,
    /// Offset applied this frame, undone next frame to restore the logical position.
    pub current_offset: Vec3,
}
