use std::sync::Arc;

use dashmap::DashMap;

use crate::model::{Material, MaterialComponent, Mesh, MeshComponent};

pub type Handle = u64; 

pub struct AssetCache {
    materials: DashMap<MaterialComponent, Arc<Material>>,
    meshes: DashMap<MeshComponent, Arc<Mesh>>,
}

impl AssetCache {
    pub fn new() -> Self {
        Self {
            materials: DashMap::new(),
            meshes: DashMap::new(),
        }
    }

    /// Fetches the material based off the handle. 
    /// 
    /// If it doesn't exist, it will run the loader as a function and store the mesh from there. 
    pub fn get_or_load_material<F>(&self, handle: MaterialComponent, loader: F) -> anyhow::Result<Arc<Material>>
    where
        F: FnOnce() -> anyhow::Result<Material>,
    {
        if let Some(existing) = self.materials.get(&handle) {
            return Ok(existing.clone());
        }

        let material = Arc::new(loader()?);

        match self.materials.entry(handle) {
            dashmap::mapref::entry::Entry::Occupied(entry) => Ok(entry.get().clone()),
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                entry.insert(material.clone());
                Ok(material)
            }
        }
    }

    /// Fetches the mesh based off the handle. 
    /// 
    /// If it doesn't exist, it will run the loader as a function and store the mesh from there. 
    pub fn get_or_load_mesh<F>(&self, handle: MeshComponent, loader: F) -> anyhow::Result<Arc<Mesh>>
    where
        F: FnOnce() -> anyhow::Result<Mesh>,
    {
        log::debug!("Searching for mesh {:?}", handle);
        if let Some(existing) = self.meshes.get(&handle) {
            log::debug!("Found existing mesh");
            return Ok(existing.clone());
        }

        let mesh = Arc::new(loader()?);
        log::debug!("Created new mesh");

        match self.meshes.entry(handle) {
            dashmap::mapref::entry::Entry::Occupied(entry) => Ok(entry.get().clone()),
            dashmap::mapref::entry::Entry::Vacant(entry) => {
                entry.insert(mesh.clone());
                Ok(mesh)
            }
        }
    }

    pub fn clear_everything(&mut self) {
        self.materials.clear();
        self.meshes.clear();
        log::debug!("Cleared everything in the asset cache");
    }
}