//! A straight plane (and some components). Thats it.
//!
//! Inspiration taken from `https://github.com/4tkbytes/RedLight/blob/main/src/RedLight/Entities/Plane.cs`,
//! my old game engine made in C sharp, where this is the plane "algorithm".

use crate::entity::MeshRenderer;
use crate::graphics::{SharedGraphicsContext, Texture};
use crate::model::{LoadedModel, MODEL_CACHE, Material, Mesh, Model, ModelId, ModelVertex};
use crate::utils::{ResourceReference, ResourceReferenceType};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use wgpu::{AddressMode, util::DeviceExt};

/// Creates a plane wrapped in a [`MeshRenderer`](crate::entity::MeshRenderer).
pub struct PlaneBuilder {
    width: f32,
    height: f32,
    tiles_x: u32,
    tiles_z: u32,
}

impl Default for PlaneBuilder {
    fn default() -> Self {
        Self::new()
    }
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

    pub async fn build(
        mut self,
        graphics: Arc<SharedGraphicsContext>,
        texture_bytes: &[u8],
        label: Option<&str>,
    ) -> anyhow::Result<MeshRenderer> {
        let label = if let Some(label) = label {
            label.to_string()
        } else {
            format!(
                "{}*{}_tx{}xtz{}_plane",
                self.width, self.height, self.tiles_x, self.tiles_z
            )
        };
        let mut hasher = DefaultHasher::new();
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
                let _ = position.iter().map(|v| (*v as i32).hash(&mut hasher));
                let _ = normal.iter().map(|v| (*v as i32).hash(&mut hasher));
                let _ = tex_coords.iter().map(|v| (*v as i32).hash(&mut hasher));

                vertices.push(ModelVertex {
                    position,
                    tex_coords,
                    normal,
                });
            }
        }

        indices.extend_from_slice(&[0, 2, 1, 1, 2, 3]);
        indices.hash(&mut hasher);

        let hash = hasher.finish();

        if let Some(cached_model) = MODEL_CACHE.lock().get(&label) {
            log::debug!("Model loaded from cache: {:?}", label);
            let handle = LoadedModel::new(Arc::clone(cached_model));
            return Ok(MeshRenderer::from_handle(handle));
        }

        let vertex_buffer = graphics
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(&format!("{:?} Vertex Buffer", label.clone())),
                contents: bytemuck::cast_slice(&vertices),
                usage: wgpu::BufferUsages::VERTEX,
            });
        let index_buffer = graphics
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

        let model = Arc::new(Model {
            label: label.clone(),
            path: ResourceReference::from_reference(ResourceReferenceType::Plane),
            meshes: vec![mesh],
            materials: vec![material],
            id: ModelId(hash),
        });

        MODEL_CACHE.lock().insert(label, Arc::clone(&model));

        let handle = LoadedModel::new(model);
        Ok(MeshRenderer::from_handle(handle))
    }
}
