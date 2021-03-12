layout (location = 0) in vec2 a_Pos;
layout (location = 1) in vec2 a_TexCoord;
layout (location = 2) in vec4 a_Color;

uniform mat4 transform;
uniform mat4 texture_transform;

out VS_OUTPUT {
    vec2 v_TexCoord;
    vec2 v_PaperCoord;
} OUT;

void main() {
    vec4 texCoord       = vec4(a_Pos, 0.0, 1.0) * texture_transform;

    OUT.v_TexCoord      = vec2(texCoord[0], texCoord[1]);
    gl_Position         = vec4(a_Pos, 0.0, 1.0) * transform;
    OUT.v_PaperCoord    = vec2((gl_Position[0]+1.0)/2.0, (gl_Position[1]+1.0)/2.0);
}
