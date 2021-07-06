#include <metal_stdlib>

#import "./bindings/metal_vertex2d.h"
#import "../simple/rasterizer.metal"

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
