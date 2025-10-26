//! Extension module for the dropbear graphics that are editor specific.

use std::sync::Arc;
use dropbear_engine::colour::Colour;
use dropbear_engine::graphics::{InstanceRaw, Shader, SharedGraphicsContext, Texture};
use dropbear_engine::model::{ModelVertex, Vertex};

pub const OUTLINE_SHADER: &str = include_str!("../../resources/shaders/outline.wgsl");

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct OutlineUniform {
    pub outline_width: f32,
    pub outline_color: [f32; 4],
}

impl Default for OutlineUniform {
    fn default() -> Self {
        Self {
            outline_width: 0.02,
            outline_color: Colour::ORANGE.to_raw_vec4(),
        }
    }
}

impl OutlineUniform {
    pub fn new() -> Self {
        Self {
            outline_width: 0.0,
            outline_color: [0.0, 0.0, 0.0, 0.0],
        }
    }
}

pub struct OutlineShader {
    pub pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub uniform_buffer: wgpu::Buffer,
}

impl OutlineShader {
    pub fn init(graphics: Arc<SharedGraphicsContext>, camera_layout: &wgpu::BindGroupLayout) -> Self {
        let shader = Shader::new(graphics.clone(), OUTLINE_SHADER, Some("outline_shader"));
        log::trace!("Created outline shader");

        let bind_group_layout =
            graphics.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Outline Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        log::trace!("Created outline bind group layout");

        let uniform = OutlineUniform::new();
        let uniform_buffer = graphics.create_uniform(uniform, Some("Outline Uniform"));

        let bind_group = graphics.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Outline Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });
        log::trace!("Created outline bind group");

        let pipeline_layout = graphics.device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Outline Pipeline Layout"),
                bind_group_layouts: &[
                    &bind_group_layout,
                    &camera_layout,
                ],
                push_constant_ranges: &[],
            });
        log::trace!("Created outline pipeline layout");

        let pipeline = graphics.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Outline Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader.module,
                entry_point: Some("vs_main"),
                buffers: &[ModelVertex::desc(), InstanceRaw::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader.module,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Front),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: Texture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Greater,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });
        log::trace!("Created outline pipeline");

        log::debug!("Created outline render pipeline");
        Self {
            pipeline,
            bind_group_layout,
            bind_group,
            uniform_buffer,
        }
    }
}