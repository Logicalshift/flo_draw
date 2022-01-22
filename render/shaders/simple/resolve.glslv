layout (location = 0) in vec2 a_Pos;
layout (location = 1) in vec2 a_TexCoord;
layout (location = 2) in vec4 a_Color;

void main() {
    gl_Position         = vec4(a_Pos, 0.0, 1.0);
}
