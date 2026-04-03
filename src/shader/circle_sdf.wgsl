#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct CircleMaterial {
    color: vec4<f32>,
    softness: f32,
};

@group(2) @binding(0) var<uniform> material: CircleMaterial;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = mesh.uv - vec2(0.5);
    let dist = length(uv) * 2.0;
    let alpha = 1.0 - smoothstep(1.0 - material.softness, 1.0, dist);
    return vec4(material.color.rgb, material.color.a * alpha);
}
