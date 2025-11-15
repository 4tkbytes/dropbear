use dropbear_traits::Component;
use glam::{DMat4, DQuat, DVec3, Mat4};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, hash_map::Entry},
    path::Path,
    sync::{Arc, LazyLock},
};

use crate::{
    asset::{ASSET_REGISTRY, AssetHandle, AssetKind, AssetRegistry},
    graphics::{Instance, SharedGraphicsContext, Texture},
    model::{LoadedModel, MODEL_CACHE, Model, ModelId},
    utils::ResourceReference,
};
use anyhow::anyhow;
use dropbear_derive::Component;

/// LocalTransform represents the transform of an entity relative to its parent.
/// If an entity has no parent, this is its position in world space.
#[derive(Debug, Clone, Deserialize, Serialize, Copy, PartialEq, Component)]
pub struct LocalTransform(Transform);

impl LocalTransform {
    /// Creates a new LocalTransform with the specified position, rotation, and scale
    pub fn new(position: impl Into<DVec3>, rotation: impl Into<DQuat>, scale: impl Into<DVec3>) -> Self {
        Self(Transform {
            position: position.into(),
            rotation: rotation.into(),
            scale: scale.into(),
        })
    }

    /// Creates a LocalTransform from an existing Transform
    pub fn from_transform(transform: Transform) -> Self {
        Self(transform)
    }

    /// Returns a reference to the inner Transform
    pub fn inner(&self) -> &Transform {
        &self.0
    }

    /// Returns a mutable reference to the inner Transform
    pub fn inner_mut(&mut self) -> &mut Transform {
        &mut self.0
    }

    /// Consumes self and returns the inner Transform
    pub fn into_inner(self) -> Transform {
        self.0
    }

    /// Gets the position relative to parent
    pub fn position(&self) -> DVec3 {
        self.0.position
    }

    /// Sets the position relative to parent
    pub fn set_position(&mut self, position: DVec3) {
        self.0.position = position;
    }

    /// Gets the rotation relative to parent
    pub fn rotation(&self) -> DQuat {
        self.0.rotation
    }

    /// Sets the rotation relative to parent
    pub fn set_rotation(&mut self, rotation: DQuat) {
        self.0.rotation = rotation;
    }

    /// Gets the scale relative to parent
    pub fn scale(&self) -> DVec3 {
        self.0.scale
    }

    /// Sets the scale relative to parent
    pub fn set_scale(&mut self, scale: DVec3) {
        self.0.scale = scale;
    }

    /// Computes the local transformation matrix
    pub fn matrix(&self) -> DMat4 {
        self.0.matrix()
    }
}

impl Default for LocalTransform {
    fn default() -> Self {
        Self(Transform::default())
    }
}

/// WorldTransform represents the absolute transform of an entity in world space.
/// This is computed by combining the LocalTransform with all parent transforms.
/// 
/// This should be updated by a transform propagation system that walks the hierarchy.
#[derive(Debug, Clone, Deserialize, Serialize, Copy, PartialEq, Component)]
pub struct WorldTransform(Transform);

impl WorldTransform {
    /// Creates a new WorldTransform with the specified position, rotation, and scale
    pub fn new(position: impl Into<DVec3>, rotation: impl Into<DQuat>, scale: impl Into<DVec3>) -> Self {
        Self(Transform {
            position: position.into(),
            rotation: rotation.into(),
            scale: scale.into(),
        })
    }

    /// Creates a WorldTransform from an existing Transform
    pub fn from_transform(transform: Transform) -> Self {
        Self(transform)
    }

    /// Returns a reference to the inner Transform
    pub fn inner(&self) -> &Transform {
        &self.0
    }

    /// Returns a mutable reference to the inner Transform
    pub fn inner_mut(&mut self) -> &mut Transform {
        &mut self.0
    }

    /// Consumes self and returns the inner Transform
    pub fn into_inner(self) -> Transform {
        self.0
    }

