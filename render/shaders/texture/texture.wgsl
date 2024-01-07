struct RasterData {
    @location(0)        color:      vec4<f32>,
    @location(1)        tex_coord:  vec2<f32>,
    @builtin(position)  pos:        vec4<f32>
}

struct TextureSettings {
    @location(0)    transform:  mat4x4<f32>,
    @location(1)    alpha:      f32
}

@group(0)
@binding(0)
var<uniform> transform: mat4x4<f32>;

@group(2)
@binding(0)
var<uniform> texture_settings: TextureSettings;

@vertex
fn texture_vertex_shader(
    @location(0) pos:       vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color:     vec4<u32>,
) -> RasterData {
    var result: RasterData;

    var color_r = vec4<f32>(f32(color[0]), f32(color[1]), f32(color[2]), f32(color[3]));
    color_r[0] /= 255.0;
    color_r[1] /= 255.0;
    color_r[2] /= 255.0;
    color_r[3] /= 255.0;

    let tex_coord_r     = texture_position(pos, tex_coord, texture_settings.transform);

    result.color        = color_r;
    result.tex_coord    = tex_coord_r;
    result.pos          = vec4<f32>(pos[0], pos[1], 0.0, 1.0) * transform;

    return result;
}

@fragment
fn texture_fragment_shader(vertex: RasterData) -> @location(0) vec4<f32> {
    var color = texture_color(vertex.color, vertex.tex_coord);
    color = alpha_blend(color, texture_settings.alpha);

    color = clip(color, vertex.pos);
    color = color_post_process(color);

    return color;
}
