use std::{fs, path::PathBuf};

use egui::Context;
use glam::{DMat4, DQuat, DVec3, Mat3};
use image::GenericImageView;
// use nalgebra::{Matrix4, UnitQuaternion, Vector3};
use wgpu::{BindGroup, BindGroupLayout, Buffer, BufferAddress, BufferUsages, Color, CommandEncoder, CompareFunction, DepthBiasState, Device, Extent3d, LoadOp, Operations, RenderPass, RenderPassDepthStencilAttachment, RenderPipeline, Sampler, ShaderModule, StencilState, SurfaceConfiguration, TextureDescriptor, TextureFormat, TextureUsages, TextureView, TextureViewDescriptor, VertexBufferLayout, util::{BufferInitDescriptor, DeviceExt}};

use crate::{
    State,
    model::{self, Vertex},
};

pub struct Graphics<'a> {
    pub state: &'a mut State,
    pub view: &'a TextureView,
    pub encoder: &'a mut CommandEncoder,
    pub screen_size: (f32, f32),
}

pub const NO_TEXTURE: &'static [u8] = include_bytes!("../../resources/no-texture.png");
pub const NO_MODEL: &'static [u8] = include_bytes!("../../resources/error.glb");

impl<'a> Graphics<'a> {
    pub fn new(
        state: &'a mut State,
        view: &'a TextureView,
        encoder: &'a mut CommandEncoder,
    ) -> Self {
        let screen_size = (state.config.width as f32, state.config.height as f32);
        Self {
            state,
            view,
            encoder,
            screen_size,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.state.resize(width, height);
    }

    pub fn texture_bind_group(&mut self) -> &wgpu::BindGroupLayout {
        &self.state.texture_bind_layout
    }

    pub fn create_render_pipline(
        &mut self,
        shader: &Shader,
        bind_group_layouts: Vec<&BindGroupLayout>,
        label: Option<&str>,
    ) -> RenderPipeline {
        let render_pipeline_layout =
            self.state
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(label.unwrap_or("Render Pipeline Descriptor")),
                    bind_group_layouts: bind_group_layouts.as_slice(),
                    push_constant_ranges: &[],
                });

        let render_pipeline =
            self.state
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(label.unwrap_or("Render Pipeline")),
                    layout: Some(&render_pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: &shader.module,
                        entry_point: Some("vs_main"),
                        buffers: &[model::ModelVertex::desc(), InstanceRaw::desc()],
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
                        // cull_mode: Some(wgpu::Face::Back), // todo: change for improved performance
                        cull_mode: None,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: Texture::DEPTH_FORMAT,
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
                });
        log::debug!("Created new render pipeline");
        render_pipeline
    }

    pub fn get_egui_context(&mut self) -> Context {
        self.state.egui_renderer.context()
    }

    pub fn clear_colour(&mut self, color: Color) -> RenderPass<'static> {
        self.encoder
            .begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(color),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &self.state.depth_texture.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
            })
            .forget_lifetime()
    }

    pub fn create_uniform<T>(&self, uniform: T, label: Option<&str>) -> Buffer
    where
        T: bytemuck::Pod + bytemuck::Zeroable,
    {
        self.state.device.create_buffer_init(&BufferInitDescriptor {
            label,
            contents: bytemuck::cast_slice(&[uniform]),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        })
    }

    pub fn create_model_uniform_bind_group_layout(&self) -> BindGroupLayout {
        self.state
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("model_uniform_bind_group_layout"),
            })
    }
}

pub struct Shader {
    pub label: String,
    pub module: ShaderModule,
}

impl Shader {
    pub fn new(graphics: &Graphics, shader_file_contents: &str, label: Option<&str>) -> Self {
        let module = graphics
            .state
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label,
                source: wgpu::ShaderSource::Wgsl(shader_file_contents.into()),
            });

        log::debug!("Created new shader under the label: {:?}", label);

        Self {
            label: match label {
                Some(label) => label.into(),
                None => "shader".into(),
            },
            module,
        }
    }
}

