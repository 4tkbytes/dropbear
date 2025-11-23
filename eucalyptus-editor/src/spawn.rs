use crate::editor::Editor;
use dropbear_engine::asset::ASSET_REGISTRY;
use dropbear_engine::camera::Camera;
use dropbear_engine::entity::{EntityTransform, MeshRenderer};
use dropbear_engine::future::FutureQueue;
use dropbear_engine::graphics::SharedGraphicsContext;
use dropbear_engine::model::Model;
use dropbear_engine::utils::ResourceReferenceType;
use eucalyptus_core::camera::CameraComponent;
use eucalyptus_core::scene::SceneEntity;
pub(crate) use eucalyptus_core::spawn::{PENDING_SPAWNS, PendingSpawnController};
use eucalyptus_core::states::{ModelProperties, Script, SerializedMeshRenderer};
use eucalyptus_core::utils::ResolveReference;
use eucalyptus_core::{fatal, success};
use hecs::EntityBuilder;
use std::sync::Arc;

fn component_ref<'a, T: 'static>(entity: &'a SceneEntity) -> Option<&'a T> {
    entity
        .components
        .iter()
        .find_map(|component| component.as_any().downcast_ref::<T>())
}

fn component_cloned<T: Clone + 'static>(entity: &SceneEntity) -> Option<T> {
    component_ref::<T>(entity).cloned()
}

impl PendingSpawnController for Editor {
    fn check_up(
        &mut self,
        graphics: Arc<SharedGraphicsContext>,
        queue: Arc<FutureQueue>,
    ) -> anyhow::Result<()> {
        queue.poll();
        let mut spawn_list = PENDING_SPAWNS.lock();
        let mut completed = Vec::new();

        for (index, spawn) in spawn_list.iter_mut().enumerate() {
            log_once::debug_once!(
                "Processing pending spawn for '{}'",
                spawn.scene_entity.label
            );

            let serialized_renderer =
                component_cloned::<SerializedMeshRenderer>(&spawn.scene_entity);

            if serialized_renderer.is_none() && spawn.handle.is_none() {
                log::debug!(
                    "No renderer component found for '{}', spawning immediately",
                    spawn.scene_entity.label
                );
                self.spawn_scene_entity(&spawn.scene_entity, None);
                completed.push(index);
                continue;
            }

            if spawn.handle.is_none() {
                if let Some(renderer) = serialized_renderer.clone() {
                    let graphics_clone = graphics.clone();
                    let label = spawn.scene_entity.label.to_string();
                    let future = async move {
                        load_renderer_from_serialized(renderer, graphics_clone, label).await
                    };
                    let handle = queue.push(Box::pin(future));
                    spawn.handle = Some(handle);
                }
            }

            if let Some(handle) = &spawn.handle {
                if let Some(result) = queue.exchange_owned(handle) {
                    if let Ok(r) = result.downcast::<anyhow::Result<MeshRenderer>>() {
                        match Arc::try_unwrap(r) {
                            Ok(outcome) => match outcome {
                                Ok(renderer) => {
                                    self.spawn_scene_entity(&spawn.scene_entity, Some(renderer));
                                    success!(
                                        "Spawned '{}' from pending queue",
                                        spawn.scene_entity.label
                                    );
                                    completed.push(index);
                                }
                                Err(err) => {
                                    fatal!("Unable to load mesh renderer: {}", err);
                                    completed.push(index);
                                }
                            },
                            Err(_) => {
                                log_once::warn_once!(
                                    "Renderer future for '{}' still shared, deferring",
                                    spawn.scene_entity.label
                                );
                            }
                        }
                    } else {
                        fatal!(
                            "Future result for '{}' could not be downcasted",
                            spawn.scene_entity.label
                        );
                        completed.push(index);
                    }
                }
            }
        }

        for &i in completed.iter().rev() {
            spawn_list.remove(i);
        }

        let mut completed_components = Vec::new();
        for (index, (entity, handle)) in self.pending_components.iter().enumerate() {
            if let Some(result) = queue.exchange_owned(handle) {
                match result.downcast::<anyhow::Result<MeshRenderer>>() {
                    Ok(r) => {
                        match Arc::try_unwrap(r) {
                            Ok(Ok(renderer)) => {
                                let _ = self.world.insert_one(*entity, renderer);
                                let _ = self.world.insert_one(*entity, EntityTransform::default());
                                success!("Added MeshRenderer to entity {:?}", entity);
                                completed_components.push(index);
                            }
                            Ok(Err(e)) => {
                                fatal!("Failed to load MeshRenderer: {}", e);
                                completed_components.push(index);
                            }
                            Err(_) => {} // Still shared
                        }
                    }
                    Err(result) => {
                        if let Ok(r) = result.downcast::<anyhow::Result<(Camera, CameraComponent)>>() {
                            match Arc::try_unwrap(r) {
                                Ok(Ok((camera, component))) => {
                                    let _ = self.world.insert(*entity, (camera, component));
                                    success!("Added Camera to entity {:?}", entity);
                                    completed_components.push(index);
                                }
                                Ok(Err(e)) => {
                                    fatal!("Failed to create Camera: {}", e);
                                    completed_components.push(index);
                                }
                                Err(_) => {} // Still shared
                            }
                        }
                    }
                }
            }
        }

        for &i in completed_components.iter().rev() {
            self.pending_components.remove(i);
        }

        Ok(())
    }
}

