struct RasterData {
    @location(0) color: vec<f32>
}

@vertex
fn simple_vertex_shader(
    @location(0) pos:       vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color:     vec4<u8>,
) -> RasterData {
    var result: RasterData;

    vec4<f32> color = vec4<f32>(f32(color[0]), f32(color[1]), f32(color[2]), f32(color));
    color[0]        /= 255.0;
    color[1]        /= 255.0;
    color[2]        /= 255.0;
    color[3]        /= 255.0;

    result.color = color;

    return result;
}

@fragment
fn simple_fragment_shader(vertex: RasterData) -> @location(0) vec4<f32> {
    return vertex.color;
}