    /// Gets the absolute position in world space
    pub fn position(&self) -> DVec3 {
        self.0.position
    }

    /// Sets the absolute position in world space
    pub fn set_position(&mut self, position: DVec3) {
        self.0.position = position;
    }

    /// Gets the absolute rotation in world space
    pub fn rotation(&self) -> DQuat {
        self.0.rotation
    }

    /// Sets the absolute rotation in world space
    pub fn set_rotation(&mut self, rotation: DQuat) {
        self.0.rotation = rotation;
    }

    /// Gets the absolute scale in world space
    pub fn scale(&self) -> DVec3 {
        self.0.scale
    }

    /// Sets the absolute scale in world space
    pub fn set_scale(&mut self, scale: DVec3) {
        self.0.scale = scale;
    }

    /// Computes the world transformation matrix
    pub fn matrix(&self) -> DMat4 {
        self.0.matrix()
    }

    /// Computes the WorldTransform by combining a LocalTransform with a parent's WorldTransform
    pub fn from_local_and_parent(local: &LocalTransform, parent: &WorldTransform) -> Self {
        Self(Transform {
            position: parent.position() + parent.rotation() * (local.position() * parent.scale()),
            rotation: parent.rotation() * local.rotation(),
            scale: parent.scale() * local.scale(),
        })
    }

    /// Updates this WorldTransform from a LocalTransform and parent WorldTransform
    pub fn update_from_parent(&mut self, local: &LocalTransform, parent: &WorldTransform) {
        self.0.position = parent.position() + parent.rotation() * (local.position() * parent.scale());
        self.0.rotation = parent.rotation() * local.rotation();
        self.0.scale = parent.scale() * local.scale();
    }

    /// Synchronizes this WorldTransform from a LocalTransform when there is no parent
    pub fn sync_from_local(&mut self, local: &LocalTransform) {
        self.0 = local.0;
    }
}

impl Default for WorldTransform {
    fn default() -> Self {
        Self(Transform::default())
    }
}

/// A type that represents a position, rotation, and scale of an entity
///
/// This type is the most primitive model, as it implements most traits.
#[repr(C)]
#[derive(Debug, Clone, Deserialize, Serialize, Copy, PartialEq)]
pub struct Transform {
    /// The position of the entity as [`DVec3`]
    pub position: DVec3,
    /// The rotation of the entity as [`DQuat`]
    pub rotation: DQuat,
    /// The scale of the entity as [`DVec3`]
    pub scale: DVec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: DVec3::new(0.0, 0.0, 0.0),
            rotation: DQuat::IDENTITY,
            scale: DVec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Transform {
    /// Creates a new default instance of Transform
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the matrix of the model
    pub(crate) fn matrix(&self) -> DMat4 {
        DMat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    /// Rotates the model on its X axis by a certain angle
    pub fn rotate_x(&mut self, angle_rad: f64) {
        self.rotation *= DQuat::from_euler(glam::EulerRot::XYZ, angle_rad, 0.0, 0.0);
    }

    /// Rotates the model on its Y axis by a certain value
    pub fn rotate_y(&mut self, angle_rad: f64) {
        self.rotation *= DQuat::from_euler(glam::EulerRot::XYZ, 0.0, angle_rad, 0.0);
    }

    /// Rotates the model on its Z axis by a certain value
    pub fn rotate_z(&mut self, angle_rad: f64) {
        self.rotation *= DQuat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, angle_rad);
    }

    /// Translates (moves) the model by a translation [`DVec3`].
    ///
    /// Doesn't replace the position value,
    /// it adds the value.
    pub fn translate(&mut self, translation: DVec3) {
        self.position += translation;
    }

    /// Scales the model by a scale value.
    ///
    /// Doesn't replace the scale value, just multiplies.
    pub fn scale(&mut self, scale: DVec3) {
        self.scale *= scale;
    }
}

#[derive(Clone)]
/// A renderer for meshes and materials related to a model.
///
/// It includes the instances as well as a handle
pub struct MeshRenderer {
    handle: LoadedModel,
    pub instance: Instance,
    pub previous_matrix: DMat4,
    pub is_selected: bool,
    pub material_overrides: Vec<MaterialOverride>,
    original_material_snapshots: HashMap<String, MaterialSnapshot>,
    texture_identifier_cache: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MaterialOverride {
    pub target_material: String,
    pub source_model: ResourceReference,
    pub source_material: String,
}

#[derive(Clone)]
struct MaterialSnapshot {
    texture: Texture,
    texture_tag: Option<String>,
}

impl MeshRenderer {
    pub async fn from_path(
        graphics: Arc<SharedGraphicsContext>,
        path: impl AsRef<Path>,
        label: Option<&str>,
    ) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let handle = Model::load(graphics, &path, label).await?;
        Ok(Self::from_handle(handle))
    }

