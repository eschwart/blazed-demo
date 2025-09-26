#version 460 core

layout(location = 0) in vec3 pos;
layout(location = 1) in vec4 col;
layout(location = 2) in mat4 model; // takes up 4

out vec3 frag_pos;
out vec4 obj_col;

// camera attributes
layout(location = 0) uniform mat4 view;
layout(location = 1) uniform mat4 proj;


void main() {
    // frag position to world space
    vec4 world_pos = model * vec4(pos, 1.0);
    frag_pos = vec3(world_pos);

    // set object color
    obj_col = col;

    // frag position to clip space
    gl_Position = proj * view * world_pos;
}
