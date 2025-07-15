use nalgebra::{Matrix4, Vector3};
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, ShaderStages};

use crate::{buffer::Vertex, graphics::{Graphics, Texture}};

#[derive(Default)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    vertex_buffer: Option<Buffer>,
    index_buffer: Option<Buffer>,
    texture: Option<Texture>,
    pub uniform: MeshUniform,
    pub uniform_buffer: Option<Buffer>,
    layout: Option<BindGroupLayout>,
    bind_group: Option<BindGroup>,
}

impl Mesh {
    /// Creates a new mesh
    pub fn new(graphics: &Graphics, vertices: &[Vertex], indices: &[u16], image_bytes: &[u8]) -> Self {
        let vertex_buffer = graphics.create_vertex(vertices);
        let index_buffer = graphics.create_index(indices);
        let texture = Texture::new(graphics, image_bytes);

        let mut mesh = Self {
            vertex_buffer: Some(vertex_buffer),
            vertices: vertices.to_vec(),
            index_buffer: Some(index_buffer),
            indices: indices.to_vec(),
            texture: Some(texture),
            uniform: MeshUniform::new(),
            uniform_buffer: None,
            bind_group: None,
            layout: None,
        };

        let buffer = graphics.create_uniform(mesh.uniform, Some("model"));
        mesh.uniform_buffer = Some(buffer);

        let (layout, bind_group) = Mesh::create_model_bind_group(graphics, mesh.uniform_buffer());
        mesh.layout = Some(layout);
        mesh.bind_group = Some(bind_group);

        mesh
    }

    /// Creates a new mesh instance from existing components
    pub fn from(graphics: &Graphics, vertices: &[Vertex], indices: &[u16], vertex_buffer: Buffer, index_buffer: Buffer, texture: Texture) -> Self {
        let mut mesh = Self {
            vertex_buffer: Some(vertex_buffer),
            vertices: vertices.to_vec(),
            indices: indices.to_vec(),
            index_buffer: Some(index_buffer),
            texture: Some(texture),
            uniform: MeshUniform::new(),
            uniform_buffer: None,
            bind_group: None,
            layout: None,
        };

        mesh.uniform_buffer = Some(graphics.create_uniform(mesh.uniform, Some("model")));
        let (layout, bind_group) = Mesh::create_model_bind_group(graphics, mesh.uniform_buffer());
        mesh.layout = Some(layout);
        mesh.bind_group = Some(bind_group);
        mesh
    }

    // vertex_buffer: Option<Buffer>,
    // index_buffer: Option<Buffer>,
    // texture: Option<Texture>,

    pub fn update(&mut self, graphics: &Graphics) {
        graphics.state.queue.write_buffer(&self.uniform_buffer.as_ref().unwrap(), 0, bytemuck::cast_slice(&[self.uniform]));
    }

    pub fn vertex_buffer(&self) -> &Buffer {
        self.vertex_buffer.as_ref().unwrap()
    }

    pub fn index_buffer(&self) -> &Buffer {
        self.index_buffer.as_ref().unwrap()
    }

    pub fn texture(&self) -> &Texture {
        self.texture.as_ref().unwrap()
    }

    pub fn layout(&self) -> &BindGroupLayout {
        self.layout.as_ref().unwrap()
    }

    pub fn bind_group(&self) -> &BindGroup {
        self.bind_group.as_ref().unwrap()
    }

    pub fn create_model_bind_group(
        graphics: &Graphics,
        buffer: &Buffer,
    ) -> (BindGroupLayout, BindGroup) {
        let bind_group_layout = graphics.state.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX,
                ty: BindingType::Buffer {
                    ty: BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("model_bind_group_layout"),
        });

        let bind_group = graphics.state.device.create_bind_group(&BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("model_bind_group"),
        });

        (bind_group_layout, bind_group)
    }

    pub fn uniform_buffer(&self) -> &Buffer {
        self.uniform_buffer.as_ref().unwrap()
    }

    pub fn rotate_x(&mut self, angle_rad: f32) {
        let model_matrix = Matrix4::from(self.uniform.model);
        let rotated = model_matrix * Matrix4::new_rotation(Vector3::x() * angle_rad);
        self.uniform.model = rotated.into();
    }

    pub fn rotate_y(&mut self, angle_rad: f32) {
        let model_matrix = Matrix4::from(self.uniform.model);
        let rotated = model_matrix * Matrix4::new_rotation(Vector3::y() * angle_rad);
        self.uniform.model = rotated.into();
    }

    pub fn rotate_z(&mut self, angle_rad: f32) {
        let model_matrix = Matrix4::from(self.uniform.model);
        let rotated = model_matrix * Matrix4::new_rotation(Vector3::z() * angle_rad);
        self.uniform.model = rotated.into();
    }

    pub fn translate(&mut self, translation: Vector3<f32>) {
        let model_matrix = Matrix4::from(self.uniform.model);
        let translated = model_matrix * Matrix4::new_translation(&translation);
        self.uniform.model = translated.into();
    }

    pub fn scale(&mut self, scale: Vector3<f32>) {
        let model_matrix = Matrix4::from(self.uniform.model);
        let scaled = model_matrix * Matrix4::new_nonuniform_scaling(&scale);
        self.uniform.model = scaled.into();
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct MeshUniform {
    model: [[f32; 4]; 4],
}

impl MeshUniform {
    pub fn new() -> Self {
        Self {
            model: Matrix4::<f32>::identity().into(),
        }
    }
}