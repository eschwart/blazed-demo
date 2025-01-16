#version 460

out vec4 frag_col;

uniform vec4 obj_col;


void main() {
    frag_col = obj_col;
}