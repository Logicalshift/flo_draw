struct RasterData {
    @location(0)        texture_pos:    vec2<f32>,
    @builtin(position)  pos:            vec4<f32>
}

@group(0)
@binding(0)
var input_texture: texture_2d<f32>;

@group(0)
@binding(1)
var input_sampler: sampler;

@vertex
fn filter_vertex_shader(
    @location(0) pos:       vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color:     vec4<u32>,
) -> RasterData {
    var result: RasterData;

    let texture_size    = vec2<f32>(textureDimensions(input_texture));
    
    let top_left        = vec2<f32>(1.0, 1.0);
    let bottom_right    = vec2<f32>(texture_size[0]-1.0, texture_size[1]-1.0);

    // Convert the range of the position to a value betwen 0-1
    let texture_pos     = vec2<f32>((pos[0]+1.0)/2.0, 1.0-((pos[1]+1.0)/2.0));

    // Convert to a position on the texture. We want to half the size of the texture, so start at (1.0, 1.0) - between the first four pixels of the texture
    let texture_pos     = (bottom_right-top_left) * texture_pos + top_left;

    // Convert back to coordinates in the range 0-1
    let texture_pos     = texture_pos / texture_size;

    result.pos          = vec4<f32>(pos[0], pos[1], 0.0, 1.0); 
    result.texture_pos  = texture_pos;

    return result;
}

@fragment
fn filter_fragment_shader(vertex: RasterData) -> @location(0) vec4<f32> {
    return textureSample(input_texture, input_sampler, vertex.texture_pos);
}
