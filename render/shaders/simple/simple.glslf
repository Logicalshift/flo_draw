in VS_OUTPUT {
    vec4 v_Color;
    vec2 v_TexCoord;
    vec2 v_PaperCoord;
} IN;

out vec4 f_Color;

#ifdef ERASE_MASK
uniform sampler2DMS t_EraseMask;
#endif

void main() {
    f_Color = IN.v_Color;

#ifdef ERASE_MASK
    ivec2 eraseSize = textureSize(t_EraseMask);
    
    float width         = eraseSize[0];
    float height        = eraseSize[1];
    float x             = IN.v_PaperCoord[0] * width;
    float y             = IN.v_PaperCoord[1] * height;

    ivec2 pos           = ivec2(x, y);
    float eraseColor    = 0.0;

    for (int i=0; i<4; ++i) {
        eraseColor += texelFetch(t_EraseMask, pos, i)[0];
    }

    eraseColor /= 4.0;

    f_Color[0] *= 1-eraseColor;
    f_Color[1] *= 1-eraseColor;
    f_Color[2] *= 1-eraseColor;
    f_Color[3] *= 1-eraseColor;
#endif
}
