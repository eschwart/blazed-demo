#version 460

in vec3 frag_pos;
in vec3 frag_norm;

out vec4 frag_col;

uniform vec4 obj_col;
uniform vec3 cam_pos;

// TODO - figure out if we can make this dynamic?
#define NUM_MAX_LIGHTS 16

uniform int n_of_lights;
uniform vec3 light_pos[NUM_MAX_LIGHTS]; 
uniform vec3 light_col[NUM_MAX_LIGHTS];

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
vec3 get_specular(float strength, vec3 frag_pos, vec3 frag_norm, vec3 light_dir, vec3 light_col) {
    vec3 view_dir = normalize(cam_pos - frag_pos);
    vec3 reflect_dir = reflect(-light_dir, frag_norm);  
    float spec = pow(max(dot(view_dir, reflect_dir), 0.0), 16);
    vec3 specular = strength * spec * light_col;
    return specular;
}

// standard distance attenuation
float get_attenuation(vec3 frag_to_light) {
    // distance between light and fragment
    float d = length(frag_to_light);

    // Distance	Constant	Linear	Quadratic
    // 32	    1.0	        0.14	0.07
    float c = 1.0;   // constant
    float l = 0.14;  // linear
    float q = 0.07; // quadratic

    // standard attenuation formula
    float att = 1.0 / (c + l * d + q * (d * d));

    return clamp(att, 0.0, 1.0);
}

/// standard directional-light
vec3 calc_dir_light(vec3 dir, vec3 col) {
    // direction towards light from fragment position
    vec3 light_dir = normalize(-dir);

    // ambient
    vec3 ambient = get_ambient(0.2, col);

    // diffuse
    vec3 diffuse = get_diffuse(frag_norm, light_dir, col);

    // specular
    vec3 specular = get_specular(0.5, frag_pos, frag_norm, light_dir, col);

    return ambient + diffuse + specular;
}

/// standard point-light
vec3 calc_point_light(vec3 pos, vec3 col) {
    // difference between light and fragment vectors
    vec3 frag_to_light = pos - frag_pos;

    // direction towards light from fragment position
    vec3 light_dir = normalize(frag_to_light);

    // ambient
    vec3 ambient = get_ambient(0.2, col);

    // diffuse
    vec3 diffuse = get_diffuse(frag_norm, light_dir, col);

    // specular
    vec3 specular = get_specular(0.5, frag_pos, frag_norm, light_dir, col);

    // attenuation
    float att = get_attenuation(frag_to_light);

    return (ambient + diffuse + specular) * att;
}

void main() {
    vec3 lighting = vec3(0.0, 0.0, 0.0);
    lighting += calc_dir_light(vec3(-1.0, -0.2, 0.2), vec3(0.2, 0.2, 0.15));

    // using only first light for now
    // TODO - iterate through each light's point light
    // https://learnopengl.com/Lighting/Multiple-lights
    vec3 point = calc_point_light(light_pos[0], light_col[0]);
    lighting += point;

    // putting everything together
    vec3 rgb = lighting * obj_col.rgb;
    float alpha = obj_col.a;

    frag_col = vec4(rgb, alpha);
}