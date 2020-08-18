#include <metal_stdlib>

#import "./bindings/metal_vertex2d.h"
#import "rasterizer.metal"

fragment float4 simple_eraser_multisample_fragment(
      RasterizerData            in [[stage_in]],
      metal::texture2d_ms<half> eraser_texture [[ texture(FragmentIndexTexture) ]]) {
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
    float4 color                = in.v_Color;

    color[0]                    *= 1-eraser_alpha;
    color[1]                    *= 1-eraser_alpha;
    color[2]                    *= 1-eraser_alpha;
    color[3]                    *= 1-eraser_alpha;

    return color;
}
