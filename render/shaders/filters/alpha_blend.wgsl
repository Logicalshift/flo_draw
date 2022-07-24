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

    let texture_size    = vec2<f32>(textureDimensions(input_texture));
    let texture_pos     = vec2<f32>((pos[0]+1.0)/2.0, 1.0-((pos[1]+1.0)/2.0));
    let texture_pos     = vec2<f32>(texture_size * texture_pos);

    result.pos          = vec4<f32>(pos[0], pos[1], 0.0, 1.0);
    result.texture_pos  = texture_pos;

    return result;
}

@fragment
fn filter_fragment_shader_premultiply(vertex: RasterData) -> @location(0) vec4<f32> {
    let texture_pos     = vec2<i32>(vertex.texture_pos);

    var color           = textureLoad(input_texture, texture_pos, 0);
    color[3]            *= f_alpha;

    return color;
}

@fragment
fn filter_fragment_shader_not_premultiplied(vertex: RasterData) -> @location(0) vec4<f32> {
    let texture_pos     = vec2<i32>(vertex.texture_pos);

    var color           = textureLoad(input_texture, texture_pos, 0);
    color[3]            *= f_alpha;

    return color;
}
