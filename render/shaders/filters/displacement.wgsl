struct RasterData {
    @location(0)        texture_pos:    vec2<f32>,
    @location(1)        displace_pos:   vec2<f32>,
    @builtin(position)  pos:            vec4<f32>
}

@group(0)
@binding(0)
var input_texture: texture_2d<f32>;

@group(0)
@binding(1)
var displace_texture: texture_2d<f32>;

@group(0)
@binding(2)
var displace_sampler: sampler;

@group(0)
@binding(3)
var<uniform> scale: vec2<f32>;

@vertex
fn filter_vertex_shader(
    @location(0) pos:       vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color:     vec4<u32>,
) -> RasterData {
    var result: RasterData;

    let texture_size    = vec2<f32>(textureDimensions(input_texture));
    let displace_pos    = vec2<f32>((pos[0]+1.0)/2.0, 1.0-((pos[1]+1.0)/2.0));
    let texture_pos     = vec2<f32>(texture_size * displace_pos);

    result.pos          = vec4<f32>(pos[0], pos[1], 0.0, 1.0); 
    result.displace_pos = displace_pos;
    result.texture_pos  = texture_pos;

    return result;
}

@fragment
fn filter_fragment_shader(vertex: RasterData) -> @location(0) vec4<f32> {
    let displace_pos    = vertex.displace_pos;

    let displacement    = textureSample(displace_texture, displace_sampler, displace_pos);
    let displacement    = vec2<f32>((displacement[0] - 0.5) * 2.0, (displacement[1] - 0.5) * 2.0)*scale;

    let color           = textureSample(input_texture, displace_sampler, displace_pos + displacement);

    return color;
}
