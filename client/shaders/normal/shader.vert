#version 460

in vec3 pos;
in vec4 col;
in vec3 norm;
in mat4 model;

out vec3 frag_pos;
out vec4 obj_col;
out vec3 frag_norm;

// camera attributes
uniform mat4 view;
uniform mat4 proj;


void main() {
    // frag position to world space
    vec4 world_pos = model * vec4(pos, 1.0);
    frag_pos = vec3(world_pos);

    // set object color
    obj_col = col;

    // normal to world space
    frag_norm = normalize(mat3(transpose(inverse(model))) * norm);

    // frag position to clip space
    gl_Position = proj * view * world_pos;
}