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
