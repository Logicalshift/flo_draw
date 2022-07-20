struct RasterData {
    @location(0)        texture_pos:    vec2<f32>,
    @builtin(position)  pos:            vec4<f32>
}

@group(0)
@binding(0)
var input_texture: texture_2d<f32>;

@group(0)
@binding(1)
var<uniform> f_alpha: f32;

@vertex
fn filter_vertex_shader(
    @location(0) pos:       vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color:     vec4<u32>,
) -> RasterData {
    var result: RasterData;

    let size            = vec2<f32>(textureDimensions(input_texture));
    let texture_pos     = vec2<f32>((pos[0]+1.0)/2.0, (pos[1]+1.0/2.0));
    let texture_pos     = vec2<f32>(size * texture_pos);

    result.pos          = vec4<f32>(pos[0], pos[1], 0.0, 1.0) * transform;

    return result;
}

@fragment
fn filter_fragment_shader(vertex: RasterData) -> @location(0) vec4<f32> {
    let texture_pos     = vec2<i32>(vertex.texture_pos);

    let color           = textureLoad(input_texture, pos, sample_num);
    let color           = color * f_alpha;
    let color           = color_post_process(color);

    return color;
}
