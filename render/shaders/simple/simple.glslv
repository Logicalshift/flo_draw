#version 330 core

layout (location = 0) in vec2 a_Pos;
layout (location = 1) in vec2 a_TexCoord;
layout (location = 2) in vec4 a_Color;

uniform mat4 transform;

out VS_OUTPUT {
    vec4 v_Color;
    vec2 v_TexCoord;
} OUT;

void main() {
    OUT.v_Color     = vec4(a_Color[0]/255.0, a_Color[1]/255.0, a_Color[2]/255.0, a_Color[3]/255.0);
    OUT.v_TexCoord  = a_TexCoord;
    gl_Position     = vec4(a_Pos, 0.0, 1.0) * transform;
}
