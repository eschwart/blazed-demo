#version 460 core

in vec3 frag_pos;     // fragment position
in vec4 obj_col;      // current object color
in vec3 frag_norm;    // fragment normals

out vec4 frag_col;    // fragment color

uniform vec3 cam_pos; // camera's (eye) position

// Point-Light
struct P_Light {
    vec3 pos; // position
    vec3 col; // color
};

#define P_LIGHTS_MAX 16                 // TODO - figure out if we can make this dynamic?
uniform int p_lights_len;               // current number of point lights
uniform P_Light p_lights[P_LIGHTS_MAX]; // array of point lights

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

vec3 calc_ads(vec3 light_dir, vec3 frag_pos, vec3 frag_norm, vec3 light_col) {
    // ambient
    vec3 ambient = get_ambient(0.2, light_col);

    // diffuse
    vec3 diffuse = get_diffuse(frag_norm, light_dir, light_col);
    diffuse *= obj_col.rgb; // add the object's color

    // specular
    vec3 specular = get_specular(0.2, frag_pos, frag_norm, light_dir, light_col);

    return ambient + diffuse + specular;
}

/// standard point-light
vec3 calc_point_light(P_Light light) {
    vec3 light_pos = light.pos;
    vec3 light_col = light.col;

    // difference between light and fragment vectors
    vec3 frag_to_light = light_pos - frag_pos;

    // direction towards light from fragment position
    vec3 light_dir = normalize(frag_to_light);

    // ambient + diffuse + specular
    vec3 ads = calc_ads(light_dir, frag_pos, frag_norm, light_col);

    // attenuation
    float att = get_attenuation(frag_to_light);

    return ads * att;
}

void main() {
    vec3 rgb = vec3(0.0, 0.0, 0.0);

    // do the same for all point lights
    for (int i = 0; i < p_lights_len; i++)
        rgb += calc_point_light(p_lights[i]);

    frag_col = vec4(rgb, obj_col.a);
}