use crate::editor::Editor;
use dropbear_engine::camera::Camera;
use dropbear_engine::entity::{LocalTransform, MeshRenderer, WorldTransform};
use dropbear_engine::future::FutureQueue;
use dropbear_engine::graphics::SharedGraphicsContext;
use dropbear_engine::lighting::{Light, LightComponent};
use dropbear_engine::model::Model;
use dropbear_engine::utils::ResourceReferenceType;
use eucalyptus_core::camera::CameraComponent;
pub(crate) use eucalyptus_core::spawn::{PENDING_SPAWNS, PendingSpawnController};
use eucalyptus_core::states::{
    CameraConfig, Label, ModelProperties, SceneMeshRendererComponent, ScriptComponent,
};
use eucalyptus_core::utils::ResolveReference;
use eucalyptus_core::{fatal, success};
use std::sync::Arc;

impl Editor {
    /// Helper function to load a mesh renderer from a SceneMeshRendererComponent
    async fn load_mesh_renderer_from_component(
        comp: &SceneMeshRendererComponent,
        graphics: &Arc<SharedGraphicsContext>,
        label: &str,
    ) -> anyhow::Result<MeshRenderer> {
        let renderer = match &comp.model.ref_type {
            ResourceReferenceType::File(_reference) => {
                let path = comp.model.resolve()?;
                log::debug!(
                    "Loading model for entity '{}' from path {}",
                    label,
                    path.display()
                );
                MeshRenderer::from_path(graphics.clone(), &path, Some(label)).await?
            }
            ResourceReferenceType::Bytes(bytes) => {
                log::info!(
                    "Loading entity '{}' from bytes [Len: {}]",
                    label,
                    bytes.len()
                );
                let model =
                    Model::load_from_memory(graphics.clone(), bytes.clone(), Some(label)).await?;
                MeshRenderer::from_handle(model)
            }
            ResourceReferenceType::Cube => {
                log::info!("Loading entity '{}' as cube", label);
                let model = Model::load_from_memory(
                    graphics.clone(),
                    include_bytes!("../../resources/models/cube.glb").to_vec(),
                    Some(label),
                )
                .await?;
                MeshRenderer::from_handle(model)
            }
            ResourceReferenceType::None => {
                anyhow::bail!("No model reference provided for entity '{}'", label);
            }
            ResourceReferenceType::Plane => {
                anyhow::bail!(
                    "Plane resource type not yet supported in spawn controller for entity '{}'",
                    label
                );
            }
        };

        Ok(renderer)
    }
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

