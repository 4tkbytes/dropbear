use glam::{DMat4, DQuat, DVec3, DVec4};
use wgpu::{BindGroup, BindGroupLayout, Buffer, CompareFunction, util::DeviceExt, DepthBiasState, RenderPipeline, StencilState, VertexBufferLayout, BufferAddress};

use crate::{camera::Camera, entity::Transform, graphics::{Graphics, Shader}, model::{self, Model, Vertex}};

pub const MAX_LIGHTS: usize = 8;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: [f32; 4],
    pub direction: [f32; 4],
    pub colour: [f32; 4], // last value is the light type
    // pub light_type: u32,
}

fn dvec3_to_uniform_array(vec: DVec3) -> [f32; 4] {
    [vec.x as f32, vec.y as f32, vec.z as f32, 1.0]
}

fn dvec3_colour_to_uniform_array(vec: DVec3, light_type: LightType) -> [f32; 4] {
    [vec.x as f32, vec.y as f32, vec.z as f32, light_type as u32 as f32]
}

impl Default for LightUniform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0, 1.0],
            direction: [0.0, 0.0, -1.0, 1.0],
            colour: [1.0, 1.0, 1.0, 1.0],
            // light_type: 0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightArrayUniform {
    pub lights: [LightUniform; MAX_LIGHTS],
    pub light_count: u32,
    pub _padding: [u32; 3],
}

impl Default for LightArrayUniform {
    fn default() -> Self {
        Self {
            lights: [LightUniform::default(); MAX_LIGHTS],
            light_count: 0,
            _padding: [0; 3],
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum LightType {
    // Example: Sunlight
    Directional = 0,
    // Example: Lamp
    Point = 1,
    // Example: Flashlight
    Spot = 2,
}

impl Into<u32> for LightType {
    fn into(self) -> u32 {
        match self {
            LightType::Directional => 0,
            LightType::Point => 1,
            LightType::Spot => 2,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LightComponent {
    pub position: DVec3,
    pub direction: DVec3,
    pub colour: DVec3,
    pub light_type: LightType,
    pub intensity: f32,
    pub enabled: bool,
}

impl Default for LightComponent {
    fn default() -> Self {
        Self {
            position: DVec3::ZERO,
            direction: DVec3::new(0.0, 0.0, -1.0),
            colour: DVec3::ONE,
            light_type: LightType::Directional,
            intensity: 1.0,
            enabled: true,
        }
    }
}

impl LightComponent {
    pub fn new(colour: DVec3, light_type: LightType, intensity: f32) -> Self {
        Self {
            position: Default::default(),
            direction: Default::default(),
            colour,
            light_type,
            intensity,
            enabled: true,
        }
    }

    pub fn directional(colour: DVec3, intensity: f32) -> Self {
        Self::new(colour, LightType::Directional, intensity)
    }

    pub fn point(colour: DVec3, intensity: f32) -> Self {
        Self::new(colour, LightType::Point, intensity)
    }

    pub fn spot(colour: DVec3, intensity: f32) -> Self {
        Self::new(colour, LightType::Spot, intensity)
    }
}

pub struct Light {
    pub uniform: LightUniform,
    cube_model: Model,
    pub label: String,
    buffer: Option<Buffer>,
    layout: Option<BindGroupLayout>,
    bind_group: Option<BindGroup>,
    pub instance_buffer: Option<Buffer>,
}

impl Light {
    pub fn new(graphics: &Graphics, light: &LightComponent, transform: &Transform, label: Option<&str>) -> Self {
        let uniform = LightUniform {
            position: dvec3_to_uniform_array(transform.position),
            // direction: transform.rotation.normalize().xyz().as_vec3().to_array(),
            colour: dvec3_colour_to_uniform_array(light.colour * light.intensity as f64, light.light_type),
            // light_type: light.light_type.into(),
            ..Default::default()
        };

        let cube_model = Model::load_from_memory(
            graphics, 
            include_bytes!("../../resources/cube.obj"), 
            label.clone()
        ).unwrap();

        let label_str = label.clone().unwrap_or("Light").to_string();

        let buffer = graphics.create_uniform(uniform, label.clone());

        let layout = graphics.state.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: label.clone(),
        });

        let bind_group = graphics.state.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: label.clone(),
        });

        let instance = Instance::new(transform.position, transform.rotation, DVec3::new(0.25, 0.25, 0.25));

        let instance_buffer =
            graphics
                .state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: match label {
                        Some(_) => label,
                        None => Some("instance buffer"),
                    },
                    contents: bytemuck::cast_slice(&[instance.to_raw()]),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });

        log::debug!("Created new light [{}]", label_str);

        Self {
            uniform,
            cube_model,
            label: label_str,
            buffer: Some(buffer),
            layout: Some(layout),
            bind_group: Some(bind_group),
            instance_buffer: Some(instance_buffer),
        }
    }

    pub fn update(&mut self, light: &mut LightComponent, transform: &Transform) {
        self.uniform.position = dvec3_to_uniform_array(transform.position);
        self.uniform.direction = dvec3_to_uniform_array(DVec3::from(transform.rotation.normalize().xyz().as_vec3()));
        self.uniform.colour = dvec3_colour_to_uniform_array(light.colour * light.intensity as f64, light.light_type);
    }

