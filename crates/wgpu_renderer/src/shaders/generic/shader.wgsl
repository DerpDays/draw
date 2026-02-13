@group(0) @binding(0) var<uniform> viewport: Viewport2D;
@group(1) @binding(0) var mask_atlas: texture_2d_array<f32>;
@group(1) @binding(1) var color_atlas: texture_2d_array<f32>;
@group(1) @binding(2) var tex_sampler: sampler;


struct Viewport2D {
    scale: vec2<f32>,
    translate: vec2<f32>,
};

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
    @location(1) @interpolate(flat) kind: u32,
    @location(2) @interpolate(flat) texture: u32,
    @location(3) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>((input.position * viewport.scale) + viewport.translate, 0., 1.);
    out.color = input.color;
    out.kind = input.kind;
    out.texture = input.texture;
    out.tex_coords = input.tex_coords;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (in.kind == 0u) {
        return in.color;
    } else if (in.kind == 1u) {
        let mask = textureSampleLevel(mask_atlas, tex_sampler, in.tex_coords, in.texture, 0.0).r;
        // in.color.rgb is premultiplied
        return vec4(in.color.rgb * mask, in.color.a * mask);
    }
    return textureSampleLevel(color_atlas, tex_sampler, in.tex_coords, in.texture, 0.);
}
