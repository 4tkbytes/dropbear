/// A straight plane (and some components). Thats it.
///
/// Inspiration taken from `https://github.com/4tkbytes/RedLight/blob/main/src/RedLight/Entities/Plane.cs`,
/// my old game engine made in C sharp, where this is the plane "algorithm".
use std::sync::Arc;
use crate::entity::AdoptedEntity;
use crate::graphics::{SharedGraphicsContext, Texture};
use crate::model::{Material, Mesh, Model, ModelVertex};
use crate::utils::{ResourceReference, ResourceReferenceType};
use futures::executor::block_on;
use image::GenericImageView;
use wgpu::{AddressMode, util::DeviceExt};

/// Lazily creates a new Plane. This can only be accessed through the Default trait (which you shouldn't use),
/// or the [`PlaneBuilder::lazy_build()`] (also taken from [`PlaneBuilder::new()`]).
#[derive(Default)]
pub struct LazyPlaneBuilder {
    rgba_data: Vec<u8>,
    dimensions: (u32, u32),
    width: f32,
    height: f32,
    tiles_x: u32,
    tiles_z: u32,
    label: Option<String>,
}

impl LazyPlaneBuilder {
    pub fn poke(self, graphics: Arc<SharedGraphicsContext>) -> anyhow::Result<AdoptedEntity> {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for z in 0..=1 {
            for x in 0..=1 {
                let position = [
                    (x as f32 - 0.5) * self.width,
                    0.0,
                    (z as f32 - 0.5) * self.height,
                ];
                let normal = [0.0, 1.0, 0.0];
                let tex_coords = [
                    x as f32 * self.tiles_x as f32,
                    z as f32 * self.tiles_z as f32,
                ];
                vertices.push(ModelVertex {
                    position,
                    tex_coords,
                    normal,
                });
            }
        }

        indices.extend_from_slice(&[0, 2, 1, 1, 2, 3]);

        let vertex_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer", self.label.as_deref())),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
        let index_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Index Buffer", self.label.as_deref())),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

        let mesh = Mesh {
            name: "plane".to_string(),
            vertex_buffer,
            index_buffer,
            num_elements: indices.len() as u32,
            material: 0,
        };

        let diffuse_texture = Texture::new_with_sampler_with_rgba_buffer(
            graphics.clone(),
            &self.rgba_data,
            self.dimensions,
            AddressMode::Repeat
        );
        let bind_group = diffuse_texture.bind_group().clone();
        let material = Material {
            name: "plane_material".to_string(),
            diffuse_texture,
            bind_group,
        };

        let model = Model {
            label: self.label.as_deref().unwrap_or("Plane").to_string(),
            path: ResourceReference::from_reference(ResourceReferenceType::Plane),
            meshes: vec![mesh],
            materials: vec![material],
        };

        Ok(block_on(AdoptedEntity::adopt(graphics, model)))
    }
}

/// Creates a plane in the form of an AdoptedEntity. 
pub struct PlaneBuilder {
    width: f32,
    height: f32,
    tiles_x: u32,
    tiles_z: u32,
}

impl PlaneBuilder {
    pub fn new() -> Self {
        Self {
            width: 10.0,
            height: 10.0,
            tiles_x: 0,
            tiles_z: 0,
        }
    }

    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }

    pub fn with_tiles(mut self, tiles_x: u32, tiles_z: u32) -> Self {
        self.tiles_x = tiles_x;
        self.tiles_z = tiles_z;
        self
    }

    pub async fn lazy_build(
        mut self,
        texture_bytes: &[u8],
        label: Option<&str>,
    ) -> anyhow::Result<LazyPlaneBuilder> {
        if self.tiles_x == 0 && self.tiles_z == 0 {
            self.tiles_x = self.width as u32;
            self.tiles_z = self.height as u32;
        }

        let img = image::load_from_memory(texture_bytes)?;
        let rgba = img.to_rgba8();
        let dimensions = img.dimensions();

        Ok(LazyPlaneBuilder {
            rgba_data: rgba.into_raw(),
            dimensions,
            width: self.width,
            height: self.height,
            tiles_x: self.tiles_x,
            tiles_z: self.tiles_z,
            label: label.map(|s| s.to_string()),
        })
    }

    pub async fn build(
        mut self,
        graphics: Arc<SharedGraphicsContext>,
        texture_bytes: &[u8],
        label: Option<&str>,
    ) -> anyhow::Result<AdoptedEntity> {
        if self.tiles_x == 0 && self.tiles_z == 0 {
            self.tiles_x = self.width as u32;
            self.tiles_z = self.height as u32;
        }
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for z in 0..=1 {
            for x in 0..=1 {
                let position = [
                    (x as f32 - 0.5) * self.width,
                    0.0,
                    (z as f32 - 0.5) * self.height,
                ];
                let normal = [0.0, 1.0, 0.0];
                let tex_coords = [
                    x as f32 * self.tiles_x as f32,
                    z as f32 * self.tiles_z as f32,
                ];
                vertices.push(ModelVertex {
                    position,
                    tex_coords,
                    normal,
                });
            }
        }

        indices.extend_from_slice(&[0, 2, 1, 1, 2, 3]);

        let vertex_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Vertex Buffer", label)),
                    contents: bytemuck::cast_slice(&vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });
        let index_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{:?} Index Buffer", label)),
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

        let mesh = Mesh {
            name: "plane".to_string(),
            vertex_buffer,
            index_buffer,
            num_elements: indices.len() as u32,
            material: 0,
        };

        let diffuse_texture =
            Texture::new_with_sampler(graphics.clone(), texture_bytes, AddressMode::Repeat);
        let bind_group = diffuse_texture.bind_group().clone();
        let material = Material {
            name: "plane_material".to_string(),
            diffuse_texture,
            bind_group,
        };

        let model = Model {
            label: label.unwrap_or("Plane").to_string(),
            path: ResourceReference::from_reference(ResourceReferenceType::Plane),
            meshes: vec![mesh],
            materials: vec![material],
        };

        Ok(AdoptedEntity::adopt(graphics, model).await)
    }
}
