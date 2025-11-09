use crate::{
    asset::{ASSET_REGISTRY, AssetHandle},
    graphics::{SharedGraphicsContext, Texture},
    utils::ResourceReference,
};
use image::GenericImageView;
use parking_lot::Mutex;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::{Arc, LazyLock};
use std::time::Instant;
use std::{mem, ops::Range, path::PathBuf};
use wgpu::{BufferAddress, VertexAttribute, VertexBufferLayout, util::DeviceExt};
use crate::asset::AssetRegistry;

pub const GREY_TEXTURE_BYTES: &[u8] = include_bytes!("../../resources/textures/grey.png");


pub static MODEL_CACHE: LazyLock<Mutex<HashMap<String, Arc<Model>>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ModelId(pub u64);

impl ModelId {
    pub fn raw(&self) -> u64 {
        self.0
    }
}

#[derive(Clone)]
pub struct Model {
    pub label: String,
    pub path: ResourceReference,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
    pub id: ModelId,
}

#[derive(Clone)]
pub struct LoadedModel {
    inner: Arc<Model>,
    handle: AssetHandle,
}

impl LoadedModel {
    pub fn new(inner: Arc<Model>) -> Self {
        let reference = inner.path.clone();
        let handle = ASSET_REGISTRY.register_model(reference, Arc::clone(&inner));
        Self { inner, handle }
    }

    pub fn from_registered(handle: AssetHandle, inner: Arc<Model>) -> Self {
        Self { inner, handle }
    }

    pub fn from_asset_handle_raw(registry: &AssetRegistry, handle: AssetHandle) -> Option<Self> {
        registry
            .get_model(handle)
            .map(|model| Self::from_registered(handle, model))
    }

    pub fn from_asset_handle(handle: AssetHandle) -> Option<Arc<LoadedModel>> {
        ASSET_REGISTRY
            .get_model(handle)
            .map(|model| Arc::new(Self::from_registered(handle, model)))
    }

    /// Returns the unique identifier of the underlying model asset.
    pub fn id(&self) -> ModelId {
        self.inner.id
    }

    /// Returns the asset handle associated with the underlying model.
    pub fn asset_handle(&self) -> AssetHandle {
        self.handle
    }

    pub fn matches_resource(&self, reference: &ResourceReference) -> bool {
        self.inner.matches_resource(reference)
    }

    /// Provides shared access to the underlying model.
    pub fn get(&self) -> Arc<Model> {
        Arc::clone(&self.inner)
    }

    /// Provides mutable access to the underlying model data, cloning if shared.
    pub fn make_mut(&mut self) -> &mut Model {
        Arc::make_mut(&mut self.inner)
    }

    /// Re-registers the model with the global asset registry, ensuring cached
    /// sub-assets stay in sync after mutations.
    pub fn refresh_registry(&mut self) {
        let reference = self.inner.path.clone();
        let updated_handle = ASSET_REGISTRY.register_model(reference, self.get());
        self.handle = updated_handle;
    }

    pub fn refresh_registry_raw(&mut self, registry: &AssetRegistry) {
        let reference = self.inner.path.clone();
        let updated_handle = registry.register_model(reference, self.get());
        self.handle = updated_handle;
    }

    pub fn contains_material_handle(&self, handle: AssetHandle) -> bool {
        self.inner.contains_material_handle(handle)
    }

    pub fn contains_material_reference(&self, reference: &ResourceReference) -> bool {
        self.inner.contains_material_reference(reference)
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
    pub texture_tag: Option<String>,
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
    /// Replaces the diffuse texture for the material identified by `material_name`.
    /// When `texture_tag` is provided it will be stored so the caller can later
    /// confirm which texture is applied.
    pub fn set_material_texture(
        &mut self,
        material_name: &str,
        texture: Texture,
        texture_tag: Option<String>,
    ) -> bool {
        if let Some(material) = self
            .materials
            .iter_mut()
            .find(|mat| mat.name == material_name)
        {
            let bind_group = texture.bind_group().to_owned();
            material.diffuse_texture = texture;
            material.bind_group = bind_group;
            if let Some(tag) = texture_tag {
                material.texture_tag = Some(tag);
            }
            true
        } else {
            false
        }
    }

    /// Removes any stored texture tag for the supplied material.
    pub fn clear_material_texture_tag(&mut self, material_name: &str) -> bool {
        if let Some(material) = self
            .materials
            .iter_mut()
            .find(|mat| mat.name == material_name)
        {
            material.texture_tag = None;
            true
        } else {
            false
        }
    }

    /// Returns `true` if a material with `material_name` exists within this model.
    pub fn contains_material(&self, material_name: &str) -> bool {
        self.materials.iter().any(|mat| mat.name == material_name)
    }

    /// Returns the registered asset handle for this model, if available.
    pub fn asset_handle(&self) -> Option<AssetHandle> {
        ASSET_REGISTRY.model_handle_from_reference(&self.path)
    }

    /// Returns `true` if this model was loaded from the specified resource reference.
    pub fn matches_resource(&self, reference: &ResourceReference) -> bool {
        &self.path == reference
    }

    /// Returns `true` if this model owns the supplied material handle.
    pub fn contains_material_handle(&self, material_handle: AssetHandle) -> bool {
        ASSET_REGISTRY
            .material_owner(material_handle) == Some(self.id)
    }

    /// Returns `true` if this model owns a material registered under the provided resource reference.
    pub fn contains_material_reference(&self, reference: &ResourceReference) -> bool {
        ASSET_REGISTRY
            .material_handle_from_reference(reference)
            .map_or(false, |handle| self.contains_material_handle(handle))
    }

    /// Returns `true` if any material on this model is tagged with `texture_tag`.
    pub fn contains_texture_tag(&self, texture_tag: &str) -> bool {
        self.materials
            .iter()
            .any(|mat| mat.texture_tag.as_deref() == Some(texture_tag))
    }

    /// Returns `true` if the specified material currently carries `texture_tag`.
    pub fn material_has_texture_tag(&self, material_name: &str, texture_tag: &str) -> bool {
        self.materials
            .iter()
            .find(|mat| mat.name == material_name)
            .and_then(|mat| mat.texture_tag.as_deref()) == Some(texture_tag)
    }

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
            let texture_tag = Some(material_name.clone());

            materials.push(Material {
                name: material_name,
                diffuse_texture,
                bind_group,
                texture_tag,
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

        let loaded = LoadedModel::new(Arc::clone(&model));

        MODEL_CACHE.lock().insert(cache_key.clone(), model);
        log::trace!("==================== DONE ====================");
        log::debug!("Model cached from memory: {:?}", label);
        log::debug!("Took {:?} to load model: {:?}", start.elapsed(), label);
        log::trace!("==============================================");
        Ok(loaded)
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
