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

/// Shared mesh handle for a unit circle (radius 1.0).
/// Scale via Transform to get the desired diameter.
#[derive(Resource)]
pub struct CircleMesh(pub Handle<Mesh>);

fn setup_circle_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(CircleMesh(meshes.add(Circle::new(0.5))));
}

/// Advance `time` on all circle materials that have a non-zero ripple speed.
fn tick_circle_materials(
    mut materials: ResMut<Assets<CircleMaterial>>,
    time: Res<Time>,
) {
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
