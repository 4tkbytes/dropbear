use glam::DVec3;
use wgpu::{BindGroup, BindGroupLayout, Buffer, CompareFunction, DepthBiasState, RenderPass, RenderPipeline, StencilState};

use crate::{camera::Camera, entity::Transform, graphics::{Graphics, Shader}, model::{self, DrawLight, Model, Vertex}};


#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightUniform {
    pub position: [f32; 3],
    _padding: u32,
    pub colour: [f32; 3],
    pub light_type: u32
}

impl Default for LightUniform {
    fn default() -> Self {
        Self::new()
    }
}

impl LightUniform {
    pub fn new() -> Self {
        Self {
            position: [2.0, 2.0, 2.0],
            _padding: 0,
            colour: [1.0, 1.0, 1.0],
            light_type: 0,
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
    pub colour: DVec3,
    pub light_type: LightType,
    pub intensity: f32,
    pub enabled: bool,
}

impl Default for LightComponent {
    fn default() -> Self {
        Self {
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
    buffer: Option<Buffer>,
    layout: Option<BindGroupLayout>,
    bind_group: Option<BindGroup>,
    cube_model: Model,
    label: String,
}

impl Light {
    pub fn new(graphics: &Graphics, light: &LightComponent, transform: &Transform, label: Option<&str>) -> Self {
        let uniform = LightUniform {
            position: transform.position.as_vec3().to_array(),
            _padding: 0,
            colour: (light.colour * light.intensity as f64).as_vec3().to_array(),
            light_type: light.light_type.into(),
        };

        let buffer = graphics.create_uniform(uniform, label);

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
            label,
        });

        let bind_group = graphics.state.device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label,
        });

        let cube_model = Model::load_from_memory(
            graphics, 
            include_bytes!("../../resources/cube.obj"), 
            label
        ).unwrap();

        let label_str = label.unwrap_or("Light").to_string();
        log::debug!("Created new adopted light [{}]", label_str);

        if let Some(label) = label {
            log::debug!("Created new light [{}]", label);
        } else {
            log::debug!("Created new light");
        }
        
        Self {
            uniform,
            buffer: Some(buffer),
            layout: Some(layout),
            bind_group: Some(bind_group),
            cube_model,
            label: label_str,
        }
    }

    pub fn bind_group(&self) -> &BindGroup {
        self.bind_group.as_ref().unwrap()
    }

    pub fn layout(&self) -> &BindGroupLayout {
        self.layout.as_ref().unwrap()
    }

    pub fn model(&self) -> &Model {
        &self.cube_model
    }

    pub fn label(&self) -> &str {
        &self.label
    }

    pub fn buffer(&self) -> &Buffer {
        &self.buffer.as_ref().unwrap()
    }

    pub fn render<'a>(&'a self, render_pass: &'a mut RenderPass<'a>, component: &'a LightComponent, camera: &'a Camera) {
        if component.enabled {
            render_pass.draw_light_model(
                self.model(),
                camera.bind_group(), 
                self.bind_group()
            );
        }
    }
}

pub struct LightManager {
    pub pipeline: Option<RenderPipeline>,
}

impl LightManager {
    pub fn new() -> Self {
        log::debug!("Initialised lighting");
        Self {
            pipeline: None,
        }
    }

    pub fn create_render_pipeline(&mut self, graphics: &mut Graphics, shader_contents: &str, camera: &Camera, label: Option<&str>) {
        let shader = Shader::new(graphics, shader_contents, label);
        
        let dummy_layout = graphics.state.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            label: Some("Dummy Light Layout"),
        });

        let pipeline = Self::create_render_pipeline_for_lighting(
            graphics, 
            &shader, 
            vec![camera.layout(), &dummy_layout], 
            label
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
                buffers: &[model::ModelVertex::desc()],
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

    pub fn set_pipeline<'a>(
        &'a self,
        render_pass: &mut RenderPass<'a>, 
    ) {
        if let Some(pipeline) = &self.pipeline {
            render_pass.set_pipeline(pipeline);
        } else {
            log::error!("No pipeline found");
        }
    }
}