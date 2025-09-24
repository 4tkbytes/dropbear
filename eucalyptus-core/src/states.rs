use crate::camera::DebugCamera;
use crate::camera::{CameraComponent, CameraFollowTarget, CameraType};
use crate::utils::PROTO_TEXTURE;
use chrono::Utc;
use dropbear_engine::camera::{Camera, CameraBuilder};
use dropbear_engine::entity::{AdoptedEntity, Transform};
use dropbear_engine::graphics::SharedGraphicsContext;
use dropbear_engine::lighting::{Light, LightComponent};
use dropbear_engine::model::Model;
use dropbear_engine::procedural::plane::PlaneBuilder;
use dropbear_engine::utils::{ResourceReference, ResourceReferenceType};
use egui_dock_fork::DockState;
use glam::DVec3;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::UnboundedSender;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fmt, fs};
use rayon::prelude::*;

pub static PROJECT: Lazy<RwLock<ProjectConfig>> =
    Lazy::new(|| RwLock::new(ProjectConfig::default()));

pub static RESOURCES: Lazy<RwLock<ResourceConfig>> =
    Lazy::new(|| RwLock::new(ResourceConfig::default()));

pub static SOURCE: Lazy<RwLock<SourceConfig>> = Lazy::new(|| RwLock::new(SourceConfig::default()));

pub static SCENES: Lazy<RwLock<Vec<SceneConfig>>> = Lazy::new(|| RwLock::new(Vec::new()));

/// The root config file, responsible for building and other metadata.
///
/// # Location
/// This file is {project_name}.eucp and is located at {project_dir}/
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ProjectConfig {
    pub project_name: String,
    pub project_path: PathBuf,
    pub date_created: String,
    pub date_last_accessed: String,
    #[serde(default)]
    pub dock_layout: Option<DockState<EditorTab>>,
    #[serde(default)]
    pub editor_settings: EditorSettings,
}

