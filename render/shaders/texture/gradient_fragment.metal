#include <metal_stdlib>

#import "./bindings/metal_vertex2d.h"
#import "../simple/rasterizer.metal"

typedef struct {
    float4 v_Position [[position]];
    float v_TexCoord;
    float2 v_PaperCoord;
} GradientData;

vertex GradientData gradient_vertex(
      uint        vertex_id [[ vertex_id ]],
      constant    matrix_float4x4 *transform      [[ buffer(VertexInputIndexMatrix )]],
      constant    MetalVertex2D   *vertices       [[ buffer(VertexInputIndexVertices) ]],
      constant    matrix_float4x4 *texture_transform  [[ buffer(VertexTextureMatrix) ]]) {
    float4 position     = float4(vertices[vertex_id].pos[0], vertices[vertex_id].pos[1], 0.0, 1.0) * *transform;
    float4 tex_coord    = float4(vertices[vertex_id].pos[0], vertices[vertex_id].pos[1], 0.0, 1.0) * *texture_transform;
    float2 paper_coord  = float2((position[0]+1.0)/2.0, 1.0-((position[1]+1.0)/2.0));

    GradientData data;

    data.v_Position     = position;
    data.v_TexCoord     = tex_coord[0];
    data.v_PaperCoord   = paper_coord;

    return data;
}

fragment float4 gradient_fragment(
      GradientData                in [[stage_in]],
      constant float              *texture_alpha [[ buffer(FragmentAlpha) ]],
      metal::texture1d<half>      texture [[ texture(FragmentIndexTexture) ]]) {
    constexpr metal::sampler texture_sampler (metal::mag_filter::linear, metal::min_filter::linear);

    const half4 color_sample = texture.sample(texture_sampler, in.v_TexCoord);

    float4 color  = float4(color_sample);
    color[3]      *= *texture_alpha;

    return color;
}

fragment float4 gradient_clip_mask_multisample_fragment(
      GradientData                in [[stage_in]],
      constant float              *texture_alpha [[ buffer(FragmentAlpha) ]],
      metal::texture1d<half>      texture [[ texture(FragmentIndexTexture) ]],
      metal::texture2d_ms<half>   clip_mask_texture [[ texture(FragmentIndexClipMaskTexture) ]]) {
    // Color from the gradient
    constexpr metal::sampler texture_sampler (metal::mag_filter::linear, metal::min_filter::linear);
    const half4 color_sample    = texture.sample(texture_sampler, in.v_TexCoord);

    // Apply the clip mask
    float4 color  = apply_clip_mask(static_cast<float4>(color_sample), in.v_PaperCoord, clip_mask_texture);
    color[3]      *= *texture_alpha;

    return color;
}

fragment float4 gradient_fragment_invert_color_alpha(
      GradientData                in [[stage_in]],
      constant float              *texture_alpha [[ buffer(FragmentAlpha) ]],
      metal::texture1d<half>      texture [[ texture(FragmentIndexTexture) ]]) {
    constexpr metal::sampler texture_sampler (metal::mag_filter::linear, metal::min_filter::linear);

    const half4 color_sample = texture.sample(texture_sampler, in.v_TexCoord);

    float4 color  = float4(color_sample);
    color[3]      *= *texture_alpha;

    return invert_color_alpha(color);
}

fragment float4 gradient_clip_mask_multisample_fragment_invert_color_alpha(
      GradientData                in [[stage_in]],
      constant float              *texture_alpha [[ buffer(FragmentAlpha) ]],
      metal::texture1d<half>      texture [[ texture(FragmentIndexTexture) ]],
      metal::texture2d_ms<half>   clip_mask_texture [[ texture(FragmentIndexClipMaskTexture) ]]) {
    // Color from the gradient
    constexpr metal::sampler texture_sampler (metal::mag_filter::linear, metal::min_filter::linear);
    const half4 color_sample    = texture.sample(texture_sampler, in.v_TexCoord);

    // Apply the clip mask
    float4 color  = apply_clip_mask(static_cast<float4>(color_sample), in.v_PaperCoord, clip_mask_texture);
    color[3]      *= *texture_alpha;

    return invert_color_alpha(color);
}