    /// Creates a new [`MeshRenderer`] instance from a [`LoadedModel`] handle
    pub fn from_handle(handle: LoadedModel) -> Self {
        Self {
            handle,
            instance: Instance::new(DVec3::ZERO, DQuat::IDENTITY, DVec3::ONE),
            previous_matrix: DMat4::IDENTITY,
            is_selected: false,
            material_overrides: Vec::new(),
            original_material_snapshots: HashMap::new(),
            texture_identifier_cache: HashMap::new(),
        }
    }

    pub fn model(&self) -> Arc<Model> {
        self.handle.get()
    }

    pub fn model_id(&self) -> ModelId {
        self.handle.id()
    }

    pub fn asset_handle(&self) -> AssetHandle {
        self.handle.asset_handle()
    }

    pub fn handle(&self) -> &LoadedModel {
        &self.handle
    }

    pub fn handle_mut(&mut self) -> &mut LoadedModel {
        &mut self.handle
    }

    pub fn make_model_mut(&mut self) -> &mut Model {
        self.handle.make_mut()
    }

    pub fn update(&mut self, transform: &Transform) {
        let current_matrix = transform.matrix();
        if self.previous_matrix != current_matrix {
            self.instance = Instance::from_matrix(current_matrix);
            self.previous_matrix = current_matrix;
        }
    }

    /// Swaps the currently loaded model for that renderer by the provided [`LoadedModel`]
    pub fn set_handle(&mut self, handle: LoadedModel) {
        self.set_handle_raw(handle);
    }

    pub fn set_handle_raw(&mut self, handle: LoadedModel) {
        self.handle = handle;
        self.material_overrides.clear();
        self.original_material_snapshots.clear();
        self.texture_identifier_cache.clear();
    }

    /// Swaps the currently loaded model for that renderer by the provided [`AssetHandle`]
    ///
    /// Returns an error if the assethandle provided is not in the model registry.
    pub fn set_asset_handle(&mut self, handle: AssetHandle) -> anyhow::Result<()> {
        if !ASSET_REGISTRY.contains_handle(handle) {
            return Err(anyhow!(
                "Asset handle {} is not registered with the asset registry",
                handle.raw()
            ));
        }

        if !ASSET_REGISTRY.is_handle_kind(handle, AssetKind::Model) {
            return Err(anyhow!(
                "Asset handle {} does not refer to a model asset",
                handle.raw()
            ));
        }

        let model = ASSET_REGISTRY
            .get_model(handle)
            .ok_or_else(|| anyhow!("Model handle {} not found", handle.raw()))?;

        self.set_handle_raw(LoadedModel::from_registered(handle, model));
        Ok(())
    }

