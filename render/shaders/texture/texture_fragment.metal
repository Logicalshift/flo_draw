#include <metal_stdlib>

#import "./bindings/metal_vertex2d.h"
#import "../simple/rasterizer.metal"

vertex RasterizerData texture_vertex(
      uint        vertex_id [[ vertex_id ]],
      constant    matrix_float4x4 *transform          [[ buffer(VertexInputIndexMatrix )]],
      constant    MetalVertex2D   *vertices           [[ buffer(VertexInputIndexVertices) ]],
      constant    matrix_float4x4 *texture_transform  [[ buffer(VertexTextureMatrix) ]]) {
    float4 position     = float4(vertices[vertex_id].pos[0], vertices[vertex_id].pos[1], 0.0, 1.0) * *transform;
    float4 tex_coord    = float4(vertices[vertex_id].pos[0], vertices[vertex_id].pos[1], 0.0, 1.0) * *texture_transform;
    float2 paper_coord  = float2((position[0]+1.0)/2.0, 1.0-((position[1]+1.0)/2.0));

    RasterizerData data;

    data.v_Position     = position;
    data.v_Color        = float4(0.0, 0.0, 0.0, 1.0);
    data.v_TexCoord     = float2(tex_coord[0], tex_coord[1]);
    data.v_PaperCoord   = paper_coord;

    return data;
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

fragment float4 texture_eraser_multisample_fragment(
      RasterizerData              in [[stage_in]],
      metal::texture2d<half>      texture [[ texture(FragmentIndexTexture) ]],
      metal::texture2d_ms<half>   eraser_texture [[ texture(FragmentIndexEraseTexture) ]]) {
    // Color from the texture
    constexpr metal::sampler texture_sampler (metal::mag_filter::linear, metal::min_filter::linear);
    const half4 color_sample    = texture.sample(texture_sampler, in.v_TexCoord);

    // Apply the eraser
    float4 color                = apply_eraser(static_cast<float4>(color_sample), in.v_PaperCoord, eraser_texture);

    return color;
}

fragment float4 texture_clip_mask_multisample_fragment(
      RasterizerData              in [[stage_in]],
      metal::texture2d<half>      texture [[ texture(FragmentIndexTexture) ]],
      metal::texture2d_ms<half>   clip_mask_texture [[ texture(FragmentIndexClipMaskTexture) ]]) {
    // Color from the texture
    constexpr metal::sampler texture_sampler (metal::mag_filter::linear, metal::min_filter::linear);
    const half4 color_sample    = texture.sample(texture_sampler, in.v_TexCoord);

    // Apply the clip mask
    float4 color = apply_clip_mask(static_cast<float4>(color_sample), in.v_PaperCoord, clip_mask_texture);
    return color;
}

fragment float4 texture_eraser_clip_mask_multisample_fragment(
      RasterizerData              in [[stage_in]],
      metal::texture2d<half>      texture [[ texture(FragmentIndexTexture) ]],
      metal::texture2d_ms<half>   eraser_texture [[ texture(FragmentIndexEraseTexture) ]],
      metal::texture2d_ms<half>   clip_mask_texture [[ texture(FragmentIndexClipMaskTexture) ]]) {
    // Color from the texture
    constexpr metal::sampler texture_sampler (metal::mag_filter::linear, metal::min_filter::linear);
    const half4 color_sample    = texture.sample(texture_sampler, in.v_TexCoord);

    // Apply the eraser and clip mask
    float4 color = apply_eraser(static_cast<float4>(color_sample), in.v_PaperCoord, eraser_texture);
    color = apply_clip_mask(color, in.v_PaperCoord, clip_mask_texture);
    return color;
}
