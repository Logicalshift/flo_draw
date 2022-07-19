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

@group(2)
@binding(1)
var f_texture: texture_1d<f32>;

@group(2)
@binding(2)
var f_sampler: sampler;

@vertex
fn gradient_vertex_shader(
    @location(0) pos:       vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color:     vec4<u32>,
) -> RasterData {
    var result: RasterData;

    var color = vec4<f32>(f32(color[0]), f32(color[1]), f32(color[2]), f32(color[3]));
    color[0] /= 255.0;
    color[1] /= 255.0;
    color[2] /= 255.0;
    color[3] /= 255.0;

    let tex_coord       = texture_position(pos, tex_coord, texture_settings.transform);

    result.color        = color;
    result.tex_coord    = tex_coord;
    result.pos          = vec4<f32>(pos[0], pos[1], 0.0, 1.0) * transform;

    return result;
}

@fragment
fn gradient_fragment_shader(vertex: RasterData) -> @location(0) vec4<f32> {
    let color = textureSample(f_texture, f_sampler, vertex.tex_coord[0]);
    let color = alpha_blend(color, texture_settings.alpha);

    let color = clip(color, vertex.pos);
    let color = color_post_process(color);

    return color;
}