impl ProjectConfig {
    /// Creates a new instance of the ProjectConfig. This function is typically used when creating
    /// a new project, with it creating new defaults for everything.
    pub fn new(project_name: String, project_path: impl AsRef<Path>) -> Self {
        let date_created = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        let date_last_accessed = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));

        let mut result = Self {
            project_name,
            project_path: project_path.as_ref().to_path_buf(),
            date_created,
            date_last_accessed,
            editor_settings: Default::default(),
            dock_layout: None,
        };
        let _ = result.load_config_to_memory();
        result
    }

    /// This function writes the [`ProjectConfig`] struct (and other PathBufs) to a file of the choice
    /// under the PathBuf path parameter.
    ///
    /// # Parameters
    /// * path - The root **folder** of the project.
    pub fn write_to(&mut self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        self.load_config_to_memory()?;
        self.date_last_accessed = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        // self.assets = Assets::walk(path);
        let ron_str = ron::ser::to_string_pretty(&self, PrettyConfig::default())
            .map_err(|e| anyhow::anyhow!("RON serialization error: {}", e))?;
        let config_path = path.as_ref().join(format!("{}.eucp", self.project_name.clone().to_lowercase()));
        self.project_path = path.as_ref().to_path_buf();

        fs::write(&config_path, ron_str).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(())
    }

    /// This function reads from the RON and traverses down the different folders to add more information
    /// to the ProjectConfig, such as Assets location and other stuff.
    ///
    /// # Parameters
    /// * path - The root config **file** for the project
    pub fn read_from(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let ron_str = fs::read_to_string(path.as_ref())?;
        let mut config: ProjectConfig = ron::de::from_str(ron_str.as_str())?;
        config.project_path = path.as_ref().parent().unwrap().to_path_buf();
        log::info!("Loaded project!");
        log::debug!("Loaded config info");
        log::debug!("Updating with new content");
        config.load_config_to_memory()?;
        config.write_to_all()?;
        log::debug!("Successfully updated!");
        Ok(config)
    }

    /// This function loads a `source.eucc`, `resources.eucc` or a `{scene}.eucs` config file into memory, allowing
    /// you to reference and load the nodes located inside them.
    pub fn load_config_to_memory(&mut self) -> anyhow::Result<()> {
        let project_root = PathBuf::from(&self.project_path);

        // resource config
        match ResourceConfig::read_from(&project_root) {
            Ok(resources) => {
                let mut cfg = RESOURCES.write();
                *cfg = resources;
            }
            Err(e) => {
                if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                    if io_err.kind() == std::io::ErrorKind::NotFound {
                        log::warn!("resources.eucc not found, creating default.");
                        let default = ResourceConfig {
                            path: project_root.join("resources"),
                            nodes: vec![],
                        };
                        default.write_to(&project_root)?;
                        {
                            let mut cfg = RESOURCES.write();
                            *cfg = default;
                        }
                    } else {
                        log::warn!("Failed to load resources.eucc: {}", e);
                    }
                } else {
                    log::warn!("Failed to load resources.eucc: {}", e);
                }
            }
        }

        // src config
        let mut source_config = SOURCE.write();
        match SourceConfig::read_from(&project_root) {
            Ok(source) => *source_config = source,
            Err(e) => {
                if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                    if io_err.kind() == std::io::ErrorKind::NotFound {
                        log::warn!("source.eucc not found, creating default.");
                        let default = SourceConfig {
                            path: project_root.join("src"),
                            nodes: vec![],
                        };
                        default.write_to(&project_root)?;
                        *source_config = default;
                    } else {
                        log::warn!("Failed to load source.eucc: {}", e);
                    }
                } else {
                    log::warn!("Failed to load source.eucc: {}", e);
                }
            }
        }

        // scenes
        let mut scene_configs = SCENES.write();
        scene_configs.clear();

        // iterate through each scene file in the folder
        let scene_folder = &project_root.join("scenes");

        if !scene_folder.exists() {
            fs::create_dir_all(scene_folder)?;
        }

        for scene_entry in fs::read_dir(scene_folder)? {
            let scene_entry = scene_entry?;
            let path = scene_entry.path();

            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("eucs") {
                match SceneConfig::read_from(&path) {
                    Ok(scene) => {
                        log::debug!("Loaded scene: {}", scene.scene_name);
                        scene_configs.push(scene);
                    }
                    Err(e) => {
                        if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                            if io_err.kind() == std::io::ErrorKind::NotFound {
                                log::warn!("Scene file {:?} not found", path);
                            } else {
                                log::warn!("Failed to load scene file {:?}: {}", path, e);
                            }
                        } else {
                            log::warn!("Failed to load scene file {:?}: {}", path, e);
                        }
                    }
                }
            }
        }

        if scene_configs.is_empty() {
            log::info!("No scenes found, creating default scene");
            let default_scene =
                SceneConfig::new("Default".to_string(), scene_folder.join("default.eucs"));
            default_scene.write_to(&project_root)?;
            scene_configs.push(default_scene);
        }

        Ok(())
    }

    /// # Parameters
    /// * path - The root folder of the project
    pub fn write_to_all(&mut self) -> anyhow::Result<()> {
        let path = self.project_path.clone();

        {
            let resources_config = RESOURCES.read();
            resources_config.write_to(&path)?;
        }

        {
            let source_config = SOURCE.read();
            source_config.write_to(&path)?;
        }

        {
            let scene_configs = SCENES.read();
            for scene in scene_configs.iter() {
                scene.write_to(&path)?;
            }
        }

        self.write_to(&path)?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Node {
    File(File),
    Folder(Folder),
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct File {
    pub name: String,
    pub path: PathBuf,
    pub resource_type: Option<ResourceType>,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct Folder {
    pub name: String,
    pub path: PathBuf,
    pub nodes: Vec<Node>,
}

/// The type of resource
#[derive(Debug, Serialize, Deserialize, Clone, Hash)]
pub enum ResourceType {
    Unknown,
    Model,
    Thumbnail,
    Texture,
    Shader,
}

impl Display for ResourceType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let str = match self {
            ResourceType::Unknown => "unknown",
            ResourceType::Model => "model",
            ResourceType::Texture => "texture",
            ResourceType::Shader => "shader",
            ResourceType::Thumbnail => "thumbnail",
        };
        write!(f, "{}", str)
    }
}

/// The resource config.
#[derive(Default, Debug, Serialize, Deserialize)]
pub struct ResourceConfig {
    /// The path to the resource folder.
    pub path: PathBuf,
    /// The files and folders of the assets
    pub nodes: Vec<Node>,
}

impl ResourceConfig {
    /// Builds a resource path from the ProjectConfiguration's project_path (or a string)
    #[allow(dead_code)]
    pub fn build_path(project_path: String) -> PathBuf {
        PathBuf::from(project_path).join("resources/resources.eucc")
    }

    /// # Parameters
    /// - path: The root **folder** of the project
    pub fn write_to(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let resource_dir = path.as_ref().join("resources");
        let updated_config = ResourceConfig {
            path: resource_dir.clone(),
            nodes: collect_nodes(&resource_dir, path.as_ref(), vec!["thumbnails"].as_slice()),
        };
        let ron_str = ron::ser::to_string_pretty(&updated_config, PrettyConfig::default())
            .map_err(|e| anyhow::anyhow!("RON serialization error: {}", e))?;
        let config_path = path.as_ref().join("resources").join("resources.eucc");
        fs::create_dir_all(config_path.parent().unwrap())?;
        fs::write(&config_path, ron_str).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(())
    }

    /// Updates the in-memory ResourceConfig by re-scanning the resource directory.
    pub fn update_mem(&mut self) -> anyhow::Result<ResourceConfig> {
        let resource_dir = self.path.clone();
        let project_path = resource_dir.parent().unwrap_or(&resource_dir).to_path_buf();
        let updated_config = ResourceConfig {
            path: resource_dir.clone(),
            nodes: collect_nodes(&resource_dir, &project_path, vec!["thumbnails"].as_slice()),
        };
        Ok(updated_config)
    }

    /// # Parameters
    /// - path: The location to the **resources.eucc** file
    pub fn read_from(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let config_path = path.as_ref().join("resources").join("resources.eucc");
        let ron_str = fs::read_to_string(&config_path)?;
        let config: ResourceConfig = ron::de::from_str(&ron_str)
            .map_err(|e| anyhow::anyhow!("RON deserialization error: {}", e))?;
        Ok(config)
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct SourceConfig {
    /// The path to the resource folder.
    pub path: PathBuf,
    /// The files and folders of the assets
    pub nodes: Vec<Node>,
}

impl SourceConfig {
    /// Builds a source path from the ProjectConfiguration's project_path (or a string)
    #[allow(dead_code)]
    pub fn build_path(project_path: String) -> PathBuf {
        PathBuf::from(project_path).join("src/source.eucc")
    }

    /// # Parameters
    /// - path: The root **folder** of the project
    pub fn write_to(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        let resource_dir = path.as_ref().join("src");
        let updated_config = SourceConfig {
            path: resource_dir.clone(),
            nodes: collect_nodes(&resource_dir, path.as_ref(), vec!["scripts"].as_slice()),
        };

        let ron_str = ron::ser::to_string_pretty(&updated_config, PrettyConfig::default())
            .map_err(|e| anyhow::anyhow!("RON serialisation error: {}", e))?;
        let config_path = path.as_ref().join("src").join("source.eucc");
        fs::create_dir_all(config_path.parent().unwrap())?;
        fs::write(&config_path, ron_str).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(())
    }

    /// # Parameters
    /// - path: The location to the **source.eucc** file
    pub fn read_from(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let config_path = path.as_ref().join("src").join("source.eucc");
        let ron_str = fs::read_to_string(&config_path)?;
        let config: SourceConfig = ron::de::from_str(&ron_str)
            .map_err(|e| anyhow::anyhow!("RON deserialization error: {}", e))?;
        Ok(config)
    }
}

fn collect_nodes(dir: impl AsRef<Path>, project_path: impl AsRef<Path>, exclude_list: &[&str]) -> Vec<Node> {
    let mut nodes = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            let name = entry_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            if entry_path.is_dir() && exclude_list.iter().any(|ex| ex.to_string() == *name) {
                log::debug!("Skipped past folder {:?}", name);
                continue;
            }

            if entry_path.is_dir() {
                let folder_nodes = collect_nodes(&entry_path, project_path.as_ref(), exclude_list);
                nodes.push(Node::Folder(Folder {
                    name,
                    path: entry_path.clone(),
                    nodes: folder_nodes,
                }));
            } else {
                let parent_folder = entry_path
                    .parent()
                    .and_then(|p| p.file_name())
                    .map(|n| n.to_string_lossy().to_lowercase())
                    .unwrap_or_default();

                let resource_type = if parent_folder.contains("model") {
                    Some(ResourceType::Model)
                } else if parent_folder.contains("texture") {
                    Some(ResourceType::Texture)
                } else if parent_folder.contains("shader") {
                    Some(ResourceType::Shader)
                } else {
                    Some(ResourceType::Unknown)
                };

                // Store relative path from the project root instead of absolute path
                let relative_path = entry_path
                    .strip_prefix(project_path.as_ref())
                    .unwrap_or(&entry_path)
                    .to_path_buf();

                nodes.push(Node::File(File {
                    name,
                    path: relative_path,
                    resource_type,
                }));
            }
        }
    }
    nodes
}

#[derive(Clone, Debug, Serialize, Deserialize, Hash)]
pub enum EntityNode {
    Entity {
        id: hecs::Entity,
        name: String,
    },
    Script {
        name: String,
        path: PathBuf,
    },
    Light {
        id: hecs::Entity,
        name: String,
    },
    Camera {
        id: hecs::Entity,
        name: String,
        camera_type: CameraType,
    },
    Group {
        name: String,
        children: Vec<EntityNode>,
        collapsed: bool,
    },
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct ScriptComponent {
    pub name: String,
    pub path: PathBuf,
}

impl EntityNode {
    pub fn from_world(world: &hecs::World) -> Vec<Self> {
        let mut nodes = Vec::new();
        let mut handled = std::collections::HashSet::new();

        for (id, (script, _transform, adopted)) in world
            .query::<(
                &ScriptComponent,
                &dropbear_engine::entity::Transform,
                &dropbear_engine::entity::AdoptedEntity,
            )>()
            .iter()
        {
            let name = adopted.model.label.clone();
            let mut children = vec![
                EntityNode::Entity {
                    id,
                    name: name.clone(),
                },
                EntityNode::Script {
                    name: script.name.clone(),
                    path: script.path.clone(),
                },
            ];

            // Check if this entity also has camera components
            if let Ok(mut camera_query) = world.query_one::<(&Camera, &CameraComponent)>(id) {
                if let Some((camera, component)) = camera_query.get() {
                    children.push(EntityNode::Camera {
                        id,
                        name: camera.label.clone(),
                        camera_type: component.camera_type,
                    });
                }
            }

            nodes.push(EntityNode::Group {
                name: name.clone(),
                children,
                collapsed: false,
            });
            handled.insert(id);
        }

        // Handle single entities (and potentially cameras)
        for (id, (_, adopted)) in world
            .query::<(
                &dropbear_engine::entity::Transform,
                &dropbear_engine::entity::AdoptedEntity,
            )>()
            .iter()
        {
            if handled.contains(&id) {
                continue;
            }
            let name = adopted.model.label.clone();

            // Check if this entity has camera components
            if let Ok(mut camera_query) = world.query_one::<(&Camera, &CameraComponent)>(id) {
                if let Some((camera, component)) = camera_query.get() {
                    // Create a group with the entity and its camera component
                    nodes.push(EntityNode::Group {
                        name: name.clone(),
                        children: vec![
                            EntityNode::Entity {
                                id,
                                name: name.clone(),
                            },
                            EntityNode::Camera {
                                id,
                                name: camera.label.clone(),
                                camera_type: component.camera_type,
                            },
                        ],
                        collapsed: false,
                    });
                } else {
                    // Regular entity without camera components
                    nodes.push(EntityNode::Entity { id, name });
                }
            } else {
                // Regular entity without camera components
                nodes.push(EntityNode::Entity { id, name });
            }
        }

        // lights
        for (id, (_transform, _light_comp, light)) in world
            .query::<(&dropbear_engine::entity::Transform, &LightComponent, &Light)>()
            .iter()
        {
            if handled.contains(&id) {
                continue;
            }
            nodes.push(EntityNode::Light {
                id,
                name: light.label().to_string(),
            });
            handled.insert(id);
        }

        // Handle standalone cameras (cameras without AdoptedEntity - like viewport cameras)
        for (entity, (camera, component)) in world.query::<(&Camera, &CameraComponent)>().iter() {
            if world.get::<&AdoptedEntity>(entity).is_err() {
                nodes.push(EntityNode::Camera {
                    id: entity,
                    name: camera.label.clone(),
                    camera_type: component.camera_type,
                });
            }
        }

        nodes
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CameraConfig {
    pub label: String,
    pub camera_type: CameraType,

    pub position: [f64; 3],
    pub target: [f64; 3],
    pub up: [f64; 3],
    pub aspect: f64,
    pub fov: f32,
    pub near: f32,
    pub far: f32,

    pub speed: f32,
    pub sensitivity: f32,

    pub follow_target_entity_label: Option<String>,
    pub follow_offset: Option<[f64; 3]>,
}

impl Default for CameraConfig {
    fn default() -> Self {
        let default = CameraComponent::new();
        Self {
            position: [0.0, 1.0, 2.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            aspect: 16.0 / 9.0,
            fov: 45.0,
            near: 0.1,
            far: 100.0,
            follow_target_entity_label: None,
            follow_offset: None,
            label: String::new(),
            camera_type: CameraType::Normal,
            speed: default.speed as f32,
            sensitivity: default.sensitivity as f32,
        }
    }
}

impl CameraConfig {
    pub fn from_ecs_camera(
        camera: &Camera,
        component: &CameraComponent,
        follow_target: Option<&CameraFollowTarget>,
    ) -> Self {
        Self {
            position: camera.position().to_array(),
            target: camera.target.to_array(),
            label: camera.label.clone(),
            camera_type: component.camera_type,
            up: camera.up.to_array(),
            aspect: camera.aspect,
            fov: camera.fov_y as f32,
            near: camera.znear as f32,
            far: camera.zfar as f32,
            speed: component.speed as f32,
            sensitivity: component.sensitivity as f32,
            follow_target_entity_label: follow_target.map(|target| target.follow_target.clone()),
            follow_offset: follow_target.map(|target| target.offset.to_array()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SceneEntity {
    pub model_path: ResourceReference,
    pub label: String,
    pub transform: Transform,
    pub properties: ModelProperties,
    pub script: Option<ScriptComponent>,
    pub camera: Option<CameraConfig>,

    #[serde(skip)]
    #[allow(dead_code)]
    pub entity_id: Option<hecs::Entity>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ModelProperties {
    pub custom_properties: HashMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Vec3([f32; 3]),
}

impl ModelProperties {
    pub fn new() -> Self {
        Self {
            custom_properties: HashMap::new(),
        }
    }

    pub fn set_property(&mut self, key: String, value: Value) {
        self.custom_properties.insert(key, value);
    }

    pub fn get_property(&self, key: &str) -> Option<&Value> {
        self.custom_properties.get(key)
    }
}

impl Default for ModelProperties {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct SceneConfig {
    pub scene_name: String,
    pub entities: Vec<SceneEntity>,
    pub cameras: Vec<CameraConfig>,
    pub lights: Vec<LightConfig>,
    // todo later
    // pub settings: SceneSettings,
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
            cameras: Vec::new(),
            lights: Vec::new(),
        }
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
        progress_sender: Option<UnboundedSender<WorldLoadingStatus>>
    ) -> anyhow::Result<hecs::Entity> {

        if let Some(ref s) = progress_sender {
            let _ = s.send(WorldLoadingStatus::Idle);
        }

        log::info!(
            "Loading scene [{}], clearing world with {} entities",
            self.scene_name,
            world.len()
        );
        { world.clear(); }

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
            cloned.into_par_iter().enumerate().map(|(i, e)| (i, e)).collect()
        };

        for (index, entity_config) in entity_configs {
            log::debug!("Loading entity: {}", entity_config.label);

            let total = self.entities.len();

            if let Some(ref s) = progress_sender {
                let _ = s.send(WorldLoadingStatus::LoadingEntity { index, name: entity_config.label.clone(), total });
            }

            let result = match &entity_config.model_path.ref_type {
                ResourceReferenceType::File(reference) => {
                    let path: PathBuf = {
                        if cfg!(feature = "editor") {
                            log::debug!("Using feature editor");
                            entity_config
                                .model_path
                                .to_project_path(project_config.clone())
                                .ok_or_else(|| {
                                    anyhow::anyhow!(
                                            "Unable to convert resource reference [{}] to project path",
                                            reference
                                        )
                                })?
                        } else {
                            log::debug!("Using feature data-only");
                            entity_config.model_path.to_executable_path()?
                        }
                    };
                    log::debug!(
                            "Path for entity {} is {} from reference {}",
                            entity_config.label,
                            path.display(),
                            reference
                        );

                    let adopted =
                        AdoptedEntity::new(graphics.clone(), &path, Some(&entity_config.label))
                            .await;
                    let transform = entity_config.transform;

                    let _entity = if let Some(camera_config) = &entity_config.camera {
                        let camera = Camera::new(
                            graphics.clone(),
                            CameraBuilder {
                                eye: DVec3::from_array(camera_config.position),
                                target: DVec3::from_array(camera_config.target),
                                up: DVec3::from_array(camera_config.up),
                                aspect: camera_config.aspect,
                                fov_y: camera_config.fov as f64,
                                znear: camera_config.near as f64,
                                zfar: camera_config.far as f64,
                                speed: camera_config.speed as f64,
                                sensitivity: camera_config.sensitivity as f64,
                            },
                            Some(&camera_config.label),
                        );

                        let camera_component = CameraComponent {
                            speed: camera_config.speed as f64,
                            sensitivity: camera_config.sensitivity as f64,
                            fov_y: camera_config.fov as f64,
                            camera_type: camera_config.camera_type,
                        };

                        if let Some(script_config) = &entity_config.script {
                            let script = ScriptComponent {
                                name: script_config.name.clone(),
                                path: script_config.path.clone(),
                            };
                            if let (Some(target_label), Some(offset)) = (
                                &camera_config.follow_target_entity_label,
                                &camera_config.follow_offset,
                            ) {
                                let follow_target = CameraFollowTarget {
                                    follow_target: target_label.clone(),
                                    offset: DVec3::from_array(*offset),
                                };
                                world.spawn((adopted, transform, script, entity_config.properties.clone(), camera, camera_component, follow_target))
                            } else {
                                world.spawn((adopted, transform, script, entity_config.properties.clone(), camera, camera_component))
                            }
                        } else {
                            if let (Some(target_label), Some(offset)) = (
                                &camera_config.follow_target_entity_label,
                                &camera_config.follow_offset,
                            ) {
                                let follow_target = CameraFollowTarget {
                                    follow_target: target_label.clone(),
                                    offset: DVec3::from_array(*offset),
                                };
                                world.spawn((adopted, transform, entity_config.properties.clone(), camera, camera_component, follow_target))
                            } else {
                                world.spawn((adopted, transform, entity_config.properties.clone(), camera, camera_component))
                            }
                        }
                    } else {
                        if let Some(script_config) = &entity_config.script {
                            let script = ScriptComponent {
                                name: script_config.name.clone(),
                                path: script_config.path.clone(),
                            };
                            world.spawn((adopted, transform, script, entity_config.properties.clone()))
                        } else {
                            world.spawn((adopted, transform, entity_config.properties.clone()))
                        }
                    };

                    Ok(())
                }
                ResourceReferenceType::Bytes(bytes) => {
                    log::info!("Loading entity from bytes [Len: {}]", bytes.len());
                    let bytes = bytes.to_owned();

                    let model = Model::load_from_memory(
                        graphics.clone(),
                        bytes,
                        Some(&entity_config.label),
                    )
                        .await?;
                    let adopted = AdoptedEntity::adopt(graphics.clone(), model).await;

                    let transform = entity_config.transform;

                    let _entity = if let Some(camera_config) = &entity_config.camera {
                        // Entity has camera components
                        let camera = Camera::new(
                            graphics.clone(),
                            CameraBuilder {
                                eye: DVec3::from_array(camera_config.position),
                                target: DVec3::from_array(camera_config.target),
                                up: DVec3::from_array(camera_config.up),
                                aspect: camera_config.aspect,
                                fov_y: camera_config.fov as f64,
                                znear: camera_config.near as f64,
                                zfar: camera_config.far as f64,
                                speed: camera_config.speed as f64,
                                sensitivity: camera_config.sensitivity as f64,
                            },
                            Some(&camera_config.label),
                        );

                        let camera_component = CameraComponent {
                            speed: camera_config.speed as f64,
                            sensitivity: camera_config.sensitivity as f64,
                            fov_y: camera_config.fov as f64,
                            camera_type: camera_config.camera_type,
                        };

                        if let Some(script_config) = &entity_config.script {
                            let script = ScriptComponent {
                                name: script_config.name.clone(),
                                path: script_config.path.clone(),
                            };
                            if let (Some(target_label), Some(offset)) = (
                                &camera_config.follow_target_entity_label,
                                &camera_config.follow_offset,
                            ) {
                                let follow_target = CameraFollowTarget {
                                    follow_target: target_label.clone(),
                                    offset: DVec3::from_array(*offset),
                                };
                                world.spawn((adopted, transform, script, entity_config.properties.clone(), camera, camera_component, follow_target))
                            } else {
                                world.spawn((adopted, transform, script, entity_config.properties.clone(), camera, camera_component))
                            }
                        } else {
                            if let (Some(target_label), Some(offset)) = (
                                &camera_config.follow_target_entity_label,
                                &camera_config.follow_offset,
                            ) {
                                let follow_target = CameraFollowTarget {
                                    follow_target: target_label.clone(),
                                    offset: DVec3::from_array(*offset),
                                };
                                world.spawn((adopted, transform, entity_config.properties.clone(), camera, camera_component, follow_target))
                            } else {
                                world.spawn((adopted, transform, entity_config.properties.clone(), camera, camera_component))
                            }
                        }
                    } else {
                        // Entity without camera components
                        if let Some(script_config) = &entity_config.script {
                            let script = ScriptComponent {
                                name: script_config.name.clone(),
                                path: script_config.path.clone(),
                            };
                            world.spawn((adopted, transform, script, entity_config.properties.clone()))
                        } else {
                            world.spawn((adopted, transform, entity_config.properties.clone()))
                        }
                    };

                    Ok(())
                }
                ResourceReferenceType::Plane => {
                    let width = entity_config
                        .properties
                        .custom_properties
                        .get("width")
                        .ok_or_else(|| anyhow::anyhow!("Entity has no width property"))?;
                    let width = match width {
                        Value::Float(width) => width,
                        _ => panic!("Entity has a width property that is not a float"),
                    };
                    let height = entity_config
                        .properties
                        .custom_properties
                        .get("height")
                        .ok_or_else(|| anyhow::anyhow!("Entity has no height property"))?;
                    let height = match height {
                        Value::Float(height) => height,
                        _ => panic!("Entity has a height property that is not a float"),
                    };
                    let tiles_x = entity_config
                        .properties
                        .custom_properties
                        .get("tiles_x")
                        .ok_or_else(|| anyhow::anyhow!("Entity has no tiles_x property"))?;
                    let tiles_x = match tiles_x {
                        Value::Int(tiles_x) => tiles_x,
                        _ => panic!("Entity has a tiles_x property that is not an int"),
                    };
                    let tiles_z = entity_config
                        .properties
                        .custom_properties
                        .get("tiles_z")
                        .ok_or_else(|| anyhow::anyhow!("Entity has no tiles_z property"))?;
                    let tiles_z = match tiles_z {
                        Value::Int(tiles_z) => tiles_z,
                        _ => panic!("Entity has a tiles_z property that is not an int"),
                    };

                    let label_clone = entity_config.label.clone();
                    let width_val = *width as f32;
                    let height_val = *height as f32;
                    let tiles_x_val = *tiles_x as u32;
                    let tiles_z_val = *tiles_z as u32;

                    let plane = PlaneBuilder::new()
                        .with_size(width_val, height_val)
                        .with_tiles(tiles_x_val, tiles_z_val)
                        .build(graphics.clone(), PROTO_TEXTURE, Some(&label_clone))
                        .await?;

                    let transform = entity_config.transform;

                    let _entity = if let Some(camera_config) = &entity_config.camera {
                        // Entity has camera components
                        let camera = Camera::new(
                            graphics.clone(),
                            CameraBuilder {
                                eye: DVec3::from_array(camera_config.position),
                                target: DVec3::from_array(camera_config.target),
                                up: DVec3::from_array(camera_config.up),
                                aspect: camera_config.aspect,
                                fov_y: camera_config.fov as f64,
                                znear: camera_config.near as f64,
                                zfar: camera_config.far as f64,
                                speed: camera_config.speed as f64,
                                sensitivity: camera_config.sensitivity as f64,
                            },
                            Some(&camera_config.label),
                        );

                        let camera_component = CameraComponent {
                            speed: camera_config.speed as f64,
                            sensitivity: camera_config.sensitivity as f64,
                            fov_y: camera_config.fov as f64,
                            camera_type: camera_config.camera_type,
                        };

                        if let Some(script_config) = &entity_config.script {
                            let script = ScriptComponent {
                                name: script_config.name.clone(),
                                path: script_config.path.clone(),
                            };
                            if let (Some(target_label), Some(offset)) = (
                                &camera_config.follow_target_entity_label,
                                &camera_config.follow_offset,
                            ) {
                                let follow_target = CameraFollowTarget {
                                    follow_target: target_label.clone(),
                                    offset: DVec3::from_array(*offset),
                                };
                                world.spawn((plane, transform, script, entity_config.properties.clone(), camera, camera_component, follow_target))
                            } else {
                                world.spawn((plane, transform, script, entity_config.properties.clone(), camera, camera_component))
                            }
                        } else {
                            if let (Some(target_label), Some(offset)) = (
                                &camera_config.follow_target_entity_label,
                                &camera_config.follow_offset,
                            ) {
                                let follow_target = CameraFollowTarget {
                                    follow_target: target_label.clone(),
                                    offset: DVec3::from_array(*offset),
                                };
                                world.spawn((plane, transform, entity_config.properties.clone(), camera, camera_component, follow_target))
                            } else {
                                world.spawn((plane, transform, entity_config.properties.clone(), camera, camera_component))
                            }
                        }
                    } else {
                        // Entity without camera components
                        if let Some(script_config) = &entity_config.script {
                            let script = ScriptComponent {
                                name: script_config.name.clone(),
                                path: script_config.path.clone(),
                            };
                            world.spawn((plane, transform, script, entity_config.properties.clone()))
                        } else {
                            world.spawn((plane, transform, entity_config.properties.clone()))
                        }
                    };

                    Ok(())
                }
                ResourceReferenceType::None => Err(anyhow::anyhow!(
                        "Entity has a resource reference of None, which cannot be loaded or referenced"
                    )),
            };

            result?;
            log::debug!("Loaded!");
        }

        let total = self.lights.len();
        
        for (index, light_config) in self.lights.iter().enumerate() {
            log::debug!("Loading light: {}", light_config.label);
            if let Some(ref s) = progress_sender {
                let _ = s.send(WorldLoadingStatus::LoadingLight { index, name: light_config.label.clone(), total });
            }

            let light = Light::new(
                graphics.clone(),
                light_config.light_component.clone(),
                light_config.transform.clone(),
                Some(&light_config.label),
            )
            .await;
            {
                world.spawn((
                    light_config.light_component.clone(),
                    light_config.transform,
                    light,
                    ModelProperties::default(),
                ));
            }
        }

        let total = self.cameras.len();
        for (index, camera_config) in self.cameras.iter().enumerate() {
            log::debug!(
                "Loading camera {} of type {:?}",
                camera_config.label,
                camera_config.camera_type
            );
            if let Some(ref s) = progress_sender {
                let _ = s.send(WorldLoadingStatus::LoadingCamera { index, name: camera_config.label.clone(), total });
            }

            let camera = Camera::new(
                graphics.clone(),
                CameraBuilder {
                    eye: DVec3::from_array(camera_config.position),
                    target: DVec3::from_array(camera_config.target),
                    up: DVec3::from_array(camera_config.up),
                    aspect: camera_config.aspect,
                    fov_y: camera_config.fov as f64,
                    znear: camera_config.near as f64,
                    zfar: camera_config.far as f64,
                    speed: camera_config.speed as f64,
                    sensitivity: camera_config.sensitivity as f64,
                },
                Some(&camera_config.label),
            );

            let component = CameraComponent {
                speed: camera_config.speed as f64,
                sensitivity: camera_config.sensitivity as f64,
                fov_y: camera_config.fov as f64,
                camera_type: camera_config.camera_type,
            };

            if let (Some(target_label), Some(offset)) = (
                &camera_config.follow_target_entity_label,
                &camera_config.follow_offset,
            ) {
                let follow_target = CameraFollowTarget {
                    follow_target: target_label.clone(),
                    offset: DVec3::from_array(*offset),
                };
                { world.spawn((camera, component, follow_target)); }
            } else {
                { world.spawn((camera, component)); }
            }
        }

        {
            let mut is_none = false;
            if world
                .query::<(&LightComponent, &Light)>()
                .iter()
                .next()
                .is_none()
            {
                log::info!("No lights in scene, spawning default light");
                is_none = true;
            }
            
            if is_none {
                if let Some(ref s) = progress_sender {
                    let _ = s.send(WorldLoadingStatus::LoadingLight { index: 0, name: String::from("Default Light"), total: 1 });
                }
                let comp = LightComponent::directional(glam::DVec3::ONE, 1.0);
                let trans = Transform {
                    position: glam::DVec3::new(2.0, 4.0, 2.0),
                    ..Default::default()
                };
                let light = Light::new(graphics.clone(), comp.clone(), trans, Some("Default Light")).await;

                { world.spawn((comp, trans, light, ModelProperties::default())); }
            }
        }

        log::info!(
            "Loaded {} entities, {} lights and {} cameras",
            self.entities.len(),
            self.lights.len(),
            self.cameras.len()
        );
        #[cfg(feature = "editor")]
        {
            // Editor mode - look for debug camera, create one if none exists
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
                        let _ = s.send(WorldLoadingStatus::LoadingCamera { index: 0, name: String::from("Viewport Camera"), total: 1 });
                    }
                    let camera = Camera::predetermined(graphics.clone(), Some("Viewport Camera"));
                    let component = DebugCamera::new();
                    let camera_entity = { world.spawn((camera, component)) };
                    Ok(camera_entity)
                }
            }
        }

        #[cfg(not(feature = "editor"))]
        {
            // Runtime mode - look for player camera, panic if none exists
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

#[derive(bincode::Decode, bincode::Encode, serde::Serialize, serde::Deserialize, Debug)]
pub struct RuntimeData {
    #[bincode(with_serde)]
    pub project_config: ProjectConfig,
    #[bincode(with_serde)]
    pub source_config: SourceConfig,
    #[bincode(with_serde)]
    pub scene_data: Vec<SceneConfig>,
    #[bincode(with_serde)]
    pub scripts: HashMap<String, String>, // name, script_content
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LightConfig {
    pub label: String,
    pub transform: Transform,
    pub light_component: LightComponent,
    pub enabled: bool,

    #[serde(skip)]
    pub entity_id: Option<hecs::Entity>,
}

impl Default for LightConfig {
    fn default() -> Self {
        Self {
            label: "New Light".to_string(),
            transform: Transform::default(),
            light_component: LightComponent::default(),
            enabled: true,
            entity_id: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct EditorSettings {
    pub is_debug_menu_shown: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub enum EditorTab {
    AssetViewer,       // bottom side,
    ResourceInspector, // left side,
    ModelEntityList,   // right side,
    Viewport,          // middle,
}

/// An enum that describes the status of loading the world.
///
/// This is enum is used by [`SceneConfig::load_into_world`] heavily. This enum
/// is recommended to be used with an [`UnboundedSender`]
pub enum WorldLoadingStatus {
    Idle,
    LoadingEntity {
        index: usize,
        name: String,
        total: usize,
    },
    LoadingLight {
        index: usize,
        name: String,
        total: usize,
    },
    LoadingCamera {
        index: usize,
        name: String,
        total: usize,
    },
    Completed,
}