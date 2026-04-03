#import bevy_sprite::mesh2d_vertex_output::VertexOutput

struct CircleMaterial {
    color: vec4<f32>,
    softness: f32,
    // 0.0 = normal circle, 1.0 = radial gradient (fades center→edge)
    fill_fade: f32,
    // Ripple wave speed (0.0 = off). Higher = faster outward pulse.
    ripple_speed: f32,
    // Elapsed time in seconds (updated by CPU each frame).
    time: f32,
};

@group(2) @binding(0) var<uniform> material: CircleMaterial;

@fragment
fn fragment(mesh: VertexOutput) -> @location(0) vec4<f32> {
    let uv = mesh.uv - vec2(0.5);
    let dist = length(uv) * 2.0;   // 0 at center, 1 at edge

    // Edge mask: soft or hard circle boundary.
    let edge = 1.0 - smoothstep(1.0 - material.softness, 1.0, dist);

    // Radial fade: full opacity at center, zero at edge.
    let fade = mix(1.0, 1.0 - dist, material.fill_fade);

    // Ripple: a bright ring that radiates outward from center.
    var ripple = 0.0;
    if (material.ripple_speed > 0.0) {
        let wave_pos = fract(material.time * material.ripple_speed);
        let wave_width = 0.12;
        ripple = smoothstep(wave_pos - wave_width, wave_pos, dist)
               * (1.0 - smoothstep(wave_pos, wave_pos + wave_width, dist));
        // Fade ripple as it reaches the edge.
        ripple *= 1.0 - wave_pos;
    }

    let alpha = material.color.a * edge * (fade + ripple * 0.6);
    return vec4(material.color.rgb, alpha);
}
