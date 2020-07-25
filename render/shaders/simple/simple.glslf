#version 330 core

in VS_OUTPUT {
    vec4 v_Color;
    vec2 v_TexCoord;
} IN;

out vec4 f_Color;

void main() {
    f_Color = IN.v_Color;
}
