@group(0) @binding(0) var<uniform> viewport: Viewport2D;
@group(1) @binding(0) var tex_sampler: sampler;
@group(2) @binding(0) var texture: texture_2d<f32>;


struct Viewport2D {
    scale: vec2<f32>,
    translate: vec2<f32>,
};

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>((input.position * viewport.scale) + viewport.translate, 0., 1.);
    out.tex_coords = input.uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return textureSample(texture, tex_sampler, in.tex_coords);
}
