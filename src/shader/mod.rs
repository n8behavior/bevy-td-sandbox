//! GPU-driven SDF circle rendering for range rings and aura effects.
//!
//! [`CircleMaterial`] is a [`Material2d`] asset whose fragment shader
//! (`circle_sdf.wgsl`) evaluates a signed-distance field per pixel to draw
//! soft-edged circles with optional radial fade and ripple animation.
//! `tick_circle_materials` advances `time` each frame for materials with
//! positive `ripple_speed`. [`CircleMesh`] provides a shared unit-circle
//! mesh handle — scale via [`Transform`] for the desired diameter.

use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, AsBindGroupShaderType, ShaderType};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

/// Default edge softness for SDF circles.
/// Provides slight anti-alias feathering without visible blurring.
pub const DEFAULT_SOFTNESS: f32 = 0.05;

// ANCHOR: circle_material
/// GPU-driven circle using a signed distance field fragment shader.
#[derive(Asset, AsBindGroup, Clone, Debug, Reflect)]
#[uniform(0, CircleMaterialUniform)]
pub struct CircleMaterial {
    pub color: Color,
    /// Edge smoothing width. See [`DEFAULT_SOFTNESS`] for the standard value.
    /// 0.0 = hard edge, larger = softer falloff.
    pub softness: f32,
    /// 0.0 = uniform fill, 1.0 = radial gradient (fades center→edge).
    pub fill_fade: f32,
    /// Ripple wave speed (0.0 = off, 1.0 = one pulse per second).
    pub ripple_speed: f32,
    /// Elapsed time in seconds — updated by `tick_circle_materials` each frame.
    pub time: f32,
}
// ANCHOR_END: circle_material

// ANCHOR: constructors
impl CircleMaterial {
    /// Hard-edged ring with no fill or ripple. Used for tower range indicators.
    pub fn range_indicator(color: Color) -> Self {
        Self {
            color,
            softness: DEFAULT_SOFTNESS,
            fill_fade: 0.0,
            ripple_speed: 0.0,
            time: 0.0,
        }
    }

    /// Radial-gradient ring with ripple pulse. Used for aura effects.
    ///
    /// ```
    /// use bevy::prelude::*;
    /// use bevy_td_sandbox::shader::{CircleMaterial, DEFAULT_SOFTNESS};
    ///
    /// let mat = CircleMaterial::aura(Color::WHITE);
    /// assert_eq!(mat.softness, DEFAULT_SOFTNESS);
    /// assert_eq!(mat.fill_fade, 1.0);
    /// assert_eq!(mat.ripple_speed, 0.4);
    /// ```
    pub fn aura(color: Color) -> Self {
        Self {
            color,
            softness: DEFAULT_SOFTNESS,
            fill_fade: 1.0,
            ripple_speed: 0.4,
            time: 0.0,
        }
    }
}
// ANCHOR_END: constructors

// ANCHOR: material2d_impl
/// Loads the `circle_sdf.wgsl` embedded fragment shader and enables alpha
/// blending so the SDF edge falloff composites correctly against the scene.
impl Material2d for CircleMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_td_sandbox/shader/circle_sdf.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}
// ANCHOR_END: material2d_impl

/// GPU uniform layout matching the WGSL struct.
#[derive(Clone, Default, ShaderType)]
struct CircleMaterialUniform {
    color: Vec4,
    softness: f32,
    fill_fade: f32,
    ripple_speed: f32,
    time: f32,
}

// ANCHOR: bind_group
impl AsBindGroupShaderType<CircleMaterialUniform> for CircleMaterial {
    fn as_bind_group_shader_type(
        &self,
        _images: &bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>,
    ) -> CircleMaterialUniform {
        CircleMaterialUniform {
            color: bevy::color::LinearRgba::from(self.color).to_vec4(),
            softness: self.softness,
            fill_fade: self.fill_fade,
            ripple_speed: self.ripple_speed,
            time: self.time,
        }
    }
}
// ANCHOR_END: bind_group

/// Shared mesh handle for a unit circle (radius 0.5, diameter 1.0).
/// Scale via Transform to get the desired diameter.
#[derive(Resource)]
pub struct CircleMesh(pub Handle<Mesh>);

