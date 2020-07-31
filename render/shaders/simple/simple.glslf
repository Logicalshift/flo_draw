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
    ivec2 pos       = ivec2(IN.v_PaperCoord[0] * eraseSize[0], IN.v_PaperCoord[1] * eraseSize[1]);
    vec4 eraseColor = texelFetch(t_EraseMask, pos, 3);

    f_Color[0] *= eraseColor[0];
    f_Color[1] *= eraseColor[0];
    f_Color[2] *= eraseColor[0];
    f_Color[3] *= eraseColor[0];
#endif
}