impl Editor {
    fn spawn_scene_entity(
        &mut self,
        scene_entity: &SceneEntity,
        mesh_renderer: Option<MeshRenderer>,
    ) {
        let mut builder = EntityBuilder::new();
        builder.add(scene_entity.label.clone());

        if let Some(transform) = component_ref::<EntityTransform>(scene_entity).copied() {
            builder.add(transform);
        } else {
            builder.add(EntityTransform::default());
        }

        if let Some(renderer) = mesh_renderer {
            builder.add(renderer);
        }

        if let Some(props) = component_cloned::<ModelProperties>(scene_entity) {
            builder.add(props);
        }

        if let Some(script) = component_cloned::<Script>(scene_entity) {
            builder.add(script);
        }

        if let Some(camera) = component_cloned::<CameraComponent>(scene_entity) {
            builder.add(camera);
        }

        self.world.spawn(builder.build());
    }
}

async fn load_renderer_from_serialized(
    renderer: SerializedMeshRenderer,
    graphics: Arc<SharedGraphicsContext>,
    label: String,
) -> anyhow::Result<MeshRenderer> {
    let mut mesh_renderer = match &renderer.handle.ref_type {
        ResourceReferenceType::None => anyhow::bail!(
            "Renderer for '{}' does not specify an asset reference",
            label
        ),
        ResourceReferenceType::File(_) => {
            let path = renderer.handle.resolve()?;
            MeshRenderer::from_path(graphics.clone(), &path, Some(&label)).await?
        }
        ResourceReferenceType::Bytes(bytes) => {
            let model =
                Model::load_from_memory(graphics.clone(), bytes.clone(), Some(&label)).await?;
            MeshRenderer::from_handle(model)
        }
        ResourceReferenceType::Plane => {
            anyhow::bail!("Procedural planes are not supported in pending spawns yet");
        }
        ResourceReferenceType::Cube => {
            let model = Model::load_from_memory(
                graphics.clone(),
                include_bytes!("../../resources/models/cube.glb"),
                Some(&label),
            )
            .await?;
            MeshRenderer::from_handle(model)
        }
    };

    for override_entry in renderer.material_override {
        if ASSET_REGISTRY
            .model_handle_from_reference(&override_entry.source_model)
            .is_none()
        {
            if matches!(
                override_entry.source_model.ref_type,
                ResourceReferenceType::File(_)
            ) {
                let source_path = override_entry.source_model.resolve()?;
                let label_hint = override_entry.source_model.as_uri();
                if let Err(err) = Model::load(graphics.clone(), &source_path, label_hint).await {
                    log::warn!(
                        "Failed to preload source model {:?} for override '{}': {}",
                        override_entry.source_model,
                        override_entry.target_material,
                        err
                    );
                    continue;
                }
            } else {
                log::warn!(
                    "Unsupported override source {:?} for '{}'",
                    override_entry.source_model,
                    label
                );
                continue;
            }
        }

        if let Err(err) = mesh_renderer.apply_material_override(
            &override_entry.target_material,
            override_entry.source_model.clone(),
            &override_entry.source_material,
        ) {
            log::warn!(
                "Failed to apply material override '{}' on '{}': {}",
                override_entry.target_material,
                label,
                err
            );
        }
    }

    Ok(mesh_renderer)
}
