///
/// For blending a multi-sample texture onto another texture: this will map the equivalent coordinates of the fragments from the source texture
///

uniform sampler2DMS t_SourceTexture;
uniform float t_Alpha;

out vec4 f_Color;

void main() {
    ivec2 pos       = ivec2(gl_FragCoord.x, gl_FragCoord.y);

    vec4 sample1    = texelFetch(t_SourceTexture, pos, 0);
    vec4 sample2    = texelFetch(t_SourceTexture, pos, 1);
    vec4 sample3    = texelFetch(t_SourceTexture, pos, 2);
    vec4 sample4    = texelFetch(t_SourceTexture, pos, 3);

    vec4 avg        = (sample1 + sample2 + sample3 + sample4) / 4.0;
    avg             *= t_Alpha;

    f_Color         = avg;

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
