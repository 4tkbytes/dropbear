use crate::graphics::{SharedGraphicsContext, Texture};
use crate::utils::ResourceReference;
use image::GenericImageView;
use lazy_static::lazy_static;
use parking_lot::Mutex;
// use russimp_ng::{
//     Vector3D,
//     material::{DataContent, TextureType},
//     scene::{PostProcess, Scene},
// };
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use std::{mem, ops::Range, path::PathBuf};
use wgpu::{BufferAddress, VertexAttribute, VertexBufferLayout, util::DeviceExt};
use rayon::prelude::*;

pub const GREY_TEXTURE_BYTES: &'static [u8] = include_bytes!("../../resources/grey.png");

lazy_static! {
    static ref MODEL_CACHE: Mutex<HashMap<String, Model>> = Mutex::new(HashMap::new());
    static ref MEMORY_MODEL_CACHE: Mutex<HashMap<String, Model>> = Mutex::new(HashMap::new());
}

#[derive(Clone)]
pub struct Model {
    pub label: String,
    pub path: ResourceReference,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
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

#[derive(Default, Clone)]
pub struct ParsedModelData {
    pub label: String,
    pub path: ResourceReference,
    pub mesh_data: Vec<ParsedMeshData>,
    pub material_data: Vec<ParsedMaterialData>,
}

#[derive(Default, Clone)]
pub struct ParsedMeshData {
    pub name: String,
    pub vertices: Vec<ModelVertex>,
    pub indices: Vec<u32>,
    pub material_index: usize,
}

#[derive(Default, Clone)]
pub struct ParsedMaterialData {
    pub name: String,
    pub rgba_data: Vec<u8>,
    pub dimensions: (u32, u32),
}

pub trait LazyType {
    type T;
    fn poke(self, graphics: Arc<SharedGraphicsContext>) -> anyhow::Result<Self::T>;
}

/// Loads the model into memory but graphics functions are defined after the creation
/// of the model
#[derive(Default)]
pub struct LazyModel {
    parsed_data: ParsedModelData,
}

impl LazyType for LazyModel {
    type T = Model;

    fn poke(self, graphics: Arc<SharedGraphicsContext>) -> anyhow::Result<Self::T> {
        let start = Instant::now();
        
        let cache_key = self.parsed_data.label.clone();
        if let Some(cached_model) = MEMORY_MODEL_CACHE.lock().get(&cache_key) {
            log::debug!("Model loaded from cache during poke: {:?}", cache_key);
            return Ok(cached_model.clone());
        }

        log::debug!("Creating GPU resources for model: {:?}", self.parsed_data.label);

        let mut materials = Vec::new();
        for material_data in &self.parsed_data.material_data {
            let texture_start = Instant::now();
            
            let diffuse_texture = Texture::from_rgba_buffer(
                graphics.clone(), 
                &material_data.rgba_data, 
                material_data.dimensions
            );
            
            let bind_group = diffuse_texture.bind_group().to_owned();
            
            materials.push(Material {
                name: material_data.name.clone(),
                diffuse_texture,
                bind_group,
            });
            
            log::debug!("Created GPU texture for material '{}' in {:?}", 
                        material_data.name, texture_start.elapsed());
        }

        let mut meshes = Vec::new();
        for mesh_data in &self.parsed_data.mesh_data {
            let buffer_start = Instant::now();

            let vertex_buffer = graphics.clone()
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{} Vertex Buffer", mesh_data.name)),
                    contents: bytemuck::cast_slice(&mesh_data.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                });

            let index_buffer = graphics.clone()
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&format!("{} Index Buffer", mesh_data.name)),
                    contents: bytemuck::cast_slice(&mesh_data.indices),
                    usage: wgpu::BufferUsages::INDEX,
                });

            meshes.push(Mesh {
                name: mesh_data.name.clone(),
                vertex_buffer,
                index_buffer,
                num_elements: mesh_data.indices.len() as u32,
                material: mesh_data.material_index,
            });

            log::debug!("Created GPU buffers for mesh '{}' in {:?}", 
                        mesh_data.name, buffer_start.elapsed());
        }

        let model = Model {
            meshes,
            materials,
            label: self.parsed_data.label.clone(),
            path: self.parsed_data.path.clone(),
        };

        MEMORY_MODEL_CACHE.lock().insert(cache_key, model.clone());
        log::debug!("Model GPU resource creation completed in {:?}", start.elapsed());
        
        Ok(model)
    }
}


impl Model {

