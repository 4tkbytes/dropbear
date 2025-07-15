use wgpu::Buffer;

use crate::{buffer::Vertex, graphics::{Graphics, Texture}};

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub texture: Texture,
}

impl Mesh {
    /// Creates a new mesh
    pub fn new(graphics: &Graphics, vertices: &[Vertex], indices: &[u16], image_bytes: &[u8]) -> Self {
        let vertex_buffer = graphics.create_vertex(vertices);
        let index_buffer = graphics.create_index(indices);
        let texture = Texture::new(graphics, image_bytes);

        Self {
            vertex_buffer,
            vertices: vertices.to_vec(),
            index_buffer,
            indices: indices.to_vec(),
            texture,
        }
    }

    /// Creates a new mesh instance from existing components
    pub fn from(vertices: &[Vertex], indices: &[u16], vertex_buffer: Buffer, index_buffer: Buffer, texture: Texture) -> Self {
        Self {
            vertex_buffer,
            vertices: vertices.to_vec(),
            indices: indices.to_vec(),
            index_buffer,
            texture
        }
    }
}