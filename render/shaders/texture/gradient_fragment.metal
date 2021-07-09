#include <metal_stdlib>

#import "./bindings/metal_vertex2d.h"

typedef struct {
    float4 v_Position [[position]];
    float v_TexCoord;
    float2 v_PaperCoord;
} GradientData;

vertex GradientData gradient_vertex(
      uint        vertex_id [[ vertex_id ]],
      constant    matrix_float4x4 *transform      [[ buffer(VertexInputIndexMatrix )]],
      constant    MetalVertex2D   *vertices       [[ buffer(VertexInputIndexVertices) ]]) {
    float4 position     = float4(vertices[vertex_id].pos[0], vertices[vertex_id].pos[1], 0.0, 1.0) * *transform;
    float2 tex_coord    = vertices[vertex_id].tex_coord;
    float2 paper_coord  = float2((position[0]+1.0)/2.0, 1.0-((position[1]+1.0)/2.0));

    GradientData data;

    data.v_Position     = position;
    data.v_TexCoord     = tex_coord[0];
    data.v_PaperCoord   = paper_coord;

    return data;
}

fragment float4 gradient_fragment(
      GradientData                in [[stage_in]],
      metal::texture1d<half>      texture [[ texture(FragmentIndexTexture) ]]) {
    constexpr metal::sampler texture_sampler (metal::mag_filter::linear, metal::min_filter::linear);

    const half4 color_sample = texture.sample(texture_sampler, in.v_TexCoord);

    return float4(color_sample);
}

fragment float4 gradient_eraser_multisample_fragment(
      GradientData                in [[stage_in]],
      metal::texture1d<half>      texture [[ texture(FragmentIndexTexture) ]],
      metal::texture2d_ms<half>   eraser_texture [[ texture(FragmentIndexEraseTexture) ]]) {
    // Color from the gradient
    constexpr metal::sampler texture_sampler (metal::mag_filter::linear, metal::min_filter::linear);

    const half4 color_sample    = texture.sample(texture_sampler, in.v_TexCoord);

    // Work out the coordinates in the eraser texture (which applies to the whole screen)
    float2 paperCoord           = in.v_PaperCoord;
    paperCoord[0]               *= float(eraser_texture.get_width());
    paperCoord[1]               *= float(eraser_texture.get_height());

    // Sample the eraser
    const uint num_samples      = eraser_texture.get_num_samples();
    const uint2 eraser_coord    = uint2(paperCoord);
    half eraser_total           = 0;

    for (uint sample_num=0; sample_num<num_samples; ++sample_num) {
        const half4 sample      = eraser_texture.read(eraser_coord, sample_num);
        eraser_total            += sample[0];
    }

    // Adjust the color according to the erase texture at this point
    float eraser_alpha          = float(eraser_total) / float(num_samples);
    float4 color                = static_cast<float4>(color_sample);

    color[0]                    *= 1-eraser_alpha;
    color[1]                    *= 1-eraser_alpha;
    color[2]                    *= 1-eraser_alpha;
    color[3]                    *= 1-eraser_alpha;

    return color;
}
