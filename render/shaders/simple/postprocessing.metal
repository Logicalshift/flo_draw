#include <metal_stdlib>

#import "rasterizer.metal"

float4 invert_color_alpha(float4 col) {
    col[0] = 1 - ((1-col[0]) * col[3]);
    col[1] = 1 - ((1-col[1]) * col[3]);
    col[2] = 1 - ((1-col[2]) * col[3]);

    return col;
}

float4 multiply_alpha(float4 col) {
    col[0] = 1 - ((1-col[0]) * col[3]);
    col[1] = 1 - ((1-col[1]) * col[3]);
    col[2] = 1 - ((1-col[2]) * col[3]);

    return col;
}
