uniform sampler2D   t_Texture;
uniform sampler2D   t_FilterTexture;
uniform vec2        t_Scale;
out vec4            f_Color;

void main() {
    vec2 texture_size   = vec2(textureSize(t_Texture, 0));
    vec2 pos            = vec2(gl_FragCoord.x, gl_FragCoord.y) / texture_size;
    vec4 displace_col   = texture(t_FilterTexture, pos);

#ifdef PREMULTIPLIED_FILTER_SOURCE
    displace_col[0]     = displace_col[0] / displace_col[3];
    displace_col[1]     = displace_col[1] / displace_col[3];
#endif

    vec2 displacement   = vec2((displace_col[0]-0.5)*2.0, (displace_col[1]-0.5)*2.0)*t_Scale;
    displacement        = displacement * displace_col[3];

    f_Color             = texture(t_Texture, pos + displacement, 0);

#ifdef INVERT_COLOUR_ALPHA
    // Blend towards one as the alpha approaches 0 (used for the multiply blend mode)
    f_Color[0]  = 1 - ((1-f_Color[0]) * (f_Color[3]));
    f_Color[1]  = 1 - ((1-f_Color[1]) * (f_Color[3]));
    f_Color[2]  = 1 - ((1-f_Color[2]) * (f_Color[3]));
#endif
}
