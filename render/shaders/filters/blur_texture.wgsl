struct RasterData {
    @location(0)        texture_pos:    vec2<f32>,
    @builtin(position)  pos:            vec4<f32>
}

@group(0)
@binding(0)
var input_texture: texture_2d<f32>;

@group(0)
@binding(1)
var f_sampler: sampler;

@group(0)
@binding(2)
var offset_texture: texture_1d<f32>;

@group(0)
@binding(3)
var weight_texture: texture_1d<f32>;

@vertex
fn filter_vertex_shader(
    @location(0) pos:       vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color:     vec4<u32>,
) -> RasterData {
    var result: RasterData;

    let texture_size    = vec2<f32>(textureDimensions(input_texture));
    let texture_pos     = vec2<f32>((pos[0]+1.0)/2.0, 1.0-((pos[1]+1.0)/2.0));

    result.pos          = vec4<f32>(pos[0], pos[1], 0.0, 1.0);
    result.texture_pos  = texture_pos;

    return result;
}

fn offset(i: i32) -> f32 {
    return textureLoad(offset_texture, i, 0)[0];
}

fn offset_horiz(i: i32) -> vec2<f32> {
    return vec2<f32>(offset(i), 0.0);
}

fn offset_vert(i: i32) -> vec2<f32> {
    return vec2<f32>(0.0, offset(i));
}

fn weight(i: i32) -> f32 {
    return textureLoad(weight_texture, i, 0)[0];
}

@fragment
fn filter_fragment_shader_blur_texture_horiz(vertex: RasterData) -> @location(0) vec4<f32> {
    let len     = textureDimensions(offset_texture);
    var color   = textureSample(input_texture, f_sampler, vertex.texture_pos) * weight(0);

    for (var idx=1; idx<len; idx++) {
        color = color + textureSample(input_texture, f_sampler, vertex.texture_pos + offset_horiz(1)) * weight(1);
        color = color + textureSample(input_texture, f_sampler, vertex.texture_pos + offset_horiz(2)) * weight(2);
    }

    return color;
}

@fragment
fn filter_fragment_shader_blur_texture_vert(vertex: RasterData) -> @location(0) vec4<f32> {
    let len     = textureDimensions(offset_texture);
    var color   = textureSample(input_texture, f_sampler, vertex.texture_pos) * weight(0);

    for (var idx=1; idx<len; idx++) {
        color = color + textureSample(input_texture, f_sampler, vertex.texture_pos + offset_vert(1)) * weight(1);
        color = color + textureSample(input_texture, f_sampler, vertex.texture_pos + offset_vert(2)) * weight(2);
    }

    return color;
}
