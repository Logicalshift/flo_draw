@group(2)
@binding(1)
var f_texture: texture_2d<f32>;

@group(2)
@binding(2)
var f_sampler: sampler;

@group(2)
@binding(3)
var<uniform> f_alpha: f32;

fn texture_color(vertex_color: vec4<f32>, texture_pos: vec2<f32>) -> vec4<f32> {
    let raw_color       = textureSample(f_texture, f_sampler, texture_pos);
    let alpha_blended   = alpha_blend(raw_color, f_alpha);

    return alpha_blended;
}