    pub fn uniform(&self) -> &LightUniform {
        &self.uniform
    }

    pub fn model(&self) -> &Model {
        &self.cube_model
    }

    pub fn label(&self) -> &str {
        &self.label
    }
    
    pub fn bind_group(&self) -> &BindGroup {
        self.bind_group.as_ref().unwrap()
    }

    pub fn layout(&self) -> &BindGroupLayout {
        self.layout.as_ref().unwrap()
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer.as_ref().unwrap()
    }
}

pub struct LightManager {
    pub pipeline: Option<RenderPipeline>,
    light_array_buffer: Option<Buffer>,
    light_array_bind_group: Option<BindGroup>,
    light_array_layout: Option<BindGroupLayout>,
}

impl LightManager {
    pub fn new() -> Self {
        log::info!("Initialised lighting");
        Self {
            pipeline: None,
            light_array_buffer: None,
            light_array_bind_group: None,
            light_array_layout: None,
        }
    }

    pub fn create_light_array_resources(&mut self, graphics: &Graphics) {
        let layout = graphics.state.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("Light Array Layout"),
        });

        let buffer = graphics.create_uniform(LightArrayUniform::default(), Some("Light Array"));

        let bind_group = graphics.state.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("Light Array Bind Group"),
        });

        self.light_array_layout = Some(layout);
        self.light_array_buffer = Some(buffer);
        self.light_array_bind_group = Some(bind_group);
    }

    pub fn update(&mut self, graphics: &Graphics, world: &hecs::World) {
        let mut light_array = LightArrayUniform::default();
        let mut light_index = 0;

        for (_, (light_component, transform, light)) in world
            .query::<(&LightComponent, &Transform, &mut Light)>()
            .iter()
        {
            // if it fails to update, the cause it probably the ModelVertex or smth like that
            // note: its not...
            let instance = Instance::from_matrix(transform.matrix());

            if let Some(instance_buffer) = &light.instance_buffer {
                let instance_raw = instance.to_raw();
                graphics.state.queue.write_buffer(
                    instance_buffer,
                    0,
                    bytemuck::cast_slice(&[instance_raw]),
                );
            }

            if light_component.enabled && light_index < MAX_LIGHTS {
                light_array.lights[light_index] = light.uniform().clone();
                light_index += 1;
            }
        }

        light_array.light_count = light_index as u32;

        if let Some(buffer) = &self.light_array_buffer {
            graphics.state.queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[light_array]));
        }

        log_once::debug_once!("LightUniform size = {}", size_of::<LightUniform>())
    }

    pub fn layout(&self) -> &BindGroupLayout {
        self.light_array_layout.as_ref().unwrap()
    }

    pub fn bind_group(&self) -> &BindGroup {
        self.light_array_bind_group.as_ref().unwrap()
    }

    pub fn create_render_pipeline(&mut self, graphics: &mut Graphics, shader_contents: &str, camera: &Camera, label: Option<&str>) {
        use crate::graphics::Shader;
        
        let shader = Shader::new(graphics, shader_contents, label.clone());
        
        let pipeline = Self::create_render_pipeline_for_lighting(
            graphics, 
            &shader, 
            vec![camera.layout(), self.light_array_layout.as_ref().unwrap()], 
            label.clone()
        );
        
        self.pipeline = Some(pipeline);
        log::debug!("Created ECS light render pipeline");
    }

    fn create_render_pipeline_for_lighting(graphics: &mut Graphics, shader: &Shader, bind_group_layouts: Vec<&BindGroupLayout>, label: Option<&str>) -> RenderPipeline {
        let render_pipeline_layout = graphics.state.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some(label.unwrap_or("Light Render Pipeline Descriptor")),
                bind_group_layouts: bind_group_layouts.as_slice(),
                push_constant_ranges: &[],
            });

        graphics.state.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader.module,
                entry_point: Some("vs_main"),
                buffers: &[model::ModelVertex::desc(), InstanceRaw::desc()],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader.module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState {
                        alpha: wgpu::BlendComponent::REPLACE,
                        color: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: CompareFunction::Greater,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        })
    }
}

#[derive(Default)]
pub struct Instance {
    pub position: DVec3,
    pub rotation: DQuat,
    pub scale: DVec3,

    buffer: Option<Buffer>,
}

impl Instance {
    pub fn new(position: DVec3, rotation: DQuat, scale: DVec3) -> Self {
        Self {
            position,
            rotation,
            scale,
            buffer: None,
        }
    }

    pub fn to_raw(&self) -> InstanceRaw {
        let model_matrix =
            DMat4::from_scale_rotation_translation(self.scale, self.rotation, self.position);
        InstanceRaw {
            model: model_matrix.as_mat4().to_cols_array_2d(),
        }
    }

    pub fn buffer(&self) -> &Buffer {
        self.buffer.as_ref().unwrap()
    }

    pub fn from_matrix(mat: DMat4) -> Self {
        let (scale, rotation, position) = mat.to_scale_rotation_translation();
        Instance {
            position,
            rotation,
            scale,
            buffer: None,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    model: [[f32; 4]; 4],
}

impl InstanceRaw {
    fn desc() -> VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: size_of::<InstanceRaw>() as BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // model
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}
