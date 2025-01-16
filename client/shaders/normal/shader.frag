#version 460

in vec3 frag_pos;
in vec3 frag_norm;

out vec4 frag_col;

uniform vec4 obj_col;
uniform vec3 view_pos;

uniform vec3 light_pos;
uniform vec3 light_col;


// ambient shading property
vec3 get_ambient(float strength, vec3 light_col) {
    vec3 ambient = strength * light_col;
    return ambient;
}

// diffuse shading property
vec3 get_diffuse(vec3 frag_norm, vec3 light_dir, vec3 light_col) {
    float diff = max(dot(frag_norm, light_dir), 0.0);
    vec3 diffuse = light_col * diff;
    return diffuse;
}

// specular shading property
vec3 get_specular(float strength, vec3 view_pos, vec3 frag_pos, vec3 frag_norm, vec3 light_dir, vec3 light_col) {
    vec3 view_dir = normalize(view_pos - frag_pos);
    vec3 reflect_dir = reflect(-light_dir, frag_norm);  
    float spec = pow(max(dot(view_dir, reflect_dir), 0.0), 16);
    vec3 specular = strength * spec * light_col;
    return specular;
}

// standard distance attenuation
float get_attenuation(vec3 frag_to_light) {
    // distance between light and fragment
    float d = length(frag_to_light);

    // long range configuration
    float c = 1.0;   // constant
    float l = 0.09;  // linear
    float q = 0.032; // quadratic

    // standard attenuation formula
    float att = 1.0 / (c + l * d + q * (d * d));

    return clamp(att, 0.0, 1.0);
}


void main() {
    // difference between light and fragment vectors
    vec3 frag_to_light = light_pos - frag_pos;

    // direction towards light from fragment position
    vec3 light_dir = normalize(frag_to_light);

    // ambient
    vec3 ambient = get_ambient(0.1, light_col);

    // diffuse
    vec3 diffuse = get_diffuse(frag_norm, light_dir, light_col);

    // specular
    vec3 specular = get_specular(0.5, view_pos, frag_pos, frag_norm, light_dir, light_col);

    // attenuation
    float att = get_attenuation(frag_to_light);

    // putting everything together
    vec3 rgb = ((ambient + diffuse + specular) * obj_col.rgb) * att;
    float alpha = obj_col.a;

    frag_col = vec4(rgb, alpha);
}