/// Startup system: creates a shared [`CircleMesh`] resource containing a unit
/// circle (radius 0.5). All SDF circle entities clone this handle and scale
/// via [`Transform`] to achieve the desired diameter.
fn setup_circle_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(CircleMesh(meshes.add(Circle::new(0.5))));
}

// ANCHOR: tick_system
/// Advance `time` on all circle materials that have a positive ripple speed.
///
/// Materials with `ripple_speed == 0.0` are static (e.g., range indicators)
/// and gain nothing from time advancement. Negative speeds are treated as
/// disabled rather than reversed, because the WGSL shader's `fract()` call
/// would wrap incorrectly for negative time products.
fn tick_circle_materials(mut materials: ResMut<Assets<CircleMaterial>>, time: Res<Time>) {
    for (_, mat) in materials.iter_mut() {
        if mat.ripple_speed > 0.0 {
            mat.time += time.delta_secs();
        }
    }
}
// ANCHOR_END: tick_system

// ANCHOR: plugin
/// Bevy plugin that registers the SDF circle rendering pipeline.
///
/// Adds [`Material2dPlugin<CircleMaterial>`], inserts the shared [`CircleMesh`]
/// resource at startup, and runs `tick_circle_materials` each frame to
/// advance ripple animations.
pub struct ShaderPlugin;

