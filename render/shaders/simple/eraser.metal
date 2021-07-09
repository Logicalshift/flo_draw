#include <metal_stdlib>

#import "./bindings/metal_vertex2d.h"
#import "rasterizer.metal"

float4 apply_eraser(
      float4                    color, 
      float2                    paper_coord,
      metal::texture2d_ms<half> eraser_texture) {
    // Work out the coordinates in the eraser texture (which applies to the whole screen)
    paper_coord[0]              *= float(eraser_texture.get_width());
    paper_coord[1]              *= float(eraser_texture.get_height());

    // Sample the eraser
    const uint num_samples      = eraser_texture.get_num_samples();
    const uint2 eraser_coord    = uint2(paper_coord);
    half eraser_total           = 0;

    for (uint sample_num=0; sample_num<num_samples; ++sample_num) {
        const half4 sample      = eraser_texture.read(eraser_coord, sample_num);
        eraser_total            += sample[0];
    }

    // Adjust the color according to the erase texture at this point
    float eraser_alpha          = float(eraser_total) / float(num_samples);

    color[0]                    *= 1-eraser_alpha;
    color[1]                    *= 1-eraser_alpha;
    color[2]                    *= 1-eraser_alpha;
    color[3]                    *= 1-eraser_alpha;

    return color;
}
