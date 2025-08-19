use std::{mem, ops::Range, path::PathBuf};

use russimp_ng::{
    Vector3D,
    material::{DataContent, TextureType},
    scene::{PostProcess, Scene},
};
use wgpu::{BufferAddress, VertexAttribute, VertexBufferLayout, util::DeviceExt};

use crate::graphics::{Graphics, NO_MODEL, NO_TEXTURE, Texture};

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

pub struct Model {
    pub label: String,
    pub path: PathBuf,
    pub meshes: Vec<Mesh>,
    pub materials: Vec<Material>,
}

pub struct Material {
    pub name: String,
    pub diffuse_texture: Texture,
    pub bind_group: wgpu::BindGroup,
}

pub struct Mesh {
    pub name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
    pub material: usize,
}

impl Model {
    pub fn load_from_memory(
        graphics: &Graphics<'_>,
        buffer: &[u8],
        label: Option<&str>,
    ) -> anyhow::Result<Model> {
        log::debug!("Loading from memory");

        let scene = match Scene::from_buffer(
            buffer,
            vec![
                PostProcess::Triangulate,
                PostProcess::FlipUVs,
                PostProcess::GenerateNormals,
            ],
            "obj",
        ) {
            Ok(v) => v,
            Err(_) => Scene::from_buffer(
                NO_MODEL,
                vec![
                    PostProcess::Triangulate,
                    PostProcess::FlipUVs,
                    PostProcess::GenerateNormals,
                ],
                "glb",
            )?,
        };

        let mut materials = Vec::new();
        for m in &scene.materials {
            let mut name = String::new();
            let diffuse_bytes_opt = m
                .textures
                .iter()
                .find(|(t_type, _)| **t_type == TextureType::Diffuse)
                .and_then(|(_, tex)| {
                    name = tex.borrow().filename.clone();
                    match &tex.borrow().data {
                        DataContent::Bytes(b) => Some(b.clone()),
                        DataContent::Texel(_) => {
                            log::warn!("Skipping texel-based texture for material '{}'", &name);
                            None
                        }
                    }
                });

            let diffuse_texture = if let Some(bytes) = diffuse_bytes_opt {
                Texture::new(graphics, &bytes)
            } else {
                if !name.is_empty() {
                    log::warn!(
                        "Error loading material {}, using default missing texture",
                        name
                    );
                } else {
                    log::warn!("Error loading material, using default missing texture");
                }
                Texture::new(graphics, NO_TEXTURE)
            };

            let bind_group = diffuse_texture.bind_group().to_owned();
            materials.push(Material {
                name: name,
                diffuse_texture,
                bind_group,
            });
        }

        let mut meshes = Vec::new();
        for mesh in &scene.meshes {
            let vertices: Vec<ModelVertex> = mesh
                .vertices
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let normal = mesh.normals.get(i).copied().unwrap_or(Vector3D {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    });
                    let tex_coords = mesh
                        .texture_coords
                        .get(0)
                        .and_then(|coords| coords.as_ref().and_then(|vec| vec.get(i)))
                        .map(|tc| [tc.x, tc.y])
                        .unwrap_or([0.0, 0.0]);
                    ModelVertex {
                        position: [v.x, v.y, v.z],
                        tex_coords,
                        normal: [normal.x, normal.y, normal.z],
                    }
                })
                .collect();

            let indices: Vec<u32> = mesh
                .faces
                .iter()
                .flat_map(|f| f.0.iter().copied())
                .collect();