    /// Creates a [`LazyModel`]. 
    pub async fn lazy_load(
        buffer: impl AsRef<[u8]>,
        label: Option<&str>,
    ) -> anyhow::Result<LazyModel> {
        let start = Instant::now();
        let label_str = label.unwrap_or("default");
        
        log::debug!("Starting lazy load for model: {:?}", label_str);
        
        let res_ref = ResourceReference::from_bytes(buffer.as_ref());
        let (gltf, buffers, _images) = gltf::import_slice(buffer.as_ref())?;

        let mut texture_data = Vec::new();
        for material in gltf.materials() {
            log::debug!("Processing material: {:?}", material.name());
            let material_name = material.name().unwrap_or("Unnamed Material").to_string();
            
            let image_data = if let Some(pbr) = material.pbr_metallic_roughness().base_color_texture() {
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
        let processed_materials: Vec<_> = texture_data
            .into_par_iter()
            .map(|(material_name, image_data)| {
                let material_start = Instant::now();
                
                let diffuse_image = image::load_from_memory(&image_data)
                    .expect("Failed to load image from memory");
                let diffuse_rgba = diffuse_image.to_rgba8();
                let dimensions = diffuse_image.dimensions();
                
                log::debug!("Processed material '{}' in {:?}", material_name, material_start.elapsed());
                
                ParsedMaterialData {
                    name: material_name,
                    rgba_data: diffuse_rgba.into_raw(),
                    dimensions,
                }
            })
            .collect();

        log::debug!("Parallel material processing took: {:?}", parallel_start.elapsed());

        let mut mesh_data = Vec::new();
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

                let indices: Vec<u32> = reader
                    .read_indices()
                    .ok_or_else(|| anyhow::anyhow!("Mesh missing indices"))?
                    .into_u32()
                    .collect();

                let material_index = primitive.material().index().unwrap_or(0);

                mesh_data.push(ParsedMeshData {
                    name: mesh.name().unwrap_or("Unnamed Mesh").to_string(),
                    vertices,
                    indices,
                    material_index,
                });
            }
        }

        let parsed_data = ParsedModelData {
            label: label_str.to_string(),
            path: res_ref,
            mesh_data,
            material_data: processed_materials,
        };

        log::debug!("Lazy load completed for model: {:?} in {:?}", label_str, start.elapsed());
        
        Ok(LazyModel { parsed_data })
    }

    pub async fn load_from_memory(
        graphics: Arc<SharedGraphicsContext>,
        buffer: impl AsRef<[u8]>,
        label: Option<&str>
    ) -> anyhow::Result<Model> {
        let start = Instant::now();
        let cache_key = label.unwrap_or("default").to_string();

        if let Some(cached_model) = MEMORY_MODEL_CACHE.lock().get(&cache_key) {
            log::debug!("Model loaded from memory cache: {:?}", cache_key);
            return Ok(cached_model.clone());
        }
        
        println!("========== Benchmarking speed of loading {:?} ==========", label);
        log::debug!("Loading from memory");
        let res_ref = ResourceReference::from_bytes(buffer.as_ref());

        let (gltf, buffers, _images) = gltf::import_slice(buffer.as_ref())?;
        let mut meshes = Vec::new();

        let mut texture_data = Vec::new();
        for material in gltf.materials() {
            log::debug!("Processing material: {:?}", material.name());
            let material_name = material.name().unwrap_or("Unnamed Material").to_string();
            
            let image_data = if let Some(pbr) = material.pbr_metallic_roughness().base_color_texture() {
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
                println!("Loading image to memory: {:?}", load_start.elapsed());
                
                let rgba_start = Instant::now();
                let diffuse_rgba = diffuse_image.to_rgba8();
                println!("Converting diffuse image to rgba8 took {:?}", rgba_start.elapsed());
                
                let dimensions = diffuse_image.dimensions();
                
                println!("Parallel processing of material '{}' took: {:?}", material_name, material_start.elapsed());
                
                (material_name, diffuse_rgba.into_raw(), dimensions)
            })
            .collect();

        println!("Total parallel image processing took: {:?}", parallel_start.elapsed());

        let mut materials = Vec::new();
        for (material_name, rgba_data, dimensions) in processed_textures {
            let start = Instant::now();
            
            let diffuse_texture = Texture::from_rgba_buffer(graphics.clone(), &rgba_data, dimensions);
            let bind_group = diffuse_texture.bind_group().to_owned();
            
            materials.push(Material {
                name: material_name,
                diffuse_texture,
                bind_group,
            });
            
            println!("Time to create GPU texture: {:?}", start.elapsed());
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

                let indices: Vec<u32> = reader
                    .read_indices()
                    .ok_or_else(|| anyhow::anyhow!("Mesh missing indices"))?
                    .into_u32()
                    .collect();

                let vertex_buffer = graphics
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{:?} Vertex Buffer", label)),
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
        let model = Model {
            meshes,
            materials,
            label: label.unwrap_or("No named model").to_string(),
            path: res_ref,
        };

        MEMORY_MODEL_CACHE.lock().insert(cache_key, model.clone());
        println!("==================== DONE ====================");
        log::debug!("Model cached from memory: {:?}", label);
        log::debug!("Took {:?} to load model: {:?}", start.elapsed(), label);
        println!("==============================================");
        Ok(model)
    }

    pub async fn load(
        graphics: Arc<SharedGraphicsContext>,
        path: &PathBuf,
        label: Option<&str>,
    ) -> anyhow::Result<Model> {
        let file_name = path.file_name();
        log::debug!("Loading model [{:?}]", file_name);

        let path_str = path.to_string_lossy().to_string();

        if let Some(cached_model) = MODEL_CACHE.lock().get(&path_str) {
            log::debug!("Model loaded from cache: {:?}", path_str);
            return Ok(cached_model.clone());
        }

        let buffer = tokio::fs::read(path).await?;
        let model = Self::load_from_memory(graphics, buffer, label).await?;

        MODEL_CACHE.lock().insert(path_str, model.clone());
        log::debug!("Model cached and loaded: {:?}", file_name);
        Ok(model)
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
