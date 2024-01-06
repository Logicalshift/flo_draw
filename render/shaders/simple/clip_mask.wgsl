@group(1)
@binding(0)
var clip_texture: texture_multisampled_2d<f32>;

fn clip(color: vec4<f32>, position: vec4<f32>) -> vec4<f32> {
    let clip_x              = position[0];
    let clip_y              = position[1];

    let clip_pos            = vec2<i32>(i32(clip_x), i32(clip_y));
    var clip_alpha          = f32(0.0);

    for (var sample_num: i32 = 0; sample_num < 4; sample_num++) {
        clip_alpha += textureLoad(clip_texture, clip_pos, sample_num)[0];
    }

    clip_alpha              *= 0.25;

    let clip_color          = vec4<f32>(
        color[0] * clip_alpha,
        color[1] * clip_alpha,
        color[2] * clip_alpha,
        color[3] * clip_alpha
    );

    return clip_color;
}
