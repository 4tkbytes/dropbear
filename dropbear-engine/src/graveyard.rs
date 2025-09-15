// This is the graveyard, where old code go to die
// They either served me for a long time, but outgrew their clothing, or they were useless and are there temporarily. 
// This mainly exists in the case that there is some code I could reference. 

pub struct Graphics<'a> {
    pub state: &'a mut State,
    pub view: &'a TextureView,
    pub encoder: &'a mut CommandEncoder,
    pub screen_size: (f32, f32),
    pub diffuse_sampler: Sampler,
}

impl<'a> Graphics<'a> {
    pub fn new(
        state: &'a mut State,
        view: &'a TextureView,
        encoder: &'a mut CommandEncoder,
    ) -> Self {
        let screen_size = (state.config.width as f32, state.config.height as f32);
        let diffuse_sampler = state
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
        Self {
            state,
            view,
            encoder,
            screen_size,
            diffuse_sampler,
        }
    }

    /// Fetches the [`FrameGraphicsContext`], which may be required for certain functions. 
    pub fn frame_context(&self) -> FrameGraphicsContext {
        FrameGraphicsContext {
            device: self.state.device.clone(),
            queue: self.state.queue.clone(),
            texture_bind_layout: Arc::new(self.state.texture_bind_layout.clone()),
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