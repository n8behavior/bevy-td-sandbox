use bevy::asset::embedded_asset;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, AsBindGroupShaderType, ShaderType};
use bevy::shader::ShaderRef;
use bevy::sprite_render::{AlphaMode2d, Material2d, Material2dPlugin};

/// GPU-driven circle using a signed distance field fragment shader.
#[derive(Asset, AsBindGroup, Clone, Debug, Reflect)]
#[uniform(0, CircleMaterialUniform)]
pub struct CircleMaterial {
    pub color: Color,
    /// 0.0 = hard edge, larger = softer falloff. 0.05 is a good default.
    pub softness: f32,
    /// 0.0 = uniform fill, 1.0 = radial gradient (fades center→edge).
    pub fill_fade: f32,
    /// Ripple wave speed (0.0 = off, 1.0 = one pulse per second).
    pub ripple_speed: f32,
    /// Elapsed time in seconds — updated by `tick_circle_materials` each frame.
    pub time: f32,
}

impl CircleMaterial {
    /// Hard-edged ring with no fill or ripple. Used for tower range indicators.
    pub fn range_indicator(color: Color) -> Self {
        Self {
            color,
            softness: 0.05,
            fill_fade: 0.0,
            ripple_speed: 0.0,
            time: 0.0,
        }
    }

    /// Radial-gradient ring with ripple pulse. Used for aura effects.
    pub fn aura(color: Color) -> Self {
        Self {
            color,
            softness: 0.05,
            fill_fade: 1.0,
            ripple_speed: 0.4,
            time: 0.0,
        }
    }
}

impl Material2d for CircleMaterial {
    fn fragment_shader() -> ShaderRef {
        "embedded://bevy_td_sandbox/shader/circle_sdf.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode2d {
        AlphaMode2d::Blend
    }
}

/// GPU uniform layout matching the WGSL struct.
#[derive(Clone, Default, ShaderType)]
struct CircleMaterialUniform {
    color: Vec4,
    softness: f32,
    fill_fade: f32,
    ripple_speed: f32,
    time: f32,
}

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

/// Shared mesh handle for a unit circle (radius 0.5, diameter 1.0).
/// Scale via Transform to get the desired diameter.
#[derive(Resource)]
pub struct CircleMesh(pub Handle<Mesh>);

fn setup_circle_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(CircleMesh(meshes.add(Circle::new(0.5))));
}

/// Advance `time` on all circle materials that have a non-zero ripple speed.
fn tick_circle_materials(mut materials: ResMut<Assets<CircleMaterial>>, time: Res<Time>) {
    for (_, mat) in materials.iter_mut() {
        if mat.ripple_speed > 0.0 {
            mat.time += time.delta_secs();
        }
    }
}

pub struct ShaderPlugin;

impl Plugin for ShaderPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "circle_sdf.wgsl");
        app.add_plugins(Material2dPlugin::<CircleMaterial>::default())
            .add_systems(Startup, setup_circle_mesh)
            .add_systems(Update, tick_circle_materials);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::test_app_with_assets;

    // -- Constructor tests (#50) ------------------------------------------

    #[test]
    fn range_indicator_field_values() {
        let mat = CircleMaterial::range_indicator(Color::WHITE);
        assert_eq!(mat.softness, 0.05);
        assert_eq!(mat.fill_fade, 0.0);
        assert_eq!(mat.ripple_speed, 0.0);
        assert_eq!(mat.time, 0.0);
    }

    #[test]
    fn aura_field_values() {
        let mat = CircleMaterial::aura(Color::WHITE);
        assert_eq!(mat.softness, 0.05);
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
                softness: 0.05,
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
}
