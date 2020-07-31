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
    vec4 eraseColor;
    eraseColor = texelFetch(t_EraseMask, ivec2(0,0), 3);

    f_Color[0] *= eraseColor[0];
    f_Color[1] *= eraseColor[0];
    f_Color[2] *= eraseColor[0];
    f_Color[3] *= eraseColor[0];
#endif
}
