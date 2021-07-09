#include <metal_stdlib>

#import "rasterizer.metal"

float4 apply_clip_mask(
      float4                    color, 
      float2                    paper_coord,
      metal::texture2d_ms<half> clip_mask_texture) {
    // Work out the coordinates in the clip mask texture (which applies to the whole screen)
    paper_coord[0]              *= float(clip_mask_texture.get_width());
    paper_coord[1]              *= float(clip_mask_texture.get_height());

    // Sample the clip mask
    const uint num_samples      = clip_mask_texture.get_num_samples();
    const uint2 clip_coord      = uint2(paper_coord);
    half clip_mask_total        = 0;

    for (uint sample_num=0; sample_num<num_samples; ++sample_num) {
        const half4 sample      = clip_mask_texture.read(clip_coord, sample_num);
        clip_mask_total         += sample[0];
    }

    // Adjust the color according to the clip mask texture at this point
    float clip_mask_alpha       = float(clip_mask_total) / float(num_samples);

    color[0]                    *= clip_mask_alpha;
    color[1]                    *= clip_mask_alpha;
    color[2]                    *= clip_mask_alpha;
    color[3]                    *= clip_mask_alpha;

    return color;
}
