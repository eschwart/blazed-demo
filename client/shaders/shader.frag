#version 460

in vec3 frag_pos;
in vec3 frag_norm;

out vec4 frag_color;

uniform vec3 light_pos;
uniform vec4 color;

uniform float near; 
uniform float far; 

// Calculate depth
vec3 calculate_depth(float depth) {
    float z = depth * 2.0 - 1.0; // back to NDC 
    depth = (2.0 * near * far) / (far + near - z * (far - near));
    return vec3(pow(depth, 1.4));
}

// Calculate diffuse shading
float diffuse_scalar(vec3 normal, vec3 lightDir) {
    float diffuse = dot(normalize(lightDir), normalize(normal));
    diffuse = abs(diffuse);
    diffuse = diffuse / 2.0 + 0.5;
    return diffuse;
}

void main() {
    // direction of light
    vec3 light_dir = normalize(light_pos - frag_pos);

    // calculate depth
    vec3 depth = calculate_depth(gl_FragCoord.z) / far;

    // calculate diffuse shading
    float diffuse = diffuse_scalar(frag_norm, light_dir);

    frag_color = vec4((color.rgb * diffuse) * (1.0 - depth) + depth, color.a);
}