use glam::{DMat4, DQuat, DVec3};
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use wgpu::{
    BindGroup, BindGroupLayout, Buffer, BufferAddress, CompareFunction, DepthBiasState,
    RenderPipeline, StencilState, VertexBufferLayout, util::DeviceExt,
};

use crate::attenuation::{Attenuation, RANGE_50};
use crate::graphics::SharedGraphicsContext;
use crate::model::{LazyModel, LazyType};
use crate::{
    camera::Camera,
    entity::Transform,
    graphics::Shader,
    model::{self, Model, Vertex},
};

pub const MAX_LIGHTS: usize = 8;

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
// ENSURE THAT THE SIZE OF THE UNIFORM IS OF A MULTIPLE OF 16. USE `size_of::<LightUniform>()`
pub struct LightUniform {
    pub position: [f32; 4],
    pub direction: [f32; 4], // outer cutoff is .w value
    pub colour: [f32; 4],    // last value is the light type
    // pub light_type: u32,
    pub constant: f32,
    pub linear: f32,
    pub quadratic: f32,
    pub cutoff: f32,
}

fn dvec3_to_uniform_array(vec: DVec3) -> [f32; 4] {
    [vec.x as f32, vec.y as f32, vec.z as f32, 1.0]
}

fn dvec3_colour_to_uniform_array(vec: DVec3, light_type: LightType) -> [f32; 4] {
    [
        vec.x as f32,
        vec.y as f32,
        vec.z as f32,
        light_type as u32 as f32,
    ]
}

fn dvec3_direction_to_uniform_array(vec: DVec3, outer_cutoff_angle: f32) -> [f32; 4] {
    [
        vec.x as f32,
        vec.y as f32,
        vec.z as f32,
        f32::cos(outer_cutoff_angle.to_radians()),
    ]
}

impl Default for LightUniform {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0, 1.0],
            direction: [0.0, 0.0, -1.0, 1.0],
            colour: [1.0, 1.0, 1.0, 1.0],
            // light_type: 0,
            constant: 0.0,
            linear: 0.0,
            quadratic: 0.0,
            cutoff: f32::cos(12.5_f32.to_radians()),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct LightArrayUniform {
    pub lights: [LightUniform; MAX_LIGHTS],
    pub light_count: u32,
    pub ambient_strength: f32,
    pub _padding: [u32; 2],
}

impl Default for LightArrayUniform {
    fn default() -> Self {
        Self {
            lights: [LightUniform::default(); MAX_LIGHTS],
            light_count: 0,
            ambient_strength: 0.1,
            _padding: [0; 2],
        }
    }
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, Hash)]
pub enum LightType {
    // Example: Sunlight
    Directional = 0,
    // Example: Lamp
    Point = 1,
    // Example: Flashlight
    Spot = 2,
}

impl Display for LightType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LightType::Directional => write!(f, "Directional"),
            LightType::Point => write!(f, "Point"),
            LightType::Spot => write!(f, "Spot"),
        }
    }
}