        for (i, spawn) in spawn_list.iter_mut().enumerate() {
            log_once::debug_once!("Caught pending spawn! Entity: {}", spawn.entity.label);

            if spawn.handle.is_none() {
                log_once::debug_once!("Pending spawn does NOT have a handle, creating new one now");

                let graphics_clone = graphics.clone();
                let entity = spawn.entity.clone();

                let func = async move {
                    let mut mesh_renderer: Option<MeshRenderer> = None;
                    let mut world_transform: Option<WorldTransform> = None;
                    let mut local_transform: Option<LocalTransform> = None;
                    let mut camera_config: Option<CameraConfig> = None;
                    let mut light_config: Option<(
                        LightComponent,
                        dropbear_engine::entity::Transform,
                    )> = None;
                    let mut script_comp: Option<ScriptComponent> = None;
                    let mut model_props: Option<ModelProperties> = None;

                    for comp in &entity.components {
                        if let Some(mesh_comp) =
                            comp.as_any().downcast_ref::<SceneMeshRendererComponent>()
                        {
                            match Self::load_mesh_renderer_from_component(
                                mesh_comp,
                                &graphics_clone,
                                &entity.label.to_string(),
                            )
                            .await
                            {
                                Ok(renderer) => mesh_renderer = Some(renderer),
                                Err(e) => {
                                    log::error!(
                                        "Failed to load mesh for entity '{}': {}",
                                        entity.label,
                                        e
                                    );
                                }
                            }
                        } else if let Some(wt) = comp.as_any().downcast_ref::<WorldTransform>() {
                            world_transform = Some(*wt);
                        } else if let Some(lt) = comp.as_any().downcast_ref::<LocalTransform>() {
                            local_transform = Some(*lt);
                        } else if let Some(cam_cfg) = comp.as_any().downcast_ref::<CameraConfig>() {
                            camera_config = Some(cam_cfg.clone());
                        } else if let Some(light_comp) =
                            comp.as_any().downcast_ref::<LightComponent>()
                        {
                            let transform = local_transform
                                .map(|lt| lt.into_inner())
                                .unwrap_or_default();
                            light_config = Some((light_comp.clone(), transform));
                        } else if let Some(script) = comp.as_any().downcast_ref::<ScriptComponent>()
                        {
                            script_comp = Some(script.clone());
                        } else if let Some(props) = comp.as_any().downcast_ref::<ModelProperties>()
                        {
                            model_props = Some(props.clone());
                        }
                    }

                    let lt = local_transform.unwrap_or_default();
                    let wt = world_transform
                        .unwrap_or_else(|| WorldTransform::from_transform(lt.into_inner()));

                    Ok::<_, anyhow::Error>((
                        entity.label,
                        mesh_renderer,
                        lt,
                        wt,
                        camera_config,
                        light_config,
                        script_comp,
                        model_props,
                    ))
                };

                let handle = queue.push(Box::pin(func));
                spawn.handle = Some(handle);
            } else {
                log_once::debug_once!("Spawn does have handle, using that one");
            }

            if let Some(handle) = &spawn.handle {
                log_once::debug_once!("Handle located");
                if let Some(result) = queue.exchange_owned(handle) {
                    log_once::debug_once!("Loading done, located result");

                    type EntityData = anyhow::Result<(
                        Label,
                        Option<MeshRenderer>,
                        LocalTransform,
                        WorldTransform,
                        Option<CameraConfig>,
                        Option<(LightComponent, dropbear_engine::entity::Transform)>,
                        Option<ScriptComponent>,
                        Option<ModelProperties>,
                    )>;

                    if let Ok(r) = result.downcast::<EntityData>() {
                        log_once::debug_once!("Result has been successfully downcasted");
                        match Arc::try_unwrap(r) {
                            Ok(entity_result) => match entity_result {
                                Ok((
                                    label,
                                    mesh_renderer,
                                    lt,
                                    wt,
                                    camera_config,
                                    light_config,
                                    script_comp,
                                    model_props,
                                )) => {
                                    log::debug!("Entity loaded: {}", label);

                                    let mut builder = hecs::EntityBuilder::new();
                                    builder.add(label.clone());
                                    builder.add(lt);
                                    builder.add(wt);

                                    if let Some(mut renderer) = mesh_renderer {
                                        renderer.update(wt.inner());
                                        builder.add(renderer);
                                    }

                                    if let Some(cam_cfg) = camera_config {
                                        let camera = Camera::new(
                                            graphics.clone(),
                                            cam_cfg.clone().into(),
                                            Some(&cam_cfg.label),
                                        );
                                        let component = CameraComponent::new();
                                        builder.add(camera);
                                        builder.add(component);
                                    }

                                    if let Some((light_comp, transform)) = light_config {
                                        let graphics_clone = graphics.clone();
                                        let label_str = label.to_string();
                                        let light_comp_clone = light_comp.clone();
                                        let light_future = async move {
                                            Light::new(
                                                graphics_clone,
                                                light_comp_clone,
                                                transform,
                                                Some(&label_str),
                                            )
                                            .await
                                        };
                                        let light_handle =
                                            graphics.future_queue.push(Box::pin(light_future));
                                        self.light_spawn_queue.push(light_handle);
                                        builder.add(light_comp);
                                    }

                                    if let Some(script) = script_comp {
                                        builder.add(script);
                                    }

                                    if let Some(props) = model_props {
                                        builder.add(props);
                                    }

                                    self.world.spawn(builder.build());
                                    success!("Spawned entity '{}' successfully", label);
                                    completed.push(i);
                                }
                                Err(e) => {
                                    fatal!("Unable to load entity: {}", e);
                                    completed.push(i);
                                }
                            },
                            Err(_) => {
                                log_once::warn_once!("Cannot unwrap Arc result");
                                completed.push(i);
                            }
                        }
                    }
                } else {
                    log_once::debug_once!("Handle exchanging failed, probably not ready yet");
                }
            } else {
                log_once::debug_once!("Spawn has no handle");
            }
        }

        for &i in completed.iter().rev() {
            log_once::debug_once!("Removing item {} from pending spawn list", i);
            spawn_list.remove(i);
        }

        Ok(())
    }
}