impl Plugin for ShaderPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "circle_sdf.wgsl");
        app.add_plugins(Material2dPlugin::<CircleMaterial>::default())
            .add_systems(Startup, setup_circle_mesh)
            .add_systems(Update, tick_circle_materials);
    }
}
// ANCHOR_END: plugin

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::test_app_with_assets;
    use std::time::Duration;

    // -- Constructor tests (#50) ------------------------------------------

    #[test]
    fn range_indicator_field_values() {
        let mat = CircleMaterial::range_indicator(Color::WHITE);
        assert_eq!(mat.softness, DEFAULT_SOFTNESS);
        assert_eq!(mat.fill_fade, 0.0);
        assert_eq!(mat.ripple_speed, 0.0);
        assert_eq!(mat.time, 0.0);
    }

    #[test]
    fn aura_field_values() {
        let mat = CircleMaterial::aura(Color::WHITE);
        assert_eq!(mat.softness, DEFAULT_SOFTNESS);
        assert_eq!(mat.fill_fade, 1.0);
        assert_eq!(mat.ripple_speed, 0.4);
        assert_eq!(mat.time, 0.0);
    }

    // -- Color conversion (#49) -------------------------------------------

    #[test]
    fn srgb_to_linear_conversion() {
        // The AsBindGroupShaderType impl converts via LinearRgba::from().to_vec4().
        // sRGB extremes (0.0, 1.0) map to themselves in linear space.
        let linear = bevy::color::LinearRgba::from(Color::srgb(1.0, 0.0, 0.0)).to_vec4();
        assert_eq!(linear, Vec4::new(1.0, 0.0, 0.0, 1.0));

        // Mid-range: sRGB 0.5 ≈ linear 0.214
        let linear_mid = bevy::color::LinearRgba::from(Color::srgb(0.5, 0.5, 0.5)).to_vec4();
        assert!(
            (linear_mid.x - 0.214).abs() < 0.01,
            "sRGB 0.5 should convert to ~0.214 linear, got {}",
            linear_mid.x
        );
    }

    // -- tick_circle_materials (#49) ---------------------------------------

    fn shader_test_app() -> App {
        let mut app = test_app_with_assets();
        app.init_asset::<CircleMaterial>();
        app.add_systems(Update, tick_circle_materials);
        app
    }

    #[test]
    fn tick_advances_time_for_rippling_material() {
        let mut app = shader_test_app();
        let handle = app
            .world_mut()
            .resource_mut::<Assets<CircleMaterial>>()
            .add(CircleMaterial::aura(Color::WHITE));

        // First update initialises Time; second provides non-zero delta.
        app.update();
        app.update();

        let mats = app.world().resource::<Assets<CircleMaterial>>();
        let mat = mats.get(&handle).unwrap();
        assert!(mat.time > 0.0, "time should advance for rippling material");
    }

    #[test]
    fn tick_does_not_advance_time_for_static_material() {
        let mut app = shader_test_app();
        let handle = app
            .world_mut()
            .resource_mut::<Assets<CircleMaterial>>()
            .add(CircleMaterial::range_indicator(Color::WHITE));

        app.update();
        app.update();

        let mats = app.world().resource::<Assets<CircleMaterial>>();
        let mat = mats.get(&handle).unwrap();
        assert_eq!(mat.time, 0.0, "time should not advance for static material");
    }

    #[test]
    fn tick_does_not_advance_time_for_negative_ripple_speed() {
        let mut app = shader_test_app();
        let handle = app
            .world_mut()
            .resource_mut::<Assets<CircleMaterial>>()
            .add(CircleMaterial {
                color: Color::WHITE,
                softness: DEFAULT_SOFTNESS,
                fill_fade: 0.0,
                ripple_speed: -1.0,
                time: 0.0,
            });

        app.update();
        app.update();

        let mats = app.world().resource::<Assets<CircleMaterial>>();
        let mat = mats.get(&handle).unwrap();
        assert_eq!(
            mat.time, 0.0,
            "time should not advance for negative ripple_speed"
        );
    }

    // -- setup_circle_mesh (#68) ------------------------------------------

    #[test]
    fn setup_circle_mesh_inserts_resource() {
        let mut app = test_app_with_assets();
        app.init_asset::<Mesh>();
        app.add_systems(Update, setup_circle_mesh);
        app.update();

        let circle_mesh = app.world().get_resource::<CircleMesh>();
        assert!(
            circle_mesh.is_some(),
            "setup_circle_mesh should insert CircleMesh resource"
        );

        let meshes = app.world().resource::<Assets<Mesh>>();
        assert!(
            meshes.get(&circle_mesh.unwrap().0).is_some(),
            "CircleMesh handle should point to a valid mesh"
        );
    }

    // -- Multi-frame ripple accumulation (#68) ----------------------------

    #[test]
    fn ripple_time_accumulates_across_frames() {
        let mut app = shader_test_app();
        let handle = app
            .world_mut()
            .resource_mut::<Assets<CircleMaterial>>()
            .add(CircleMaterial::aura(Color::WHITE));

        let dt = Duration::from_secs_f32(1.0 / 60.0);
        let frame_count = 1000;

        for _ in 0..frame_count {
            app.world_mut()
                .resource_mut::<Time<Virtual>>()
                .advance_by(dt);
            app.update();
        }

        let mats = app.world().resource::<Assets<CircleMaterial>>();
        let mat = mats.get(&handle).unwrap();

        assert!(
            mat.time.is_finite(),
            "time should remain finite after {frame_count} frames"
        );
        assert!(
            mat.time > 0.0,
            "time should be positive after {frame_count} frames"
        );

        // The shader uses fract(time * ripple_speed), so verify the product is valid.
        let wave_pos = (mat.time * mat.ripple_speed).fract();
        assert!(
            wave_pos.is_finite(),
            "fract(time * ripple_speed) should be finite"
        );
        assert!(
            (0.0..1.0).contains(&wave_pos),
            "fract result should be in [0, 1), got {wave_pos}"
        );
    }

    // -- Bind-group serialization (#68) -----------------------------------

    #[test]
    fn bind_group_uniform_matches_material() {
        let mat = CircleMaterial::aura(Color::WHITE);

        // Create a dummy RenderAssets — the impl ignores it.
        let images =
            bevy::render::render_asset::RenderAssets::<bevy::render::texture::GpuImage>::default();

        let uniform: CircleMaterialUniform = mat.as_bind_group_shader_type(&images);

        assert_eq!(uniform.softness, mat.softness);
        assert_eq!(uniform.fill_fade, mat.fill_fade);
        assert_eq!(uniform.ripple_speed, mat.ripple_speed);
        assert_eq!(uniform.time, mat.time);

        // White sRGB should convert to linear (1, 1, 1, 1).
        let expected = bevy::color::LinearRgba::from(Color::WHITE).to_vec4();
        assert_eq!(uniform.color, expected);
    }

    // -- Shader math property tests (#68) ---------------------------------

    /// GLSL/WGSL smoothstep equivalent.
    fn smoothstep_f32(edge0: f32, edge1: f32, x: f32) -> f32 {
        let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
        t * t * (3.0 - 2.0 * t)
    }

    /// Rust mirror of the WGSL SDF edge mask.
    fn sdf_edge(dist: f32, softness: f32) -> f32 {
        1.0 - smoothstep_f32(1.0 - softness, 1.0, dist)
    }

    /// Rust mirror of the WGSL ripple calculation.
    fn sdf_ripple(dist: f32, time: f32, ripple_speed: f32) -> f32 {
        if ripple_speed <= 0.0 {
            return 0.0;
        }
        let wave_pos = (time * ripple_speed).fract();
        let wave_width = 0.12;
        let ripple = smoothstep_f32(wave_pos - wave_width, wave_pos, dist)
            * (1.0 - smoothstep_f32(wave_pos, wave_pos + wave_width, dist));
        ripple * (1.0 - wave_pos)
    }

    /// Rust mirror of the full WGSL alpha output.
    fn sdf_alpha(
        color_a: f32,
        dist: f32,
        softness: f32,
        fill_fade: f32,
        time: f32,
        ripple_speed: f32,
    ) -> f32 {
        let edge = sdf_edge(dist, softness);
        let fade = (1.0 - fill_fade) + fill_fade * (1.0 - dist); // mix(1.0, 1.0-dist, fill_fade)
        let ripple = sdf_ripple(dist, time, ripple_speed);
        color_a * edge * (fade + ripple * 0.6)
    }

    #[test]
    fn ripple_zero_when_speed_zero() {
        for dist_i in 0..=20 {
            let dist = dist_i as f32 / 20.0;
            for time_i in 0..=10 {
                let time = time_i as f32 * 0.1;
                assert_eq!(
                    sdf_ripple(dist, time, 0.0),
                    0.0,
                    "ripple should be 0 when speed is 0 (dist={dist}, time={time})"
                );
            }
        }
    }

    #[test]
    fn edge_falloff_is_monotonic() {
        let softness = DEFAULT_SOFTNESS;
        let steps = 100;
        let mut prev = sdf_edge(0.0, softness);
        for i in 1..=steps {
            let dist = i as f32 / steps as f32;
            let curr = sdf_edge(dist, softness);
            assert!(
                curr <= prev + f32::EPSILON,
                "edge should be monotonically non-increasing at dist={dist}"
            );
            prev = curr;
        }
    }

    #[test]
    fn alpha_composed_correctly() {
        let color_a = 0.8;
        let softness = DEFAULT_SOFTNESS;

        // At center (dist=0), no ripple: alpha should equal color_a
        // (edge=1.0, fade=1.0 for fill_fade=0).
        let alpha_center = sdf_alpha(color_a, 0.0, softness, 0.0, 0.0, 0.0);
        assert!(
            (alpha_center - color_a).abs() < 1e-6,
            "center alpha should equal color.a, got {alpha_center}"
        );

        // Well outside circle (dist=2.0): alpha should be ~0.
        let alpha_outside = sdf_alpha(color_a, 2.0, softness, 0.0, 0.0, 0.0);
        assert!(
            alpha_outside.abs() < 1e-6,
            "outside alpha should be ~0, got {alpha_outside}"
        );

        // With ripple active, the additive term (ripple * 0.6) can push alpha
        // above color.a — but it must stay non-negative and finite.
        for dist_i in 0..=20 {
            let dist = dist_i as f32 / 20.0;
            for time_i in 0..=10 {
                let time = time_i as f32 * 0.1;
                let alpha = sdf_alpha(color_a, dist, softness, 0.5, time, 0.4);
                assert!(
                    alpha >= 0.0,
                    "alpha should be non-negative at dist={dist}, time={time}"
                );
                assert!(
                    alpha.is_finite(),
                    "alpha should be finite at dist={dist}, time={time}"
                );
            }
        }
    }
}
