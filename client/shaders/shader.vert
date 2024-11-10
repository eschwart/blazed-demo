#version 460

in vec3 pos;
in vec3 norm;

out vec3 frag_pos;
out vec3 frag_norm;

uniform mat4 model;
uniform mat4 view;
uniform mat4 projection;
uniform mat4 translation;
uniform mat4 rotation;

void main() {
    vec4 pos = vec4(pos, 1.0);

    // compute normal matrix from model
    mat3 norm_mat = transpose(inverse(mat3(model)));
    frag_norm = normalize(norm_mat * norm);

    // precompute model view matrix
    mat4 model_view = view * model;

    // compute world position
    frag_pos = vec3(model_view * pos);

    gl_Position = projection * model_view * translation * rotation * pos;
}