    /// Swaps the loaded model to a different one by using an AssetHandle.
    ///
    /// The main difference between [`MeshRenderer::set_asset_handle`] and
    /// [`MeshRenderer::set_asset_handle_raw`] is that it does not use any static variables
    /// (like [`ASSET_REGISTRY`]), instead allowing for the registry to be manually provided.
    pub fn set_asset_handle_raw(
        &mut self,
        registry: &AssetRegistry,
        handle: AssetHandle,
    ) -> anyhow::Result<()> {
        if !registry.contains_handle(handle) {
            return Err(anyhow!(
                "Asset handle {} is not registered with the asset registry",
                handle.raw()
            ));
        }

        if !registry.is_handle_kind(handle, AssetKind::Model) {
            return Err(anyhow!(
                "Asset handle {} does not refer to a model asset",
                handle.raw()
            ));
        }

        let model = registry
            .get_model(handle)
            .ok_or_else(|| anyhow!("Model handle {} not found", handle.raw()))?;

        self.set_handle_raw(LoadedModel::from_registered(handle, model));
        Ok(())
    }

    pub fn uses_model_handle(&self, handle: AssetHandle) -> bool {
        self.asset_handle() == handle
    }

    pub fn uses_model_reference(&self, reference: &ResourceReference) -> bool {
        self.handle().matches_resource(reference)
    }

    pub fn contains_material_handle(&self, handle: AssetHandle) -> bool {
        self.handle().contains_material_handle(handle)
    }

    pub fn contains_material_reference(&self, reference: &ResourceReference) -> bool {
        self.handle().contains_material_reference(reference)
    }

    pub fn material_handle(&self, material_name: &str) -> Option<AssetHandle> {
        self.material_handle_raw(&ASSET_REGISTRY, material_name)
    }

    pub fn material_handle_raw(
        &self,
        registry: &AssetRegistry,
        material_name: &str,
    ) -> Option<AssetHandle> {
        registry.material_handle(self.model_id(), material_name)
    }

    pub fn mesh_handle(&self, mesh_name: &str) -> Option<AssetHandle> {
        self.mesh_handle_raw(&ASSET_REGISTRY, mesh_name)
    }

    pub fn mesh_handle_raw(
        &self,
        registry: &AssetRegistry,
        mesh_name: &str,
    ) -> Option<AssetHandle> {
        registry.mesh_handle(self.model_id(), mesh_name)
    }

    pub fn apply_material_override(
        &mut self,
        target_material: &str,
        source_model: ResourceReference,
        source_material: &str,
    ) -> anyhow::Result<()> {
        self.apply_material_override_raw(
            &ASSET_REGISTRY,
            LazyLock::force(&MODEL_CACHE),
            target_material,
            source_model,
            source_material,
        )
    }

    pub fn apply_material_override_raw(
        &mut self,
        registry: &AssetRegistry,
        model_cache: &Mutex<HashMap<String, Arc<Model>>>,
        target_material: &str,
        source_model: ResourceReference,
        source_material: &str,
    ) -> anyhow::Result<()> {
        let snapshot_entry = {
            let current_model = self.model();
            let original = current_model
                .materials
                .iter()
                .find(|mat| mat.name == target_material)
                .ok_or_else(|| {
                    anyhow!(
                        "Target material '{}' does not exist on model '{}'",
                        target_material,
                        current_model.label
                    )
                })?;

            MaterialSnapshot {
                texture: original.diffuse_texture.clone(),
                texture_tag: original.texture_tag.clone(),
            }
        };

        self.original_material_snapshots
            .entry(target_material.to_string())
            .or_insert(snapshot_entry);

        let source_reference = registry
            .model_handle_from_reference(&source_model)
            .ok_or_else(|| {
                anyhow!(
                    "Source model {:?} is not registered in the asset registry",
                    source_model
                )
            })?;

        let source_model_arc = registry.get_model(source_reference).ok_or_else(|| {
            anyhow!(
                "Unable to fetch model handle {:?} from registry",
                source_reference
            )
        })?;

        let material = source_model_arc
            .materials
            .iter()
            .find(|mat| mat.name == source_material)
            .ok_or_else(|| {
                anyhow!(
                    "Material '{}' does not exist on source model {:?}",
                    source_material,
                    source_model
                )
            })?;

        {
            let model = self.make_model_mut();
            if !model.set_material_texture(
                target_material,
                material.diffuse_texture.clone(),
                material.texture_tag.clone(),
            ) {
                anyhow::bail!(
                    "Target material '{}' does not exist on model '{}'",
                    target_material,
                    model.label
                );
            }
        }

        let original_reference = self.model().path.clone();
        let is_default = original_reference == source_model && target_material == source_material;

        self.material_overrides
            .retain(|entry| entry.target_material != target_material);

        if !is_default {
            self.material_overrides.push(MaterialOverride {
                target_material: target_material.to_string(),
                source_model,
                source_material: source_material.to_string(),
            });
        } else {
            self.original_material_snapshots.remove(target_material);
            self.clear_material_override(target_material);
        }

        // ensure downstream caches observe the newly applied material state
        self.handle.refresh_registry_raw(registry);

        self.refresh_model_cache_with(model_cache);

        Ok(())
    }

