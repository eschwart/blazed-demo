#version 460

in vec4 obj_col;

out vec4 frag_col;


void main() {
    frag_col = obj_col;
}