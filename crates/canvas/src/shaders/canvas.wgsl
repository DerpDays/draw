@group(0) @binding(0) var<uniform> world_projection: mat4x4<f32>;
@group(0) @binding(1) var<uniform> viewport_projection: mat4x4<f32>;
@group(1) @binding(0) var mask_atlas: texture_2d_array<f32>;
@group(1) @binding(1) var color_atlas: texture_2d_array<f32>;
@group(1) @binding(2) var tex_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) kind: u32,
    @location(3) texture: u32,
    @location(4) tex_coords: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) kind: u32,
    @location(2) texture: u32,
    @location(3) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    if input.kind < 3 { // world coordinates
        out.clip_position = world_projection * vec4(input.position, 0.0, 1.0);
    } else { // viewport coordinates
        out.clip_position = viewport_projection * vec4(input.position, 0.0, 1.0);
    }

    out.color = input.color;
    out.kind = input.kind;
    out.texture = input.texture;
    out.tex_coords = input.tex_coords;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    switch in.kind {
        case 0u, 3u: {return in.color;}
        case 1u, 4u: { return vec4<f32>(in.color.rgb, in.color.a * textureSampleLevel(mask_atlas, tex_sampler, in.tex_coords, in.texture, 0.).x) ;}
        case 2u, 5u: {return textureSample(color_atlas, tex_sampler, in.tex_coords, in.texture);}
        default: {return vec4<f32>(0.);}
    }
}
