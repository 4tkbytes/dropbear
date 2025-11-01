use crate::graphics::{SharedGraphicsContext, Texture};
use crate::utils::ResourceReference;
use image::GenericImageView;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use rayon::prelude::*;
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::Arc;
use std::time::Instant;
use std::{mem, ops::Range, path::PathBuf};
use wgpu::{BufferAddress, VertexAttribute, VertexBufferLayout, util::DeviceExt};

pub const GREY_TEXTURE_BYTES: &[u8] = include_bytes!("../../resources/textures/grey.png");

lazy_static! {
    pub static ref MODEL_CACHE: Mutex<HashMap<String, Arc<Model>>> = Mutex::new(HashMap::new());
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ModelId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MaterialComponent(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshComponent(pub u64);

#[derive(Clone)]
pub struct Model {
    pub id: ModelId,
    pub label: String,
    pub path: ResourceReference,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

#[derive(Clone)]
pub struct LoadedModel {
    inner: Arc<Model>,
}

impl LoadedModel {
    pub fn new(inner: Arc<Model>) -> Self {
        Self { inner }
    }

    /// Returns the unique identifier of the underlying model asset.
    pub fn id(&self) -> ModelId {
        self.inner.id
    }

    /// Provides shared access to the underlying model.
    pub fn get(&self) -> Arc<Model> {
        Arc::clone(&self.inner)
    }

    /// Provides mutable access to the underlying model data, cloning if shared.
    pub fn make_mut(&mut self) -> &mut Model {
        Arc::make_mut(&mut self.inner)
    }
}

impl std::ops::Deref for LoadedModel {
    type Target = Model;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[derive(Clone)]
pub struct Material {
    pub name: String,
    pub diffuse_texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

#[derive(Clone)]
pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

impl Model {
    pub async fn load_from_memory(
        graphics: Arc<SharedGraphicsContext>,
        buffer: impl AsRef<[u8]>,
        label: Option<&str>,
    ) -> anyhow::Result<LoadedModel> {
        let start = Instant::now();
        let mut hasher = DefaultHasher::new();

        let cache_key = label.unwrap_or("default").to_string();

        if let Some(cached_model) = MODEL_CACHE.lock().get(&cache_key) {
            log::debug!("Model loaded from memory cache: {:?}", cache_key);
            return Ok(LoadedModel::new(cached_model.clone()));
        }

        log::trace!(
            "========== Benchmarking speed of loading {:?} ==========",
            label
        );
        log::debug!("Loading from memory");
        let res_ref = ResourceReference::from_bytes(buffer.as_ref());

        let (gltf, buffers, _images) = gltf::import_slice(buffer.as_ref())?;
        let mut meshes = Vec::new();

        let mut texture_data = Vec::new();
        for material in gltf.materials() {
            log::debug!("Processing material: {:?}", material.name());
            let material_name = material.name().unwrap_or("Unnamed Material").to_string();

            let image_data =
                if let Some(pbr) = material.pbr_metallic_roughness().base_color_texture() {
                    let texture_info = pbr.texture();
                    let image = texture_info.source();
                    match image.source() {
                        gltf::image::Source::View { view, mime_type: _ } => {
                            let buffer_data = &buffers[view.buffer().index()];
                            let start = view.offset();
                            let end = start + view.length();
                            buffer_data[start..end].to_vec()
                        }
                        gltf::image::Source::Uri { uri, mime_type: _ } => {
                            log::warn!("External URI textures not supported: {}", uri);
                            GREY_TEXTURE_BYTES.to_vec()
                        }
                    }
                } else {
                    GREY_TEXTURE_BYTES.to_vec()
                };

            texture_data.push((material_name, image_data));
        }

        if texture_data.is_empty() {
            texture_data.push(("Default".to_string(), GREY_TEXTURE_BYTES.to_vec()));
        }

        let parallel_start = Instant::now();
        let processed_textures: Vec<_> = texture_data
            .into_par_iter()
            .map(|(material_name, image_data)| {
                let material_start = Instant::now();

                let load_start = Instant::now();
                let diffuse_image = image::load_from_memory(&image_data).unwrap();
                log::trace!("Loading image to memory: {:?}", load_start.elapsed());

                let rgba_start = Instant::now();
                let diffuse_rgba = diffuse_image.to_rgba8();
                log::trace!(
                    "Converting diffuse image to rgba8 took {:?}",
                    rgba_start.elapsed()
                );

                let dimensions = diffuse_image.dimensions();

                log::trace!(
                    "Parallel processing of material '{}' took: {:?}",
                    material_name,
                    material_start.elapsed()
                );

                (material_name, diffuse_rgba.into_raw(), dimensions)
            })
            .collect();

        log::trace!(
            "Total parallel image processing took: {:?}",
            parallel_start.elapsed()
        );

        let mut materials = Vec::new();
        for (material_name, rgba_data, dimensions) in processed_textures {
            let start = Instant::now();

            let diffuse_texture =
                Texture::from_rgba_buffer(graphics.clone(), &rgba_data, dimensions);
            let bind_group = diffuse_texture.bind_group().to_owned();

            materials.push(Material {
                name: material_name,
                diffuse_texture,
                bind_group,
            });

            log::trace!("Time to create GPU texture: {:?}", start.elapsed());
        }

        for mesh in gltf.meshes() {
            log::debug!("Processing mesh: {:?}", mesh.name());
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));

                let positions: Vec<[f32; 3]> = reader
                    .read_positions()
                    .ok_or_else(|| anyhow::anyhow!("Mesh missing positions"))?
                    .collect();

                let normals: Vec<[f32; 3]> = reader
                    .read_normals()
                    .map(|iter| iter.collect())
                    .unwrap_or_else(|| vec![[0.0, 1.0, 0.0]; positions.len()]);

                let tex_coords: Vec<[f32; 2]> = reader
                    .read_tex_coords(0)
                    .map(|iter| iter.into_f32().collect())
                    .unwrap_or_else(|| vec![[0.0, 0.0]; positions.len()]);

                let vertices: Vec<ModelVertex> = positions
                    .iter()
                    .zip(normals.iter())
                    .zip(tex_coords.iter())
                    .map(|((pos, norm), tex)| ModelVertex {
                        position: *pos,
                        normal: *norm,
                        tex_coords: *tex,
                    })
                    .collect();
                for v in &vertices {
                    let _ = v.position.iter().map(|v| (*v as i32).hash(&mut hasher));
                    let _ = v.normal.iter().map(|v| (*v as i32).hash(&mut hasher));
                    let _ = v.tex_coords.iter().map(|v| (*v as i32).hash(&mut hasher));
                }

                let indices: Vec<u32> = reader
                    .read_indices()
                    .ok_or_else(|| anyhow::anyhow!("Mesh missing indices"))?
                    .into_u32()
                    .collect();
                indices.hash(&mut hasher);

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

                let material_index = primitive.material().index().unwrap_or(0);

                meshes.push(Mesh {
                    name: mesh.name().unwrap_or("Unnamed Mesh").to_string(),
                    vertex_buffer,
                    index_buffer,
                    num_elements: indices.len() as u32,
                    material: material_index,
                });
            }
        }

        log::debug!("Successfully loaded model [{:?}]", label);
        let model = Arc::new(Model {
            meshes,
            materials,
            label: label.unwrap_or("No named model").to_string(),
            path: res_ref,
            id: ModelId(hasher.finish()),
        });

        MODEL_CACHE
            .lock()
            .insert(cache_key.clone(), Arc::clone(&model));
        log::trace!("==================== DONE ====================");
        log::debug!("Model cached from memory: {:?}", label);
        log::debug!("Took {:?} to load model: {:?}", start.elapsed(), label);
        log::trace!("==============================================");
        Ok(LoadedModel::new(model))
    }

    pub async fn load(
        graphics: Arc<SharedGraphicsContext>,
        path: &PathBuf,
        label: Option<&str>,
    ) -> anyhow::Result<LoadedModel> {
        let file_name = path.file_name();
        log::debug!("Loading model [{:?}]", file_name);

        let path_str = path.to_string_lossy().to_string();

        log::debug!("Checking if model exists in cache");
        if let Some(cached_model) = MODEL_CACHE.lock().get(&path_str) {
            log::debug!("Model loaded from cache: {:?}", path_str);
            return Ok(LoadedModel::new(cached_model.clone()));
        }
        log::debug!("Model does not exist in cache, loading memory...");

        log::debug!("Path of model: {}", path.display());

        let buffer = std::fs::read(path)?;
        let loaded = Self::load_from_memory(graphics, buffer, label).await?;

        let mut model_clone: Model = (*loaded).clone();
        if let Ok(reference) = ResourceReference::from_path(path) {
            model_clone.path = reference;
        }
        if let Some(custom_label) = label {
            model_clone.label = custom_label.to_string();
        }

        let updated = Arc::new(model_clone);
        {
            let mut cache = MODEL_CACHE.lock();
            cache.insert(path_str, Arc::clone(&updated));
            if let Some(custom_label) = label {
                cache.insert(custom_label.to_string(), Arc::clone(&updated));
            }
        }

        log::debug!("Model cached and loaded: {:?}", file_name);
        Ok(LoadedModel::new(updated))
    }
}

pub trait DrawModel<'a> {
    #[allow(unused)]
    fn draw_mesh(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        material: &'a Material,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    #[allow(unused)]
    fn draw_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawModel<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_mesh(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.draw_mesh_instanced(mesh, material, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_mesh_instanced(
        &mut self,
        mesh: &'b Mesh,
        material: &'b Material,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, &material.bind_group, &[]);
        self.set_bind_group(1, camera_bind_group, &[]);
        self.set_bind_group(2, light_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_model(
        &mut self,
        model: &'b Model,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.draw_model_instanced(model, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_model_instanced(
        &mut self,
        model: &'b Model,
        instances: Range<u32>,
        camera_bind_group: &'b wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            let material = &model.materials[mesh.material];
            self.draw_mesh_instanced(
                mesh,
                material,
                instances.clone(),
                camera_bind_group,
                light_bind_group,
            );
        }
    }
}

pub trait DrawLight<'a> {
    #[allow(unused)]
    fn draw_light_mesh(
        &mut self,
        mesh: &'a Mesh,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );

    #[allow(unused)]
    fn draw_light_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
    fn draw_light_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    );
}

impl<'a, 'b> DrawLight<'b> for wgpu::RenderPass<'a>
where
    'b: 'a,
{
    fn draw_light_mesh(
        &mut self,
        mesh: &'a Mesh,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.draw_light_mesh_instanced(mesh, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_light_mesh_instanced(
        &mut self,
        mesh: &'a Mesh,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.set_bind_group(0, camera_bind_group, &[]);
        self.set_bind_group(1, light_bind_group, &[]);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }

    fn draw_light_model(
        &mut self,
        model: &'a Model,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        self.draw_light_model_instanced(model, 0..1, camera_bind_group, light_bind_group);
    }

    fn draw_light_model_instanced(
        &mut self,
        model: &'a Model,
        instances: Range<u32>,
        camera_bind_group: &'a wgpu::BindGroup,
        light_bind_group: &'a wgpu::BindGroup,
    ) {
        for mesh in &model.meshes {
            self.draw_light_mesh_instanced(
                mesh,
                instances.clone(),
                camera_bind_group,
                light_bind_group,
            );
        }
    }
}

pub trait Vertex {
    fn desc() -> VertexBufferLayout<'static>;
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelVertex {
    pub position: [f32; 3],
    pub tex_coords: [f32; 2],
    pub normal: [f32; 3],
}

impl Vertex for ModelVertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        VertexBufferLayout {
            array_stride: mem::size_of::<ModelVertex>() as BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                VertexAttribute {
                    format: wgpu::VertexFormat::Float32x3,
                    offset: 0,
                    shader_location: 0,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}
