@group(2)
@binding(0)
var f_texture: texture_multisampled_2d<f32>;

fn texture_color(vertex_color: vec4<f32>, texture_pos: vec2<f32>) -> vec4<f32> {
    let size            = vec2<f32>(textureDimensions(f_texture));
    let num_samples     = textureNumSamples(f_texture);

    let pos             = vec2<i32>(size * texture_pos);

    var sample_totals   = vec4<f32>();
    for (var sample_num = 0; sample_num < num_samples; sample_num++) {
        sample_totals += textureLoad(f_texture, pos, sample_num);
    }

    return sample_totals / f32(num_samples);
}