            let vertex_buffer =
                graphics
                    .state
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{:?} Vertex Buffer", label)),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
            let index_buffer =
                graphics
                    .state
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{:?} Index Buffer", label)),
                        contents: bytemuck::cast_slice(&indices),
                        usage: wgpu::BufferUsages::INDEX,
                    });

            meshes.push(Mesh {
                name: mesh.name.clone(),
                vertex_buffer,
                index_buffer,
                num_elements: indices.len() as u32,
                material: mesh.material_index as usize,
            });
        }
        log::debug!("Successfully loaded model [{:?}]", label);
        Ok(Model {
            meshes,
            materials,
            label: if let Some(l) = label {
                l.to_string()
            } else {
                String::from("Model")
            },
            path: PathBuf::new(),
        })
    }

    pub fn load(
        graphics: &Graphics<'_>,
        path: &PathBuf,
        label: Option<&str>,
    ) -> anyhow::Result<Model> {
        let file_name = path.file_name().unwrap().to_str().unwrap();
        log::debug!("Loading model [{}]", file_name);

        let scene = match Scene::from_file(
            path.to_str().unwrap(),
            vec![
                PostProcess::Triangulate,
                PostProcess::FlipUVs,
                PostProcess::GenerateNormals,
            ],
        ) {
            Ok(v) => v,
            Err(_) => Scene::from_buffer(
                NO_MODEL,
                vec![
                    PostProcess::Triangulate,
                    PostProcess::FlipUVs,
                    PostProcess::GenerateNormals,
                ],
                "glb",
            )?,
        };

        let mut materials = Vec::new();
        for m in &scene.materials {
            let mut name = String::new();
            let diffuse_bytes_opt = m
                .textures
                .iter()
                .find(|(t_type, _)| **t_type == TextureType::Diffuse)
                .and_then(|(_, tex)| {
                    name = tex.borrow().filename.clone();
                    match &tex.borrow().data {
                        DataContent::Bytes(b) => Some(b.clone()),
                        DataContent::Texel(_) => {
                            log::warn!("Skipping texel-based texture for material '{}'", &name);
                            None
                        }
                    }
                });

            let diffuse_texture = if let Some(bytes) = diffuse_bytes_opt {
                Texture::new(graphics, &bytes)
            } else {
                if !name.is_empty() {
                    log::warn!(
                        "Error loading material {}, using default missing texture",
                        name
                    );
                } else {
                    log::warn!("Error loading material, using default missing texture");
                }
                Texture::new(graphics, NO_TEXTURE)
            };

            let bind_group = diffuse_texture.bind_group().to_owned();
            materials.push(Material {
                name: name,
                diffuse_texture,
                bind_group,
            });
        }

        let mut meshes = Vec::new();
        for mesh in &scene.meshes {
            let vertices: Vec<ModelVertex> = mesh
                .vertices
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    let normal = mesh.normals.get(i).copied().unwrap_or(Vector3D {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                    });
                    let tex_coords = mesh
                        .texture_coords
                        .get(0)
                        .and_then(|coords| coords.as_ref().and_then(|vec| vec.get(i)))
                        .map(|tc| [tc.x, tc.y])
                        .unwrap_or([0.0, 0.0]);
                    ModelVertex {
                        position: [v.x, v.y, v.z],
                        tex_coords,
                        normal: [normal.x, normal.y, normal.z],
                    }
                })
                .collect();

            let indices: Vec<u32> = mesh
                .faces
                .iter()
                .flat_map(|f| f.0.iter().copied())
                .collect();

            let vertex_buffer =
                graphics
                    .state
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{:?} Vertex Buffer", file_name)),
                        contents: bytemuck::cast_slice(&vertices),
                        usage: wgpu::BufferUsages::VERTEX,
                    });
            let index_buffer =
                graphics
                    .state
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("{:?} Index Buffer", file_name)),
                        contents: bytemuck::cast_slice(&indices),
                        usage: wgpu::BufferUsages::INDEX,
                    });

            meshes.push(Mesh {
                name: mesh.name.clone(),
                vertex_buffer,
                index_buffer,
                num_elements: indices.len() as u32,
                material: mesh.material_index as usize,
            });
        }
        log::debug!("Successfully loaded model [{}]", file_name);
        Ok(Model {
            meshes,
            materials,
            label: if let Some(l) = label {
                l.to_string()
            } else {
                String::from(file_name.split(".").into_iter().next().unwrap())
            },
            path: path.clone(),
        })
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
    fn draw_model(&mut self, model: &'a Model, camera_bind_group: &'a wgpu::BindGroup, light_bind_group: &'a wgpu::BindGroup);
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

    fn draw_model(&mut self, model: &'b Model, camera_bind_group: &'b wgpu::BindGroup, light_bind_group: &'a wgpu::BindGroup) {
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
            self.draw_mesh_instanced(mesh, material, instances.clone(), camera_bind_group, light_bind_group);
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
    fn draw_light_model(&mut self, model: &'a Model, camera_bind_group: &'a wgpu::BindGroup, light_bind_group: &'a wgpu::BindGroup);
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

    fn draw_light_model(&mut self, model: &'a Model, camera_bind_group: &'a wgpu::BindGroup, light_bind_group: &'a wgpu::BindGroup) {
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
            self.draw_light_mesh_instanced(mesh, instances.clone(), camera_bind_group, light_bind_group);
        }
    }
}