    pub fn material_overrides(&self) -> &[MaterialOverride] {
        &self.material_overrides
    }

    pub fn clear_texture_identifier_cache(&mut self) {
        self.texture_identifier_cache.clear();
    }

    pub fn register_texture_identifier(&mut self, identifier: String, material_name: String) {
        match self.texture_identifier_cache.entry(identifier) {
            Entry::Occupied(_) => {}
            Entry::Vacant(slot) => {
                slot.insert(material_name);
            }
        }
    }

    pub fn resolve_texture_identifier(&self, identifier: &str) -> Option<&str> {
        self.texture_identifier_cache
            .get(identifier)
            .map(|value| value.as_str())
    }

    pub fn sync_asset_registry(&mut self) {
        self.handle.refresh_registry_raw(&ASSET_REGISTRY);
        self.refresh_model_cache_with(LazyLock::force(&MODEL_CACHE));
    }

    pub fn clear_material_override(&mut self, target_material: &str) {
        self.material_overrides
            .retain(|entry| entry.target_material != target_material);
    }

    pub fn restore_original_material(&mut self, target_material: &str) -> anyhow::Result<()> {
        self.restore_original_material_raw(
            target_material,
            &ASSET_REGISTRY,
            LazyLock::force(&MODEL_CACHE),
        )
    }

    pub fn restore_original_material_raw(
        &mut self,
        target_material: &str,
        registry: &AssetRegistry,
        model_cache: &Mutex<HashMap<String, Arc<Model>>>,
    ) -> anyhow::Result<()> {
        let snapshot = self
            .original_material_snapshots
            .get(target_material)
            .cloned();

        self.clear_material_override(target_material);

        if let Some(snapshot) = snapshot {
            let model = self.make_model_mut();
            if !model.set_material_texture(
                target_material,
                snapshot.texture.clone(),
                snapshot.texture_tag.clone(),
            ) {
                anyhow::bail!(
                    "Target material '{}' does not exist on model '{}'",
                    target_material,
                    model.label
                );
            }

            if snapshot.texture_tag.is_none() {
                let _ = model.clear_material_texture_tag(target_material);
            }

            self.original_material_snapshots.remove(target_material);
        }

        self.handle.refresh_registry_raw(registry);
        self.refresh_model_cache_with(model_cache);

        Ok(())
    }

    fn refresh_model_cache_with(&self, cache: &Mutex<HashMap<String, Arc<Model>>>) {
        let mut guard = cache.lock();
        self.refresh_model_cache_raw(&mut guard);
    }

    fn refresh_model_cache_raw(&self, cache: &mut HashMap<String, Arc<Model>>) {
        let current = self.handle.get();
        let keys: Vec<String> = cache
            .iter()
            .filter_map(|(key, model)| (model.id == current.id).then(|| key.clone()))
            .collect();

        for key in keys {
            cache.insert(key, Arc::clone(&current));
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelUniform {
    model: [[f32; 4]; 4],
}

impl ModelUniform {
    pub fn new() -> Self {
        Self {
            model: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}
