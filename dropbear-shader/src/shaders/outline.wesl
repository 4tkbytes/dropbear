// outline.wgsl
import package::input::CameraUniform;
import package::input::OutlineUniform;

@group(1) @binding(0)
var<uniform> camera: CameraUniform;

@group(0) @binding(0)
var<uniform> outline: OutlineUniform;

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,

    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>, // unused, but must match vertex layout
    @location(2) normal: vec3<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
}

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

    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );

    // Transform normal to world space
    let world_normal = normalize(normal_matrix * model.normal);

    // Extrude position in WORLD SPACE
    let world_position = (model_matrix * vec4<f32>(model.position, 1.0)).xyz;
    let expanded_world_position = world_position + world_normal * outline.outline_width;

    var out: VertexOutput;
    out.clip_position = camera.view_proj * vec4<f32>(expanded_world_position, 1.0);
    return out;
}

@fragment
fn fs_main() -> @location(0) vec4<f32> {
    return outline.outline_color; // Use the uniform color!
}