use std::collections::HashMap;

use glam::{DQuat, DVec3};
use wgpu::{BindGroup, BindGroupLayout, Buffer, CompareFunction, DepthBiasState, RenderPass, RenderPipeline, StencilState};

use crate::{camera::Camera, entity::AdoptedEntity, graphics::{Graphics, Shader}, model::{self, DrawLight, Model, Vertex}};


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

#[derive(Clone, Copy)]
pub enum LightType {
    Ambient = 0,
    Diffuse = 1,
}

impl Into<u32> for LightType {
    fn into(self) -> u32 {
        match self {
            LightType::Ambient => 0,
            LightType::Diffuse => 1,
        }
    }
}

pub struct Light {
    pub label: String,
    pub position: DVec3,
    pub colour: DVec3,
    pub light_type: LightType,

    pub uniform: LightUniform,
    // cannot be public, must be accessed with helper funcs
    buffer: Option<Buffer>,
    layout: Option<BindGroupLayout>,
    bind_group: Option<BindGroup>,
}

impl Light {
    pub fn new(graphics: &Graphics, position: DVec3, colour: DVec3, light_type: LightType, label: Option<&str>) -> Self {
        let uniform = LightUniform {
            position: position.as_vec3().to_array(),
            _padding: 0,
            colour: colour.as_vec3().to_array(),
            light_type: light_type.into(),
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

        if let Some(label) = label {
            log::debug!("Created new light [{}]", label);
        } else {
            log::debug!("Created new light");
        }
        
        Self {
            label: label.unwrap_or("Light").to_string(),
            position,
            colour,
            uniform,
            buffer: Some(buffer),
            layout: Some(layout),
            bind_group: Some(bind_group),
            light_type: light_type,
        }
    }

    pub fn uniform_buffer(&self) -> &Buffer {
        self.buffer.as_ref().unwrap()
    }

    pub fn layout(&self) -> &BindGroupLayout {
        self.layout.as_ref().unwrap()
    }

    pub fn bind_group(&self) -> &BindGroup {
        self.bind_group.as_ref().unwrap()
    }

    pub fn update(&mut self, graphics: &Graphics) {
        let old_position = self.position;
        self.uniform.position = (DQuat::from_axis_angle(DVec3::Y, 1.0_f64.to_radians()) * old_position).as_vec3().to_array();
        if let Some(buffer) = &self.buffer {
            graphics.state.queue.write_buffer(buffer, 0, bytemuck::cast_slice(&[self.uniform]));
        } else {
            log::error!("Writing to buffer failed: No buffer available/created for light [{}]", self.label);
        }
    }
}

pub struct LightManager {
    pub lights: HashMap<String, Light>,
    pub pipeline: Option<RenderPipeline>,
    cube: AdoptedEntity,
}

impl LightManager {
    pub fn new() -> Self {
        log::debug!("Initialised lighting");
        Self {
            lights: HashMap::new(),
            pipeline: None,
            cube: AdoptedEntity::default(),
        }
    }

    pub fn create_render_pipeline(&mut self, graphics: &mut Graphics, light_name: &str, shader_contents: &str, camera: &Camera, label: Option<&str>) {
        self.cube = AdoptedEntity::adopt(graphics, Model::load_from_memory(graphics, include_bytes!("../../resources/cube.obj"), Some("Cube")).unwrap(), Some("Cube"));
        if let Some(light) = self.get(light_name) {
            let light_pipeline = {
                let shader = Shader::new(graphics, shader_contents, label);
                Self::create_render_pipeline_for_lighting(graphics, &shader, vec![camera.layout(), light.layout()], label)
            };
            self.pipeline = Some(light_pipeline);
            log::debug!("Created new render pipeline")
        } else {
            log::warn!("No light was available to create render pipeline :(");
        }
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

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>, camera: &'a Camera) {
        render_pass.set_pipeline(&self.pipeline.as_ref().unwrap());
        for (_, light) in self.iter() {
            // todo: fix up error management in the model_load_from_mem function and the renderpass...
            render_pass.draw_light_model(&self.cube.model(), camera.bind_group(), light.bind_group());
        }
    }

    pub fn add(&mut self, name: impl Into<String>, light: Light) {
        self.lights.insert(name.into(), light);
    }

    pub fn remove(&mut self, name: &str) -> Option<Light> {
        self.lights.remove(name)
    }

    pub fn get(&self, name: &str) -> Option<&Light> {
        self.lights.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Light> {
        self.lights.get_mut(name)
    }

    pub fn has(&self, name: &str) -> bool {
        self.lights.contains_key(name)
    }

    pub fn clear(&mut self) {
        self.lights.clear();
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<'_, String, Light> {
        self.lights.iter()
    }

    pub fn iter_mut(&mut self) -> std::collections::hash_map::IterMut<'_, String, Light> {
        self.lights.iter_mut()
    }

    pub fn update_all(&mut self, graphics: &Graphics) {
        for (_name, light) in self.lights.iter_mut() {
            light.update(graphics);
        }
    }

    pub fn len(&self) -> usize {
        self.lights.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lights.is_empty()
    }
}