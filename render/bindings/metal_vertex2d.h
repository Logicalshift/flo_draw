#include <simd/vector_types.h>
#include <simd/matrix_types.h>
#include "metal_bindings.h"

typedef struct MetalVertex2D {
    vector_float2 pos;
    vector_float2 tex_coord;
    vector_uchar4 color;
} MetalVertex2D;
