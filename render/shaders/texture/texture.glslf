in VS_OUTPUT {
    vec2 v_TexCoord;
    vec2 v_PaperCoord;
} IN;

out vec4 f_Color;

uniform sampler2D t_Texture;
uniform float texture_alpha;

#ifdef ERASE_MASK
uniform sampler2DMS t_EraseMask;
#endif

#ifdef CLIP_MASK
uniform sampler2DMS t_ClipMask;
#endif

void main() {
    f_Color             = texture(t_Texture, IN.v_TexCoord);

    f_Color[3]          *= texture_alpha;

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
}
