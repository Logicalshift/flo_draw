uniform sampler2D   t_Texture;
uniform float       t_Offset[15];
uniform float       t_Weight[15];
out vec4            f_Color;

// See <https://www.rastergrid.com/blog/2010/09/efficient-gaussian-blur-with-linear-sampling/> for a description of how we use bilinear sampling here

// Horizontal and vertical blurs can be done in separate passes, and a blur can be increased to a larger radius by repeatedly applying the effect
void main() {
    vec2 size       = textureSize(t_Texture, 0);

    f_Color         = texture(t_Texture, vec2(gl_FragCoord) / size) * t_Weight[0];

    for (int idx=1; idx<15; ++idx) {
#ifdef FILTER_HORIZ
        f_Color     += texture(t_Texture, (vec2(gl_FragCoord) + vec2(t_Offset[idx], 0.0)) / size) * t_Weight[idx];
        f_Color     += texture(t_Texture, (vec2(gl_FragCoord) - vec2(t_Offset[idx], 0.0)) / size) * t_Weight[idx];
#else
        f_Color     += texture(t_Texture, (vec2(gl_FragCoord) + vec2(0.0, t_Offset[idx])) / size) * t_Weight[idx];
        f_Color     += texture(t_Texture, (vec2(gl_FragCoord) - vec2(0.0, t_Offset[idx])) / size) * t_Weight[idx];
#endif
    }

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
