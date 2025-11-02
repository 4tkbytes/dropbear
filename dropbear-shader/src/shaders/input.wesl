const MAX_LIGHTS: u32 = 8;

struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

struct OutlineUniform {
    outline_width: f32,
    outline_color: vec4<f32>,
};

struct Light {
    position: vec4<f32>,
    direction: vec4<f32>, // x, y, z, outer_cutoff_angle
    color: vec4<f32>, // r, g, b, light_type (0, 1, 2)
    constant: f32,
    lin: f32,
    quadratic: f32,
    cutoff: f32,
}

struct LightArray {
    _lights: array<Light, config::MAX_LIGHTS>,
    light_count: u32,
    ambient_strength: f32,
}