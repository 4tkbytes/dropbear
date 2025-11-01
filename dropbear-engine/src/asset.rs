use std::sync::Arc;

use dashmap::DashMap;

use crate::model::{Material, MaterialComponent, Mesh, MeshComponent};

/// A typedef for a Asset handle. 
pub type Handle = u64; 

/// A cache that holds all the assets loaded at that moment in time. 
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
    /// If it doesn't exist, it will run the loader as a function. 
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

    /// Fetches the model based off the handle. 
    /// 
    /// If it doesn't exist, it will run the loader as a function.
    pub fn get_or_load_mesh<F>(&self, handle: MeshComponent, loader: F) -> anyhow::Result<Arc<Mesh>>
    where
        F: FnOnce() -> anyhow::Result<Mesh>,
    {
        if let Some(existing) = self.meshes.get(&handle) {
            return Ok(existing.clone());
        }

        let mesh = Arc::new(loader()?);

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