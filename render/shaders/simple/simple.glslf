in VS_OUTPUT {
    vec4 v_Color;
    vec2 v_TexCoord;
    vec2 v_PaperCoord;
} IN;

out vec4 f_Color;

#ifdef ERASE_MASK
uniform sampler2DMS t_EraseMask;
#endif

#ifdef CLIP_MASK
uniform sampler2DMS t_ClipMask;
#endif

void main() {
    f_Color = IN.v_Color;

#ifdef CLIP_MASK
    ivec2 clipSize      = textureSize(t_ClipMask);
    
    float clipWidth     = float(clipSize[0]);
    float clipHeight    = float(clipSize[1]);
    float clipX         = IN.v_PaperCoord[0] * clipWidth;
    float clipY         = IN.v_PaperCoord[1] * clipHeight;

    ivec2 clipPos       = ivec2(int(clipX), int(clipY));
    float clipColor     = 0.0;

    for (int i=0; i<4; ++i) {
        clipColor += texelFetch(t_ClipMask, clipPos, i)[0];
    }

    clipColor /= 4.0;

    f_Color[0] *= clipColor;
    f_Color[1] *= clipColor;
    f_Color[2] *= clipColor;
    f_Color[3] *= clipColor;
#endif

#ifdef INVERT_COLOUR_ALPHA
    // Blend towards one as the alpha approaches 0 (used for the multiply blend mode)
    f_Color[0]  = 1 - ((1-f_Color[0]) * (f_Color[3]));
    f_Color[1]  = 1 - ((1-f_Color[1]) * (f_Color[3]));
    f_Color[2]  = 1 - ((1-f_Color[2]) * (f_Color[3]));
#endif

#ifdef MULTIPLY_ALPHA
    // This means that the input texture does not have pre-multiplied alpha but we want the output texture to be set up this way
    // This is used in particular for some blend modes (Multiply, Screen)
    f_Color[0]  *= f_Color[3];
    f_Color[1]  *= f_Color[3];
    f_Color[2]  *= f_Color[3];
#endif
}
