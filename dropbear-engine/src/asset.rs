use std::sync::{Arc, atomic::{AtomicU64, Ordering}, LazyLock};

use dashmap::DashMap;

use crate::{
    model::{Material, Mesh, Model, ModelId},
    utils::ResourceReference,
};

/// Opaque identifier returned from the [`AssetRegistry`] for stored assets.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AssetHandle(u64);
impl AssetHandle {
    /// Creates a new [`AssetHandle`].
    ///
    /// This function does not guarantee if the raw value exists in the registry.
    /// You will have to check yourself.
    pub fn new(raw: u64) -> Self { Self(raw) }
    /// Returns the raw/primitive [`u64`] value.
    pub fn raw(&self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AssetKind {
    Model,
    Material,
    Mesh,
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub enum PointerKind {
    Const(&'static str),
    Mut(&'static str),
}

/// Centralised cache for models and their dependent resources.
///
/// The registry assigns stable [`AssetHandle`] values that can be
/// reused by systems without having to keep strong references to the
/// underlying assets. Models are keyed by their [`ResourceReference`]
/// while meshes and materials are keyed by `(ModelId, name)` pairs.
pub struct AssetRegistry {
    next_id: AtomicU64,

    model_handles: DashMap<ResourceReference, AssetHandle>,
    model_id_lookup: DashMap<ModelId, AssetHandle>,
    model_references: DashMap<AssetHandle, ResourceReference>,
    model_reference_lookup: DashMap<ResourceReference, AssetHandle>,
    models: DashMap<AssetHandle, Arc<Model>>,

    material_lookup: DashMap<(ModelId, String), AssetHandle>,
    material_owners: DashMap<AssetHandle, ModelId>,
    material_references: DashMap<AssetHandle, ResourceReference>,
    material_reference_lookup: DashMap<ResourceReference, AssetHandle>,
    materials: DashMap<AssetHandle, Arc<Material>>,

    mesh_lookup: DashMap<(ModelId, String), AssetHandle>,
    mesh_owners: DashMap<AssetHandle, ModelId>,
    mesh_references: DashMap<AssetHandle, ResourceReference>,
    mesh_reference_lookup: DashMap<ResourceReference, AssetHandle>,
    meshes: DashMap<AssetHandle, Arc<Mesh>>,

    pointers: DashMap<PointerKind, usize>,
}

impl AssetRegistry {
    pub fn new() -> Self {
        Self {
            next_id: AtomicU64::new(1),
            model_handles: DashMap::new(),
            model_id_lookup: DashMap::new(),
            model_references: DashMap::new(),
            model_reference_lookup: DashMap::new(),
            models: DashMap::new(),
            material_lookup: DashMap::new(),
            material_owners: DashMap::new(),
            material_references: DashMap::new(),
            material_reference_lookup: DashMap::new(),
            materials: DashMap::new(),
            mesh_lookup: DashMap::new(),
            mesh_owners: DashMap::new(),
            mesh_references: DashMap::new(),
            mesh_reference_lookup: DashMap::new(),
            meshes: DashMap::new(),
            pointers: DashMap::new(),
        }
    }

    pub fn add_pointer(&self, pointer_kind: PointerKind, pointer: usize) {
        self.pointers.insert(pointer_kind, pointer);
    }

    pub fn get_pointer(&self, pointer_kind: PointerKind) -> Option<usize> {
        self.pointers.get(&pointer_kind).map(|entry| *entry.value())
    }

    fn allocate_handle(&self) -> AssetHandle {
        AssetHandle(self.next_id.fetch_add(1, Ordering::Relaxed))
    }

    /// Registers a model and caches its meshes and materials.
    pub fn register_model(&self, reference: ResourceReference, model: Arc<Model>) -> AssetHandle {
        let canonical = reference.clone();
        let model_handle = if let Some(existing) = self.model_handles.get(&canonical) {
            let handle = *existing;
            self.models.insert(handle, Arc::clone(&model));
            handle
        } else {
            let handle = self.allocate_handle();
            self.models.insert(handle, Arc::clone(&model));
            self.model_handles.insert(canonical.clone(), handle);
            handle
        };

        self.model_id_lookup.insert(model.id, model_handle);

        self.model_references
            .insert(model_handle, canonical.clone());
        self.model_reference_lookup.insert(canonical, model_handle);

        self.cache_model_components(&model);

        model_handle
    }

    /// Iterates through all models, allowing you to iterate through all items in the
    /// model registry.
    pub fn iter_model(&self) -> dashmap::iter::Iter<'_, AssetHandle, Arc<Model>> {
        self.models.iter()
    }

    pub fn iter_material(&self) -> dashmap::iter::Iter<'_, AssetHandle, Arc<Material>> {
        self.materials.iter()
    }

    /// Returns the cached model handle if it exists.
    pub fn model_handle(&self, reference: &ResourceReference) -> Option<AssetHandle> {
        self.model_handles.get(reference).map(|entry| *entry)
    }

    /// Fetches a model by handle.
    pub fn get_model(&self, handle: AssetHandle) -> Option<Arc<Model>> {
        self.models
            .get(&handle)
            .map(|entry| Arc::clone(entry.value()))
    }

    /// Fetches a material by handle.
    pub fn get_material(&self, handle: AssetHandle) -> Option<Arc<Material>> {
        self.materials
            .get(&handle)
            .map(|entry| Arc::clone(entry.value()))
    }

    /// Fetches a mesh by handle.
    pub fn get_mesh(&self, handle: AssetHandle) -> Option<Arc<Mesh>> {
        self.meshes
            .get(&handle)
            .map(|entry| Arc::clone(entry.value()))
    }

    /// Fetches a handle from a [`ResourceReference`] by checking through
    /// each cache
    pub fn get_handle_from_reference(&self, reference: &ResourceReference) -> Option<AssetHandle> {
        self.material_handle_from_reference(reference)
            .or_else(|| self.mesh_handle_from_reference(reference))
            .or_else(|| self.model_handle_from_reference(reference))
    }

    /// Retrieves (or lazily creates) the handle for a specific material on a model.
    pub fn material_handle(&self, model_id: ModelId, name: &str) -> Option<AssetHandle> {
        let key = (model_id, name.to_string());
        self.material_lookup.get(&key).map(|entry| *entry)
    }

    /// Retrieves (or lazily creates) the handle for a specific mesh on a model.
    pub fn mesh_handle(&self, model_id: ModelId, name: &str) -> Option<AssetHandle> {
        let key = (model_id, name.to_string());
        self.mesh_lookup.get(&key).map(|entry| *entry)
    }

    /// Returns the kind of asset associated with a handle, if known.
    pub fn kind(&self, handle: AssetHandle) -> Option<AssetKind> {
        if self.models.contains_key(&handle) {
            Some(AssetKind::Model)
        } else if self.materials.contains_key(&handle) {
            Some(AssetKind::Material)
        } else if self.meshes.contains_key(&handle) {
            Some(AssetKind::Mesh)
        } else {
            None
        }
    }

    /// Returns `true` if the handle exists in any asset cache.
    pub fn contains_handle(&self, handle: AssetHandle) -> bool {
        self.models.contains_key(&handle)
            || self.materials.contains_key(&handle)
            || self.meshes.contains_key(&handle)
    }

    /// Returns `true` if the handle represents the expected asset kind.
    pub fn is_handle_kind(&self, handle: AssetHandle, expected: AssetKind) -> bool {
        matches!(self.kind(handle), Some(kind) if kind == expected)
    }

    /// Returns the `ResourceReference` recorded for a model handle, if any.
    pub fn model_reference_for_handle(&self, handle: AssetHandle) -> Option<ResourceReference> {
        self.model_references
            .get(&handle)
            .map(|entry| entry.value().clone())
    }

    /// Attempts to resolve a model handle from a `ResourceReference`.
    pub fn model_handle_from_reference(
        &self,
        reference: &ResourceReference,
    ) -> Option<AssetHandle> {
        self.model_reference_lookup
            .get(reference)
            .map(|entry| *entry)
    }

    /// Attempts to resolve a model handle directly from a [`ModelId`].
    pub fn model_handle_from_id(&self, model_id: ModelId) -> Option<AssetHandle> {
        self.model_id_lookup.get(&model_id).map(|entry| *entry)
    }

    /// Returns `true` if the handle refers to a material asset.
    pub fn is_material(&self, handle: AssetHandle) -> bool {
        self.materials.contains_key(&handle)
    }

    /// Returns `true` if the handle refers to a mesh asset.
    pub fn is_mesh(&self, handle: AssetHandle) -> bool {
        self.meshes.contains_key(&handle)
    }

    /// Returns `true` if the handle refers to a model asset.
    pub fn is_model(&self, handle: AssetHandle) -> bool {
        self.models.contains_key(&handle)
    }

    /// Returns the owning model ID for the given material handle.
    pub fn material_owner(&self, handle: AssetHandle) -> Option<ModelId> {
        self.material_owners.get(&handle).map(|entry| *entry)
    }

    /// Returns the owning model ID for the given mesh handle.
    pub fn mesh_owner(&self, handle: AssetHandle) -> Option<ModelId> {
        self.mesh_owners.get(&handle).map(|entry| *entry)
    }

    /// Returns the synthetic `ResourceReference` associated with a material handle.
    pub fn material_reference_for_handle(&self, handle: AssetHandle) -> Option<ResourceReference> {
        self.material_references
            .get(&handle)
            .map(|entry| entry.value().clone())
    }

    /// Returns the synthetic `ResourceReference` associated with a mesh handle.
    pub fn mesh_reference_for_handle(&self, handle: AssetHandle) -> Option<ResourceReference> {
        self.mesh_references
            .get(&handle)
            .map(|entry| entry.value().clone())
    }

    /// Attempts to resolve a material handle from its synthetic `ResourceReference`.
    pub fn material_handle_from_reference(
        &self,
        reference: &ResourceReference,
    ) -> Option<AssetHandle> {
        self.material_reference_lookup
            .get(reference)
            .map(|entry| *entry)
    }

    /// Attempts to resolve a mesh handle from its synthetic `ResourceReference`.
    pub fn mesh_handle_from_reference(&self, reference: &ResourceReference) -> Option<AssetHandle> {
        self.mesh_reference_lookup
            .get(reference)
            .map(|entry| *entry)
    }

    fn cache_model_components(&self, model: &Arc<Model>) {
        let model_id = model.id;

        for material in &model.materials {
            let name = material.name.clone();
            let key = (model_id, name.clone());
            let handle = if let Some(existing) = self.material_lookup.get(&key) {
                *existing
            } else {
                let handle = self.allocate_handle();
                self.material_lookup.insert(key.clone(), handle);
                handle
            };

            self.material_owners.insert(handle, model_id);

            if let Some(reference) = material_reference(model_id, &name) {
                self.material_references.insert(handle, reference.clone());
                self.material_reference_lookup.insert(reference, handle);
            }

            self.materials.insert(handle, Arc::new(material.clone()));
        }

        for mesh in &model.meshes {
            let name = mesh.name.clone();
            let key = (model_id, name.clone());
            let handle = if let Some(existing) = self.mesh_lookup.get(&key) {
                *existing
            } else {
                let handle = self.allocate_handle();
                self.mesh_lookup.insert(key.clone(), handle);
                handle
            };

            self.mesh_owners.insert(handle, model_id);

            if let Some(reference) = mesh_reference(model_id, &name) {
                self.mesh_references.insert(handle, reference.clone());
                self.mesh_reference_lookup.insert(reference, handle);
            }

            self.meshes.insert(handle, Arc::new(mesh.clone()));
        }
    }
}

impl Default for AssetRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub static ASSET_REGISTRY: LazyLock<AssetRegistry> = LazyLock::new(AssetRegistry::new);

fn material_reference(model_id: ModelId, name: &str) -> Option<ResourceReference> {
    resource_reference_for("materials", model_id, name)
}

fn mesh_reference(model_id: ModelId, name: &str) -> Option<ResourceReference> {
    resource_reference_for("meshes", model_id, name)
}

fn resource_reference_for(
    category: &str,
    model_id: ModelId,
    name: &str,
) -> Option<ResourceReference> {
    let sanitized = sanitize_component(name);
    if sanitized.is_empty() {
        return None;
    }
    let uri = format!("euca://{}/{}/{}", category, model_id.raw(), sanitized);
    ResourceReference::from_euca_uri(uri).ok()
}

fn sanitize_component(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    trimmed
        .chars()
        .map(|ch| {
            let lower = ch.to_ascii_lowercase();
            match lower {
                'a'..='z' | '0'..='9' | '-' | '_' => lower,
                _ => '_',
            }
        })
        .collect()
}
