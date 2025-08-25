// Main shader for standard objects.
const MAX_LIGHTS: u32 = 8;

struct CameraUniform {
    view_pos: vec4<f32>,
    view_proj: mat4x4<f32>,
};
@group(1) @binding(0)
var<uniform> camera: CameraUniform;

struct Light {
    position: vec4<f32>,
    direction: vec4<f32>,
    color: vec4<f32>, // r, g, b, light_type (0, 1, 2)
    constant: f32,
    lin: f32,
    quadratic: f32,
//    ambient_strength: f32,
}

struct LightArray {
    lights: array<Light, MAX_LIGHTS>,
    light_count: u32,
    ambient_strength: f32,
}

@group(2) @binding(0)
var<uniform> light_array: LightArray;

struct InstanceInput {
    @location(5) model_matrix_0: vec4<f32>,
    @location(6) model_matrix_1: vec4<f32>,
    @location(7) model_matrix_2: vec4<f32>,
    @location(8) model_matrix_3: vec4<f32>,

    @location(9) normal_matrix_0: vec3<f32>,
    @location(10) normal_matrix_1: vec3<f32>,
    @location(11) normal_matrix_2: vec3<f32>,
};

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) tex_coords: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) world_position: vec3<f32>,
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
    let normal_matrix = mat3x3<f32>(
        instance.normal_matrix_0,
        instance.normal_matrix_1,
        instance.normal_matrix_2,
    );
    var out: VertexOutput;
    out.tex_coords = model.tex_coords;
    out.world_normal = normal_matrix * model.normal;
    var world_position: vec4<f32> = model_matrix * vec4<f32>(model.position, 1.0);
    out.world_position = world_position.xyz;
    out.clip_position = camera.view_proj * world_position;
    return out;
}

@group(0) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(0) @binding(1)
var s_diffuse: sampler;

fn calculate_light(light: Light, world_pos: vec3<f32>, world_normal: vec3<f32>, view_dir: vec3<f32>) -> vec3<f32> {
    let light_dir = normalize(light.position.xyz - world_pos);
    
    // dihfuse
    let diffuse_strength = max(dot(world_normal, light_dir), 0.0);
    let diffuse_color = light.color.xyz * diffuse_strength;
    
    // specular
    let half_dir = normalize(view_dir + light_dir);
    let specular_strength = pow(max(dot(world_normal, half_dir), 0.0), 32.0);
    let specular_color = specular_strength * light.color.xyz;
    
    return diffuse_color + specular_color;
}

fn directional_light(
    light: Light,
    world_normal: vec3<f32>,
    view_dir: vec3<f32>,
    tex_color: vec3<f32>
) -> vec3<f32> {
    let light_dir = normalize(-light.direction.xyz);

    let ambient = light.color.xyz * light_array.ambient_strength * tex_color;

    let diff = max(dot(world_normal, light_dir), 0.0);
    let diffuse = light.color.xyz * diff * tex_color;

    let reflect_dir = reflect(-light_dir, world_normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
    let specular = light.color.xyz * spec * tex_color;

    return ambient + diffuse + specular;
}

// https://learnopengl.com/code_viewer_gh.php?code=src/2.lighting/5.2.light_casters_point/5.2.light_casters.fs
// deal with later. current issue: it is showing only yellow and white in point light (weird...)
fn point_light(light: Light, world_pos: vec3<f32>, world_normal: vec3<f32>, view_dir: vec3<f32>, tex_color: vec3<f32>) -> vec3<f32> {
    let light_dir = normalize(light.position.xyz - world_pos);
    let distance = length(light.position.xyz - world_pos);

    let attenuation = 1.0 / (light.constant + light.lin * distance + light.quadratic * distance * distance);

    let ambient = light.color.xyz * light_array.ambient_strength * tex_color;

    let diff = max(dot(world_normal, light_dir), 0.0);
    let diffuse = light.color.xyz * diff * tex_color;

    let reflect_dir = reflect(-light_dir, world_normal);
    let spec = pow(max(dot(view_dir, reflect_dir), 0.0), 32.0);
    let specular = light.color.xyz * spec * tex_color;

    return (ambient + diffuse + specular) * attenuation;
}


@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var tex_color = textureSample(t_diffuse, s_diffuse, in.tex_coords);
    if (tex_color.a < 0.1) {
        discard;
    }

    var ambient_strength = light_array.ambient_strength; // 0.1
    let view_dir = normalize(camera.view_pos.xyz - in.world_position);
    let world_normal = normalize(in.world_normal);
    
    var final_color = vec3<f32>(0.0);
    var total_ambient = vec3<f32>(0.0);
    
    for (var i: u32 = 0u; i < light_array.light_count; i = i + 1u) {
        let light = light_array.lights[i];
        
        total_ambient += light.color.xyz * ambient_strength;

        // light type is color.w
        if light.color.w == 0.0 {
            // directional
            final_color += directional_light(light, world_normal, view_dir, tex_color.xyz);
        } else if light.color.w == 1.0 {
            // point
            final_color += point_light(light, in.world_position, world_normal, view_dir, tex_color.xyz);
        } else if light.color.w == 2.0 {
            // spot
//            final_color += spot_light(light, world_normal, view_dir);
        }
//        final_color += calculate_light(light, in.world_position, world_normal, view_dir);
    }
    
    final_color = (total_ambient + final_color) * tex_color.xyz;
    
    return vec4<f32>(final_color, tex_color.a);
}