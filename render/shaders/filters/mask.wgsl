struct RasterData {
    @location(0)        texture_pos:    vec2<f32>,
    @location(1)        clip_pos:       vec2<f32>,
    @builtin(position)  pos:            vec4<f32>
}

@group(0)
@binding(0)
var input_texture: texture_2d<f32>;

@group(0)
@binding(1)
var mask_texture: texture_2d<f32>;

@group(0)
@binding(2)
var mask_sampler: sampler;

@vertex
fn filter_vertex_shader(
    @location(0) pos:       vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color:     vec4<u32>,
) -> RasterData {
    var result: RasterData;

    let texture_size    = vec2<f32>(textureDimensions(input_texture));
    let clip_pos        = vec2<f32>((pos[0]+1.0)/2.0, 1.0-((pos[1]+1.0)/2.0));
    let texture_pos     = vec2<f32>(texture_size * clip_pos);

    result.pos          = vec4<f32>(pos[0], pos[1], 0.0, 1.0); 
    result.clip_pos     = clip_pos;
    result.texture_pos  = texture_pos;

    return result;
}

@fragment
fn filter_fragment_shader_premultiply(vertex: RasterData) -> @location(0) vec4<f32> {
    let texture_pos     = vec2<i32>(vertex.texture_pos);
    let clip_pos        = vertex.clip_pos;

    let color           = textureLoad(input_texture, texture_pos, 0);
    let clip_color      = textureSample(mask_texture, mask_sampler, clip_pos);

    let color           = color * clip_color[3];
    
    return color;
}

@fragment
fn filter_fragment_shader_no_premultiply(vertex: RasterData) -> @location(0) vec4<f32> {
    let texture_pos     = vec2<i32>(vertex.texture_pos);
    let clip_pos        = vertex.clip_pos;

    let color           = textureLoad(input_texture, texture_pos, 0);
    let clip_color      = textureSample(mask_texture, mask_sampler, clip_pos);

    let color           = vec4<f32>(color[0], color[1], color[2], color[3] * clip_color[3]);
    
    return color;
}