impl From<LightType> for u32 {
    fn from(val: LightType) -> Self {
        match val {
            LightType::Directional => 0,
            LightType::Point => 1,
            LightType::Spot => 2,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LightComponent {
    pub position: DVec3,          // point, spot
    pub direction: DVec3,         // directional, spot
    pub colour: DVec3,            // all
    pub light_type: LightType,    // all
    pub intensity: f32,           // all
    pub attenuation: Attenuation, // point, spot
    pub enabled: bool,            // all - light
    pub visible: bool,            // all - cube
    pub cutoff_angle: f32,        // spot
    pub outer_cutoff_angle: f32,  // spot
}

impl Default for LightComponent {
    fn default() -> Self {
        Self {
            position: DVec3::ZERO,
            direction: DVec3::new(0.0, 0.0, -1.0),
            colour: DVec3::ONE,
            light_type: LightType::Point,
            intensity: 1.0,
            attenuation: RANGE_50,
            enabled: true,
            cutoff_angle: 12.5,
            outer_cutoff_angle: 17.5,
            visible: true,
        }
    }
}

impl LightComponent {
    pub fn new(
        colour: DVec3,
        light_type: LightType,
        intensity: f32,
        attenuation: Option<Attenuation>,
    ) -> Self {
        Self {
            position: Default::default(),
            direction: Default::default(),
            colour,
            light_type,
            intensity,
            attenuation: attenuation.unwrap_or(RANGE_50),
            enabled: true,
            cutoff_angle: 12.5,
            outer_cutoff_angle: 17.5,
            visible: true,
        }
    }

    pub fn directional(colour: DVec3, intensity: f32) -> Self {
        Self::new(colour, LightType::Directional, intensity, None)
    }

    pub fn point(colour: DVec3, intensity: f32, attenuation: Attenuation) -> Self {
        Self::new(colour, LightType::Point, intensity, Some(attenuation))
    }

    pub fn spot(colour: DVec3, intensity: f32) -> Self {
        Self::new(colour, LightType::Spot, intensity, None)
    }

    pub fn hide_cube(&mut self) {
        self.visible = false;
    }

    pub fn show_cube(&mut self) {
        self.visible = true;
    }

    pub fn disable_light(&mut self) {
        self.enabled = false;
    }

    pub fn enable_light(&mut self) {
        self.enabled = true;
    }
}

pub struct LazyLight {
    light_component: LightComponent,
    transform: Transform,
    label: Option<String>,
    cube_lazy_model: Option<LazyModel>,
}

impl LazyType for LazyLight {
    type T = Light;

    fn poke(self, graphics: Arc<SharedGraphicsContext>) -> anyhow::Result<Self::T> {
        let label_str = self.label.clone().unwrap_or_else(|| "Light".to_string());

        let forward = DVec3::new(0.0, 0.0, -1.0);
        let direction = self.transform.rotation * forward;

        let uniform = LightUniform {
            position: dvec3_to_uniform_array(self.transform.position),
            direction: dvec3_direction_to_uniform_array(
                direction,
                self.light_component.outer_cutoff_angle,
            ),
            colour: dvec3_colour_to_uniform_array(
                self.light_component.colour * self.light_component.intensity as f64,
                self.light_component.light_type,
            ),
            constant: self.light_component.attenuation.constant,
            linear: self.light_component.attenuation.linear,
            quadratic: self.light_component.attenuation.quadratic,
            cutoff: f32::cos(self.light_component.cutoff_angle.to_radians()),
        };

        let cube_model = Arc::new(if let Some(lazy_model) = self.cube_lazy_model {
            lazy_model.poke(graphics.clone())?
        } else {
            anyhow::bail!(
                "The light cube LazyModel has not been initialised yet. Use Light::new(/** params */).preload_cube_model() to preload it (which is required)"
            );
        });

        let buffer = graphics.create_uniform(uniform, self.label.as_deref());

        let layout = graphics
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                label: self.label.as_deref(),
            });

        let bind_group = graphics
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
                label: self.label.as_deref(),
            });

        let instance = Instance::new(
            self.transform.position,
            self.transform.rotation,
            DVec3::new(0.25, 0.25, 0.25),
        );

        let instance_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: self.label.as_deref().or(Some("instance buffer")),
                    contents: bytemuck::cast_slice(&[instance.to_raw()]),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });

        log::debug!("Created new light [{}]", label_str);

        Ok(Light {
            uniform,
            cube_model,
            label: label_str,
            buffer: Some(buffer),
            layout: Some(layout),
            bind_group: Some(bind_group),
            instance_buffer: Some(instance_buffer),
        })
    }
}

#[derive(Clone)]
pub struct Light {
    pub uniform: LightUniform,
    pub cube_model: Arc<Model>,
    pub label: String,
    buffer: Option<Buffer>,
    layout: Option<BindGroupLayout>,
    bind_group: Option<BindGroup>,
    pub instance_buffer: Option<Buffer>,
}

impl Light {
    pub async fn lazy_new(
        light_component: LightComponent,
        transform: Transform,
        label: Option<&str>,
    ) -> anyhow::Result<LazyLight> {
        let mut result = LazyLight {
            light_component,
            transform,
            label: label.map(|s| s.to_string()),
            cube_lazy_model: None,
        };
        if result.cube_lazy_model.is_none() {
            let lazy_model = Model::lazy_load(
                include_bytes!("../../resources/cube.glb").to_vec(),
                result.label.as_deref(),
            )
            .await?;
            result.cube_lazy_model = Some(lazy_model);
        }
        Ok(result)
    }

