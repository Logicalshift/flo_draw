in VS_OUTPUT {
    vec4 v_Color;
    vec2 v_TexCoord;
    vec2 v_PaperCoord;
} IN;

out vec4 f_Color;

// The dash pattern is a 1D mono texture
uniform sampler1D t_DashPattern;

#ifdef ERASE_MASK
uniform sampler2DMS t_EraseMask;
#endif

#ifdef CLIP_MASK
uniform sampler2DMS t_ClipMask;
#endif

void main() {
    f_Color = IN.v_Color;

    // The dash alpha values are stored in the red component (and we use only the x-coord of the texture coordinates to read it)
    float dash_alpha = texture(t_DashPattern, IN.v_TexCoord[0])[0];

    // Adjust the color
    f_Color[0] *= dash_alpha;
    f_Color[1] *= dash_alpha;
    f_Color[2] *= dash_alpha;
    f_Color[3] *= dash_alpha;

#ifdef ERASE_MASK
    ivec2 eraseSize     = textureSize(t_EraseMask);
    
    float width         = float(eraseSize[0]);
    float height        = float(eraseSize[1]);
    float x             = IN.v_PaperCoord[0] * width;
    float y             = IN.v_PaperCoord[1] * height;

    ivec2 pos           = ivec2(int(x), int(y));
    float eraseColor    = 0.0;

    for (int i=0; i<4; ++i) {
        eraseColor += texelFetch(t_EraseMask, pos, i)[0];
    }

    eraseColor /= 4.0;
    eraseColor = 1.0-eraseColor;

    f_Color[0] *= eraseColor;
    f_Color[1] *= eraseColor;
    f_Color[2] *= eraseColor;
    f_Color[3] *= eraseColor;
#endif

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
    clipColor = 1.0-clipColor;

    f_Color[0] *= clipColor;
    f_Color[1] *= clipColor;
    f_Color[2] *= clipColor;
    f_Color[3] *= clipColor;
#endif
}
