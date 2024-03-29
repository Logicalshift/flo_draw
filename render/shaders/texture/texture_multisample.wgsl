@group(2)
@binding(1)
var f_texture: texture_multisampled_2d<f32>;

fn texture_color(vertex_color: vec4<f32>, texture_pos: vec2<f32>) -> vec4<f32> {
    let size            = vec2<f32>(textureDimensions(f_texture));
    let num_samples     = i32(textureNumSamples(f_texture));

    let pos             = vec2<i32>(size * texture_pos);

    var sample_totals   = vec4<f32>();
    for (var sample_num = i32(0); sample_num < num_samples; sample_num++) {
        sample_totals += textureLoad(f_texture, pos, sample_num);
    }

    let sample_col      = sample_totals / f32(num_samples);

    return sample_col;
}
