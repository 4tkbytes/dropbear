use wgpu::{util::DeviceExt, Buffer, Color, CommandEncoder, RenderPass, RenderPipeline, ShaderModule, TextureView};

use crate::{buffer::Vertex, State};

pub struct Graphics<'a> {
    pub state: &'a State,
    pub view: &'a TextureView,
    pub encoder: &'a mut CommandEncoder,
}

impl<'a> Graphics<'a> {
    pub fn new(state: &'a State, view: &'a TextureView, encoder: &'a mut CommandEncoder) -> Self {
        Self {
            state,
            view,
            encoder,
        }
    }

    pub fn start_rendering(&mut self, shader: &ShaderModule) -> RenderPipeline {
        let render_pipeline_layout = self.state.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Descriptor"),
            bind_group_layouts: &[],
            push_constant_ranges: &[]
        });

        let render_pipeline = self.state.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"), // 1.
                buffers: &[], // 2.
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState { // 3.
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState { // 4.
                    format: self.state.config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // 1.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // 2.
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None, // 1.
            multisample: wgpu::MultisampleState {
                count: 1, // 2.
                mask: !0, // 3.
                alpha_to_coverage_enabled: false, // 4.
            },
            multiview: None, // 5.
            cache: None, // 6.
        });
        render_pipeline
    }

    pub fn new_shader(&mut self, shader_contents: &str, label: Option<&str>) -> wgpu::ShaderModule {
        let shader = self.state.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label,
            source: wgpu::ShaderSource::Wgsl(shader_contents.into()),
        });
        shader
    }

    pub fn clear_colour(&mut self, color: Color) -> RenderPass<'static> {
        self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &self.view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(color),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        }).forget_lifetime()
    }
    
    pub fn create_buffer(&self, vertices: &[Vertex]) -> Buffer {
        let vertex_buffer = self.state.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        vertex_buffer
    }
}
