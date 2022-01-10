#version 330 core

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

    vec4 avg        = (sample1 + sample2 + sample3 + sample4) / 4.0 * t_Alpha;

    f_Color         = avg;
}
