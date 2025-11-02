// Shader for rendering the white cube for lighting (or diff depending on colour)

struct Light {
    position: vec4<f32>,
    direction: vec4<f32>, // x, y, z, outer_cutoff_angle
    color: vec4<f32>, // r, g, b, light_type (0, 1, 2)
    constant: f32,
    lin: f32,
    quadratic: f32,
    cutoff: f32,
}

struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@group(1) @binding(0)
var<uniform> light: Light;

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_main(
    model: VertexInput,
    instance: InstanceInput,
) -> VertexOutput {
    let model_matrix = mat4x4<f32>(
        instance.model_matrix_0,
        instance.model_matrix_1,
        instance.model_matrix_2,
        instance.model_matrix_3,
    );
    var out: VertexOutput;
    out.clip_position = camera.view_proj * model_matrix * vec4<f32>(model.position, 1.0);
    out.color = light.color.xyz;
    return out;
}

// Fragment shader

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color, 1.0);
}