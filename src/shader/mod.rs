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
    // ShaderType handles alignment automatically.
}

impl AsBindGroupShaderType<CircleMaterialUniform> for CircleMaterial {
    fn as_bind_group_shader_type(
        &self,
        _images: &bevy::render::render_asset::RenderAssets<bevy::render::texture::GpuImage>,
    ) -> CircleMaterialUniform {
        CircleMaterialUniform {
            color: bevy::color::LinearRgba::from(self.color).to_vec4(),
            softness: self.softness,
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

pub struct ShaderPlugin;

impl Plugin for ShaderPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "circle_sdf.wgsl");
        app.add_plugins(Material2dPlugin::<CircleMaterial>::default())
            .add_systems(Startup, setup_circle_mesh);
    }
}
