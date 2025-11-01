use crate::editor::Editor;
use dropbear_engine::entity::MeshRenderer;
use dropbear_engine::future::FutureQueue;
use dropbear_engine::graphics::SharedGraphicsContext;
use dropbear_engine::model::Model;
use dropbear_engine::procedural::plane::PlaneBuilder;
use dropbear_engine::utils::ResourceReferenceType;
pub(crate) use eucalyptus_core::spawn::{PENDING_SPAWNS, PendingSpawnController};
use eucalyptus_core::states::{PROJECT, Value};
use eucalyptus_core::success;
use eucalyptus_core::utils::PROTO_TEXTURE;
use std::sync::Arc;

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
            log_once::debug_once!(
                "Caught pending spawn! Info: {} of type {}",
                spawn.asset_name,
                spawn.asset_path
            );
            if spawn.handle.is_none() {
                log_once::debug_once!("Pending spawn does NOT have a handle, creating new one now");
                let graphics_clone = graphics.clone();
                let asset_name = spawn.asset_name.clone();
                let asset_path = spawn.asset_path.ref_type.clone();
                let properties = spawn.properties.clone();

                let func = async move {
                    match asset_path {
                        ResourceReferenceType::None => {
                            Err(anyhow::anyhow!("No asset path available"))
                        }
                        ResourceReferenceType::File(file) => {
                            let path = {
                                let _guard = PROJECT.read();
                                _guard.project_path.clone()
                            };
                            let resource = path.join("resources").join(file);
                            MeshRenderer::from_path(graphics_clone, resource, Some(&asset_name))
                                .await
                        }
                        ResourceReferenceType::Bytes(bytes) => {
                            let model = Model::load_from_memory(
                                graphics_clone.clone(),
                                &bytes,
                                Some(&asset_name),
                            )
                            .await?;
                            Ok(MeshRenderer::from_handle(model))
                        }
                        ResourceReferenceType::Plane => {
                            let get_float = |key: &str| -> anyhow::Result<f32> {
                                let val = properties
                                    .custom_properties
                                    .iter()
                                    .find(|p| p.key == key)
                                    .ok_or_else(|| {
                                        anyhow::anyhow!("Entity has no {} property", key)
                                    })?;
                                match val.value {
                                    Value::Float(f) => Ok(f as f32),
                                    _ => Err(anyhow::anyhow!("{} is not a float", key)),
                                }
                            };

                            let get_int = |key: &str| -> anyhow::Result<u32> {
                                let val = properties
                                    .custom_properties
                                    .iter()
                                    .find(|p| p.key == key)
                                    .ok_or_else(|| {
                                        anyhow::anyhow!("Entity has no {} property", key)
                                    })?;
                                match val.value {
                                    Value::Int(i) => Ok(i as u32),
                                    _ => Err(anyhow::anyhow!("{} is not an int", key)),
                                }
                            };

                            let width = get_float("width")?;
                            let height = get_float("height")?;
                            let tiles_x = get_int("tiles_x")?;
                            let tiles_z = get_int("tiles_z")?;

                            PlaneBuilder::new()
                                .with_size(width, height)
                                .with_tiles(tiles_x, tiles_z)
                                .build(graphics_clone, PROTO_TEXTURE, Some(&asset_name))
                                .await
                        }
                    }
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
                    if let Ok(r) = result.downcast::<anyhow::Result<MeshRenderer>>() {
                        log_once::debug_once!("Result has been successfully downcasted");
                        match Arc::try_unwrap(r) {
                            Ok(entity) => match entity {
                                Ok(entity) => {
                                    log::debug!("Entity loaded");
                                    self.world.spawn((
                                        entity,
                                        spawn.transform,
                                        spawn.properties.clone(),
                                    ));
                                    success!("Spawned entity successfully");
                                    completed.push(i);
                                }
                                Err(e) => {
                                    log_once::error_once!("Unable to load model: {}", e);
                                    completed.push(i);
                                }
                            },
                            Err(_) => {
                                return {
                                    log_once::warn_once!("Cannot unwrap Arc result");
                                    completed.push(i);
                                    Ok(())
                                };
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