#[derive(Clone)]
pub struct Texture {
    pub texture: wgpu::Texture,
    pub sampler: Sampler,
    pub size: Extent3d,
    pub view: TextureView,
    pub bind_group: Option<BindGroup>,
    pub layout: Option<BindGroupLayout>,
}

impl Texture {
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        config: &SurfaceConfiguration,
        device: &Device,
        label: Option<&str>,
    ) -> Self {
        let size = Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };

        let desc = TextureDescriptor {
            label: label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let texture = device.create_texture(&desc);

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });

        Self {
            texture,
            sampler,
            view,
            size,
            bind_group: None,
            layout: None,
        }
    }

    pub fn create_viewport_texture(
        config: &SurfaceConfiguration,
        device: &Device,
        label: Option<&str>,
    ) -> Self {
        let size = Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };

        let desc = TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let texture = device.create_texture(&desc);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor::default());

        Self {
            texture,
            sampler,
            view,
            size,
            bind_group: None,
            layout: None,
        }
    }

    pub fn layout(&self) -> &BindGroupLayout {
        self.layout.as_ref().unwrap()
    }

    pub fn bind_group(&self) -> &BindGroup {
        self.bind_group.as_ref().unwrap()
    }

    pub fn new(graphics: &Graphics, diffuse_bytes: &[u8]) -> Self {
        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.to_rgba8();

        let dimensions = diffuse_image.dimensions();
        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let diffuse_texture = graphics
            .state
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("diffuse_texture"),
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            });

        graphics.state.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &diffuse_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &diffuse_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let diffuse_texture_view = diffuse_texture.create_view(&TextureViewDescriptor::default());
        let diffuse_sampler = graphics
            .state
            .device
            .create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

        let diffuse_bind_group =
            graphics
                .state
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &graphics.state.texture_bind_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                        },
                    ],
                    label: Some("texture_bind_group"),
                });

        Self {
            bind_group: Some(diffuse_bind_group),
            layout: Some(graphics.state.texture_bind_layout.clone()),
            texture: diffuse_texture,
            sampler: diffuse_sampler,
            size: texture_size,
            view: diffuse_texture_view,
        }
    }

    pub fn new_with_sampler(graphics: &Graphics, diffuse_bytes: &[u8], address_mode: wgpu::AddressMode) -> Self {
        let diffuse_image = image::load_from_memory(diffuse_bytes).unwrap();
        let diffuse_rgba = diffuse_image.to_rgba8();

        let dimensions = diffuse_image.dimensions();
        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let diffuse_texture = graphics
            .state
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("diffuse_texture"),
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
                view_formats: &[],
            });

        graphics.state.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &diffuse_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &diffuse_rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            texture_size,
        );

        let diffuse_texture_view = diffuse_texture.create_view(&TextureViewDescriptor::default());
        let diffuse_sampler = graphics
            .state
            .device
            .create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: address_mode,
                address_mode_v: address_mode,
                address_mode_w: address_mode,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

        let diffuse_bind_group =
            graphics
                .state
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &graphics.state.texture_bind_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&diffuse_texture_view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&diffuse_sampler),
                        },
                    ],
                    label: Some("texture_bind_group"),
                });

        Self {
            texture: diffuse_texture,
            sampler: diffuse_sampler,
            view: diffuse_texture_view,
            size: texture_size,
            bind_group: Some(diffuse_bind_group),
            layout: Some(graphics.state.texture_bind_layout.clone()),
        }
    }

    pub async fn load_texture(graphics: &Graphics<'_>, path: &PathBuf) -> anyhow::Result<Texture> {
        let data = fs::read(path)?;
        Ok(Self::new(graphics, &data))
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
            normal: Mat3::from_quat(self.rotation.as_quat()).to_cols_array_2d(),
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
    normal: [[f32; 3]; 3],
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

                // normal
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 19]>() as wgpu::BufferAddress,
                    shader_location: 10,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: size_of::<[f32; 22]>() as wgpu::BufferAddress,
                    shader_location: 11,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
