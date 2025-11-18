use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use ron::ser::PrettyConfig;
use dropbear_engine::graphics::SharedGraphicsContext;
use tokio::sync::mpsc::UnboundedSender;
use dropbear_engine::camera::{Camera, CameraBuilder};
use dropbear_engine::entity::{EntityTransform, MeshRenderer, Transform};
use glam::{DQuat, DVec3};
use dropbear_engine::lighting::{Light, LightComponent};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use dropbear_engine::asset::ASSET_REGISTRY;
use dropbear_engine::model::Model;
use dropbear_engine::utils::ResourceReferenceType;
use crate::camera::{CameraComponent, CameraType};
use crate::hierarchy::{Parent, SceneHierarchy};
use crate::states::{CameraConfig, Label, LightConfig, ModelProperties, ScriptComponent, SerializedMeshRenderer, WorldLoadingStatus, PROJECT};
use crate::utils::ResolveReference;

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct SceneEntity {
    #[serde(default)]
    pub label: Label,
    #[serde(default)]
    pub components: Vec<Box<dyn dropbear_traits::SerializableComponent>>,

    #[serde(skip)]
    pub entity_id: Option<hecs::Entity>,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct SceneSettings { /* *crickets* */ }

impl SceneSettings {
    pub fn new() -> Self {
        Self {}
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct SceneConfig {
    #[serde(default)]
    pub scene_name: String,

    #[serde(default)]
    pub entities: Vec<SceneEntity>,

    #[serde(default)]
    pub hierarchy_map: SceneHierarchy,

    #[serde(default)]
    pub settings: SceneSettings,

    #[serde(skip)]
    pub path: PathBuf,
}

impl SceneConfig {
    /// Creates a new instance of the scene config
    pub fn new(scene_name: String, path: impl AsRef<Path>) -> Self {
        Self {
            scene_name,
            path: path.as_ref().to_path_buf(),
            entities: Vec::new(),
            hierarchy_map: SceneHierarchy::new(),
            settings: SceneSettings::new(),
        }
    }

    /// Helper function to load a component and add it to the entity builder
    async fn load_component(
        component: Box<dyn dropbear_traits::SerializableComponent>,
        builder: &mut hecs::EntityBuilder,
        graphics: Arc<SharedGraphicsContext>,
        label: &str,
    ) -> anyhow::Result<()> {
        if let Some(transform) = component.as_any().downcast_ref::<EntityTransform>() {
            builder.add(*transform);
        } else if let Some(renderer) = component.as_any().downcast_ref::<SerializedMeshRenderer>() {
            let renderer = renderer.clone();
            let mut model = match &renderer.handle.ref_type {
                ResourceReferenceType::None => {
                    log::error!("Resource reference type is None for entity '{}', not supported, skipping", label);
                    return Ok(());
                }
                ResourceReferenceType::Plane => {
                    log::error!("Resource reference type is Plane for entity '{}', not supported (being remade), skipping", label);
                    return Ok(());
                }
                ResourceReferenceType::File(reference) => {
                    let path = &renderer.handle.resolve()?;

                    log::debug!(
                        "Path for entity {} is {} from reference {}",
                        label,
                        path.display(),
                        reference
                    );

                    MeshRenderer::from_path(graphics.clone(), &path, Some(label)).await?
                }
                ResourceReferenceType::Bytes(bytes) => {
                    log::info!("Loading entity from bytes [Len: {}]", bytes.len());

                    let model = Model::load_from_memory(
                        graphics.clone(),
                        bytes.clone(),
                        Some(label),
                    ).await?;
                    MeshRenderer::from_handle(model)
                }
                ResourceReferenceType::Cube => {
                    log::info!("Loading entity from cube");

                    let model = Model::load_from_memory(
                        graphics.clone(),
                        include_bytes!("../../resources/models/cube.glb"),
                        Some(label),
                    ).await?;
                    MeshRenderer::from_handle(model)
                }
            };

            if !renderer.material_override.is_empty() {
                for override_entry in &renderer.material_override {
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
                            Model::load(graphics.clone(), &source_path, label_hint).await?;
                        } else {
                            log::warn!(
                                "Material override for '{}' references unsupported resource {:?}",
                                label,
                                override_entry.source_model
                            );
                            continue;
                        }
                    }

                    if let Err(err) = model.apply_material_override(
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
            }
            
            builder.add(model);
        } else if let Some(props) = component.as_any().downcast_ref::<ModelProperties>() {
            builder.add(props.clone());
        } else if let Some(camera_comp) = component.as_any().downcast_ref::<CameraConfig>() {
            let cam_builder = CameraBuilder::from(camera_comp.clone());
            let comp = CameraComponent::from(camera_comp.clone());
            let camera = Camera::new(graphics.clone(), cam_builder, Some(label));
            builder.add_bundle((camera, comp));
        } else if let Some(light_conf) = component.as_any().downcast_ref::<LightConfig>() {
            let light = Light::new(
                graphics.clone(),
                light_conf.light_component.clone(),
                light_conf.transform,
                Some(label)
            ).await;
            builder.add_bundle((light_conf.light_component.clone(), light));
        } else if let Some(script) = component.as_any().downcast_ref::<ScriptComponent>() {
            builder.add(script.clone());
        } else if component.as_any().downcast_ref::<Parent>().is_some() {
            log::debug!("Skipping Parent component for '{}' - will be rebuilt from hierarchy_map", label);
        } else {
            log::warn!(
                "Unknown component type '{}' for entity '{}' - skipping",
                component.type_name(),
                label
            );
        }
        
        Ok(())
    }

    /// Write the scene config to a .eucs file
    pub fn write_to(&self, project_path: impl AsRef<Path>) -> anyhow::Result<()> {
        let ron_str = ron::ser::to_string_pretty(&self, PrettyConfig::default())
            .map_err(|e| anyhow::anyhow!("RON serialization error: {}", e))?;

        let scenes_dir = project_path.as_ref().join("scenes");
        fs::create_dir_all(&scenes_dir)?;

        let config_path = scenes_dir.join(format!("{}.eucs", self.scene_name));
        fs::write(&config_path, ron_str).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(())
    }

    /// Read a scene config from a .eucs file
    pub fn read_from(scene_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let ron_str = fs::read_to_string(scene_path.as_ref())?;
        let mut config: SceneConfig = ron::de::from_str(&ron_str)
            .map_err(|e| anyhow::anyhow!("RON deserialization error: {}", e))?;

        config.path = scene_path.as_ref().to_path_buf();
        Ok(config)
    }

    pub async fn load_into_world(
        &self,
        world: &mut hecs::World,
        graphics: Arc<SharedGraphicsContext>,
        progress_sender: Option<UnboundedSender<WorldLoadingStatus>>,
    ) -> anyhow::Result<hecs::Entity> {
        if let Some(ref s) = progress_sender {
            let _ = s.send(WorldLoadingStatus::Idle);
        }

        log::info!(
            "Loading scene [{}], clearing world with {} entities",
            self.scene_name,
            world.len()
        );
        world.clear();

        #[allow(unused_variables)]
        let project_config = if cfg!(feature = "editor") {
            let cfg = PROJECT.read();
            cfg.project_path.clone()
        } else {
            log::debug!("Not using the editor feature, returning empty pathbuffer");
            PathBuf::new()
        };

        log::info!("World cleared, now has {} entities", world.len());

        let entity_configs: Vec<(usize, SceneEntity)> = {
            let cloned = self.entities.clone();
            cloned
                .into_par_iter()
                .enumerate()
                .map(|(i, e)| (i, e))
                .collect()
        };

        let mut label_to_entity: HashMap<Label, hecs::Entity> = HashMap::new();

        for (index, entity_config) in entity_configs {
            let SceneEntity {
                label, components, entity_id: _,
            } = entity_config;

            let label_for_map = label.clone();
            let label_for_logs = label_for_map.to_string();

            log::debug!("Loading entity: {}", label_for_logs);

            let total = self.entities.len();

            if let Some(ref s) = progress_sender {
                let _ = s.send(WorldLoadingStatus::LoadingEntity {
                    index,
                    name: label_for_logs.clone(),
                    total,
                });
            }

            let mut builder = hecs::EntityBuilder::new();

            builder.add(label_for_map.clone());
            
            let mut has_entity_transform = false;
            
            for component in components {
                if component.as_any().downcast_ref::<EntityTransform>().is_some() {
                    has_entity_transform = true;
                }
                
                Self::load_component(component, &mut builder, graphics.clone(), &label_for_logs).await?;
            }

            let entity = world.spawn(builder.build());
            
            if has_entity_transform {
                if let Ok(mut query) = world.query_one::<(&EntityTransform, Option<&mut MeshRenderer>, Option<&mut Light>, Option<&mut LightComponent>)>(entity) {
                    if let Some((entity_transform, renderer_opt, light_opt, light_comp_opt)) = query.get() {
                        let transform = entity_transform.sync();
                        
                        if let Some(renderer) = renderer_opt {
                            renderer.update(&transform);
                            log::debug!("Updated renderer transform for '{}'", label_for_logs);
                        }
                        
                        if let (Some(light), Some(light_comp)) = (light_opt, light_comp_opt) {
                            light.update(light_comp, &transform);
                            log::debug!("Updated light transform for '{}'", label_for_logs);
                        }
                    }
                }
            }

            if let Some(previous) = label_to_entity.insert(label_for_map.clone(), entity) {
                log::warn!(
                    "Duplicate entity label '{}' detected; previous entity {:?} will be overwritten in hierarchy mapping",
                    label_for_logs,
                    previous
                );
            }

            log::debug!("Loaded entity '{}'", label_for_logs);
        }

        let mut parent_children_map: HashMap<Label, Vec<Label>> = HashMap::new();
        
        for entity_label in label_to_entity.keys() {
            let children: Vec<Label> = self.hierarchy_map.get_children(entity_label).to_vec();
            if !children.is_empty() {
                parent_children_map.insert(entity_label.clone(), children);
            }
        }
        
        for (parent_label, child_labels) in parent_children_map {
            let Some(&parent_entity) = label_to_entity.get(&parent_label) else {
                log::warn!(
                    "Unable to resolve parent entity '{}' while rebuilding hierarchy",
                    parent_label
                );
                continue;
            };

            let mut resolved_children = Vec::new();
            for child_label in child_labels {
                if let Some(&child_entity) = label_to_entity.get(&child_label) {
                    resolved_children.push(child_entity);
                } else {
                    log::warn!(
                        "Unable to resolve child '{}' for parent '{}'",
                        child_label,
                        parent_label
                    );
                }
            }

            if resolved_children.is_empty() {
                continue;
            }

            let mut local_insert_one: Option<hecs::Entity> = None;

            match world.query_one::<&mut Parent>(parent_entity) {
                Ok(mut parent_query) => {
                    if let Some(parent_component) = parent_query.get() {
                        parent_component.clear();
                        parent_component
                            .children_mut()
                            .extend(resolved_children.iter().copied());
                    } else {
                        local_insert_one = Some(parent_entity);
                    }
                }
                Err(e) => {
                    log::warn!(
                        "Failed to query Parent component for entity {:?}: {}",
                        parent_entity,
                        e
                    );
                    local_insert_one = Some(parent_entity);
                }
            }

            if let Some(parent_entity) = local_insert_one
                && let Err(e) = world.insert_one(parent_entity, Parent::new(resolved_children))
            {
                log::error!(
                    "Failed to attach Parent component to entity {:?}: {}",
                    parent_entity,
                    e
                );
            }
        }

        {
            let mut has_light = false;
            if world
                .query::<(&LightComponent, &Light)>()
                .iter()
                .next()
                .is_some()
            {
                has_light = true;
            }

            if !has_light {
                log::info!("No lights in scene, spawning default light");
                if let Some(ref s) = progress_sender {
                    let _ = s.send(WorldLoadingStatus::LoadingEntity {
                        index: 0,
                        name: String::from("Default Light"),
                        total: 1,
                    });
                }
                let comp = LightComponent::directional(glam::DVec3::ONE, 1.0);
                let light_direction = LightComponent::default_direction();
                let rotation =
                    DQuat::from_rotation_arc(DVec3::new(0.0, 0.0, -1.0), light_direction);
                let trans = Transform {
                    position: glam::DVec3::new(2.0, 4.0, 2.0),
                    rotation,
                    ..Default::default()
                };
                let entity_trans = EntityTransform::new(trans, Transform::default());
                let light =
                    Light::new(graphics.clone(), comp.clone(), trans, Some("Default Light")).await;

                {
                    world.spawn((
                        Label::from("Default Light"),
                        comp,
                        entity_trans,
                        light,
                        ModelProperties::default(),
                    ));
                }
            }
        }

        log::info!(
            "Loaded {} entities from scene",
            self.entities.len()
        );
        #[cfg(feature = "editor")]
        {
            let debug_camera = {
                world
                    .query::<(&Camera, &CameraComponent)>()
                    .iter()
                    .find_map(|(entity, (_, component))| {
                        if matches!(component.camera_type, CameraType::Debug) {
                            Some(entity)
                        } else {
                            None
                        }
                    })
            };

            {
                if let Some(camera_entity) = debug_camera {
                    log::info!("Using existing debug camera for editor");
                    Ok(camera_entity)
                } else {
                    log::info!("No debug camera found, creating viewport camera for editor");
                    if let Some(ref s) = progress_sender {
                        let _ = s.send(WorldLoadingStatus::LoadingEntity {
                            index: 0,
                            name: String::from("Viewport Camera"),
                            total: 1,
                        });
                    }
                    let camera = Camera::predetermined(graphics.clone(), Some("Viewport Camera"));
                    let component = crate::camera::DebugCamera::new();
                    let camera_entity = { world.spawn((camera, component)) };
                    Ok(camera_entity)
                }
            }
        }

        #[cfg(not(feature = "editor"))]
        {
            let player_camera = world
                .query::<(&Camera, &CameraComponent)>()
                .iter()
                .find_map(|(entity, (_, component))| {
                    if matches!(component.camera_type, CameraType::Player) {
                        Some(entity)
                    } else {
                        None
                    }
                });

            if let Some(camera_entity) = player_camera {
                log::info!("Using player camera for runtime");
                Ok(camera_entity)
            } else {
                panic!("Runtime mode requires a player camera, but none was found in the scene!");
            }
        }
    }
}