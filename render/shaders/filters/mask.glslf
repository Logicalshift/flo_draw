uniform sampler2D   t_Texture;
uniform sampler2D   t_FilterTexture;
out vec4            f_Color;

void main() {
    ivec2 pos           = ivec2(gl_FragCoord.x, gl_FragCoord.y);
    f_Color             = texelFetch(t_Texture, pos, 0);

    ivec2 tex_size      = textureSize(t_Texture, 0);
    vec2 alpha_pos      = vec2(float(pos[0]) / float(tex_size[0]), float(pos[1]) / float(tex_size[1]));
    float texture_alpha = texture(t_FilterTexture, alpha_pos)[3];

#ifdef INVERT_COLOUR_ALPHA
    // Blend towards one as the alpha approaches 0 (used for the multiply blend mode)
    f_Color[0]  = 1 - ((1-f_Color[0]) * (f_Color[3]));
    f_Color[1]  = 1 - ((1-f_Color[1]) * (f_Color[3]));
    f_Color[2]  = 1 - ((1-f_Color[2]) * (f_Color[3]));
#endif

    f_Color *= texture_alpha;
}
