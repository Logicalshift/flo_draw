#include <metal_stdlib>

#import "./bindings/metal_vertex2d.h"
#import "rasterizer.metal"

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
    float2 paper_coord  = float2((position[0]+1.0)/2.0, 1.0-((position[1]+1.0)/2.0));

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

fragment float4 simple_clip_mask_multisample_fragment(
      RasterizerData            in [[stage_in]],
      metal::texture2d_ms<half> clip_mask_texture [[ texture(FragmentIndexClipMaskTexture) ]]) {
    float4 color = apply_clip_mask(in.v_Color, in.v_PaperCoord, clip_mask_texture);
    return color;
}
