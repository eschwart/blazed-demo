#version 460

in vec3 pos;

out vec3 frag_pos;

uniform mat4 model;
uniform mat4 view;
uniform mat4 proj;


void main() {
    // frag position to world space
    vec4 world_pos = model * vec4(pos, 1.0);
    frag_pos = vec3(world_pos);

    // frag position to clip space
    gl_Position = proj * view * world_pos;
}
