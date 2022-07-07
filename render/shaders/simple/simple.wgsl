struct RasterData {
    @location(0)        color:  vec4<f32>,
    @builtin(position)  pos:    vec4<f32>
}

@group(0)
@binding(0)
var<uniform> transform: mat4x4<f32>;

@vertex
fn simple_vertex_shader(
    @location(0) pos:       vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color:     vec4<u32>,
) -> RasterData {
    var result: RasterData;

    var color = vec4<f32>(f32(color[0]), f32(color[1]), f32(color[2]), f32(color[3]));
    color[0]        /= 255.0;
    color[1]        /= 255.0;
    color[2]        /= 255.0;
    color[3]        /= 255.0;

    result.color    = color;
    result.pos      = pos * transform;

    return result;
}

@fragment
fn simple_fragment_shader(vertex: RasterData) -> @location(0) vec4<f32> {
    return vertex.color;
}
