#include <metal_stdlib>

#import "./bindings/metal_vertex2d.h"

typedef struct {
    float4 v_Position [[position]];
    float4 v_Color;
    float2 v_TexCoord;
    float2 v_PaperCoord;
} RasterizerData;

vertex RasterizerData simple_vertex(
    uint        vertex_id [[ vertex_id ]],
    constant    matrix_float4x4 *transform      [[ buffer(VertexInputIndexMatrix )]],
    constant    MetalVertex2D   *vertices       [[ buffer(VertexInputIndexVertices) ]]) {
    
    uchar4 byte_color   = vertices[vertex_id].color;
    float4 color        = float4(byte_color[0], byte_color[1], byte_color[2], byte_color[3]);
    color[0]            /= 255.0;
    color[1]            /= 255.0;
    color[2]            /= 255.0;
    color[3]            /= 255.0;

    float4 position     = float4(vertices[vertex_id].pos[0], vertices[vertex_id].pos[1], 0.0, 1.0) * *transform;
    float2 tex_coord    = vertices[vertex_id].tex_coord;
    float2 paper_coord  = float2((position[0]+1.0)/2.0, (position[1]+1.0)/2.0);

    RasterizerData data;

    data.v_Position     = position;
    data.v_Color        = color;
    data.v_TexCoord     = tex_coord;
    data.v_PaperCoord   = paper_coord;

    return data;
}

fragment float4 simple_fragment(
      RasterizerData in [[stage_in]]) {
    return in.v_Color;
}

fragment float4 texture_fragment(
    RasterizerData              in [[stage_in]],
    metal::texture2d<half>      texture [[ texture(FragmentIndexTexture) ]]) {
    constexpr metal::sampler texture_sampler (metal::mag_filter::linear, metal::min_filter::linear);

    const half4 color_sample = texture.sample(texture_sampler, in.v_TexCoord);

    return float4(color_sample);
}

fragment float4 texture_multisample_fragment(
    RasterizerData              in [[stage_in]],
    metal::texture2d_ms<half>   texture [[ texture(FragmentIndexTexture) ]]) {
    constexpr metal::sampler texture_sampler (metal::mag_filter::linear, metal::min_filter::linear);

    const uint num_samples      = texture.get_num_samples();
    const uint2 tex_coord       = uint2(in.v_TexCoord);
    half4 color_totals          = half4(0,0,0,0);

    for (uint sample_num=0; sample_num<num_samples; ++sample_num) {
        const half4 sample      = texture.read(tex_coord, sample_num);
        color_totals            += sample;
    }

    float4 color                = float4(color_totals);
    color /= float(num_samples);

    return color;
}