    pub async fn new(
        graphics: Arc<SharedGraphicsContext>,
        light: &LightComponent,
        transform: &Transform,
        label: Option<&str>,
    ) -> Self {
        let forward = DVec3::new(0.0, 0.0, -1.0);
        let direction = transform.rotation * forward;

        let uniform = LightUniform {
            position: dvec3_to_uniform_array(transform.position),
            direction: dvec3_direction_to_uniform_array(direction, light.outer_cutoff_angle),
            colour: dvec3_colour_to_uniform_array(
                light.colour * light.intensity as f64,
                light.light_type,
            ),
            constant: light.attenuation.constant,
            linear: light.attenuation.linear,
            quadratic: light.attenuation.quadratic,
            cutoff: f32::cos(light.cutoff_angle.to_radians()),
        };

        let cube_model = Arc::new(Model::load_from_memory(
            graphics.clone(),
            include_bytes!("../../resources/cube.glb").to_vec(),
            label,
        )
        .await
        .unwrap());

        let label_str = label.unwrap_or("Light").to_string();

        let buffer = graphics.create_uniform(uniform, label);

        let layout = graphics
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let bind_group = graphics
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                }],
                label,
            });

        let instance = Instance::new(
            transform.position,
            transform.rotation,
            DVec3::new(0.25, 0.25, 0.25),
        );

        let instance_buffer =
            graphics
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

        let forward = DVec3::new(0.0, 0.0, -1.0);
        let direction = transform.rotation * forward;
        self.uniform.direction =
            dvec3_direction_to_uniform_array(direction, light.outer_cutoff_angle);

        self.uniform.colour =
            dvec3_colour_to_uniform_array(light.colour * light.intensity as f64, light.light_type);
        self.uniform.constant = light.attenuation.constant;
        self.uniform.linear = light.attenuation.linear;
        self.uniform.quadratic = light.attenuation.quadratic;

        self.uniform.cutoff = f32::cos(light.cutoff_angle.to_radians());
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
        self.buffer.as_ref().unwrap()
    }
}

#[derive(Clone)]
pub struct LightManager {
    pub pipeline: Option<RenderPipeline>,
    light_array_buffer: Option<Buffer>,
    light_array_bind_group: Option<BindGroup>,
    light_array_layout: Option<BindGroupLayout>,
}

impl Default for LightManager {
    fn default() -> Self {
        Self::new()
    }
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

    pub fn create_light_array_resources(&mut self, graphics: Arc<SharedGraphicsContext>) {
        let layout = graphics
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

        let bind_group = graphics
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
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
        log::debug!("Created light array resources")
    }

    pub fn update(&mut self, graphics: Arc<SharedGraphicsContext>, world: &hecs::World) {
        let mut light_array = LightArrayUniform::default();
        let mut light_index = 0;

        for (_, (light_component, transform, light)) in world
            .query::<(&LightComponent, &Transform, &mut Light)>()
            .iter()
        {
            // if it fails to update, the cause it probably the ModelVertex or smth like that
            // note: its not.
            let instance = Instance::from_matrix(transform.matrix());

            if let Some(instance_buffer) = &light.instance_buffer {
                let instance_raw = instance.to_raw();
                graphics.queue.write_buffer(
                    instance_buffer,
                    0,
                    bytemuck::cast_slice(&[instance_raw]),
                );
            }

            if light_component.enabled && light_index < MAX_LIGHTS {
                light_array.lights[light_index] = *light.uniform();
                light_index += 1;
            }
        }

        light_array.light_count = light_index as u32;

        if let Some(buffer) = &self.light_array_buffer {
            graphics
                .queue
                .write_buffer(buffer, 0, bytemuck::cast_slice(&[light_array]));
        }

        log_once::debug_once!("LightUniform size = {}", size_of::<LightUniform>())
    }

    pub fn layout(&self) -> &BindGroupLayout {
        self.light_array_layout.as_ref().unwrap()
    }

    pub fn bind_group(&self) -> &BindGroup {
        self.light_array_bind_group.as_ref().unwrap()
    }

    pub fn create_render_pipeline(
        &mut self,
        graphics: Arc<SharedGraphicsContext>,
        shader_contents: &str,
        camera: &Camera,
        label: Option<&str>,
    ) {
        use crate::graphics::Shader;

        let shader = Shader::new(graphics.clone(), shader_contents, label);

        let pipeline = Self::create_render_pipeline_for_lighting(
            graphics,
            &shader,
            vec![camera.layout(), self.light_array_layout.as_ref().unwrap()],
            label,
        );

        self.pipeline = Some(pipeline);
        log::debug!("Created ECS light render pipeline");
    }

    fn create_render_pipeline_for_lighting(
        graphics: Arc<SharedGraphicsContext>,
        shader: &Shader,
        bind_group_layouts: Vec<&BindGroupLayout>,
        label: Option<&str>,
    ) -> RenderPipeline {
        let render_pipeline_layout =
            graphics
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some(label.unwrap_or("Light Render Pipeline Descriptor")),
                    bind_group_layouts: bind_group_layouts.as_slice(),
                    push_constant_ranges: &[],
                });

        graphics
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
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
