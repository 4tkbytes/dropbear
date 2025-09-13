use crate::camera::DebugCamera;
use crate::camera::{CameraComponent, CameraFollowTarget, CameraType};
use crate::model_ext::PendingModel;
use crate::utils::PROTO_TEXTURE;
use chrono::Utc;
use dropbear_engine::camera::Camera;
use dropbear_engine::entity::{AdoptedEntity, Transform};
use dropbear_engine::graphics::Graphics;
use dropbear_engine::lighting::{Light, LightComponent};
use dropbear_engine::starter::plane::PlaneBuilder;
use dropbear_engine::utils::{ResourceReference, ResourceReferenceType};
use egui_dock_fork::DockState;
use glam::DVec3;
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;
use std::{fmt, fs};

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
    pub fn new(project_name: String, project_path: &PathBuf) -> Self {
        let date_created = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        let date_last_accessed = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        // #[cfg(not(feature = "editor"))]
        // {
        //     let mut result = Self {
        //         project_name,
        //         project_path: project_path.to_path_buf(),
        //         date_created,
        //         date_last_accessed,
        //     };
        //     let _ = result.load_config_to_memory(); // TODO: Deal with later...
        //     result
        // }

        let mut result = Self {
            project_name,
            project_path: project_path.to_path_buf(),
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

    pub fn write_to(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        self.load_config_to_memory()?;
        self.date_last_accessed = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        // self.assets = Assets::walk(path);
        let ron_str = ron::ser::to_string_pretty(&self, PrettyConfig::default())
            .map_err(|e| anyhow::anyhow!("RON serialization error: {}", e))?;
        let config_path = path.join(format!("{}.eucp", self.project_name.clone().to_lowercase()));
        self.project_path = path.to_path_buf();

        fs::write(&config_path, ron_str).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(())
    }

    /// This function reads from the RON and traverses down the different folders to add more information
    /// to the ProjectConfig, such as Assets location and other stuff.
    ///
    /// # Parameters
    /// * path - The root config **file** for the project
    pub fn read_from(path: &PathBuf) -> anyhow::Result<Self> {
        let ron_str = fs::read_to_string(path)?;
        let mut config: ProjectConfig = ron::de::from_str(&ron_str.as_str())?;
        config.project_path = path.parent().unwrap().to_path_buf();
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
        let path = PathBuf::from(self.project_path.clone());

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
    pub fn write_to(&self, path: &PathBuf) -> anyhow::Result<()> {
        let resource_dir = path.join("resources");
        let updated_config = ResourceConfig {
            path: resource_dir.clone(),
            nodes: collect_nodes(&resource_dir, path, vec!["thumbnails"].as_slice()),
        };
        let ron_str = ron::ser::to_string_pretty(&updated_config, PrettyConfig::default())
            .map_err(|e| anyhow::anyhow!("RON serialization error: {}", e))?;
        let config_path = path.join("resources").join("resources.eucc");
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
    pub fn read_from(path: &PathBuf) -> anyhow::Result<Self> {
        let config_path = path.join("resources").join("resources.eucc");
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
    pub fn write_to(&self, path: &PathBuf) -> anyhow::Result<()> {
        let resource_dir = path.join("src");
        let updated_config = SourceConfig {
            path: resource_dir.clone(),
            nodes: collect_nodes(&resource_dir, path, vec!["scripts"].as_slice()),
        };

        let ron_str = ron::ser::to_string_pretty(&updated_config, PrettyConfig::default())
            .map_err(|e| anyhow::anyhow!("RON serialisation error: {}", e))?;
        let config_path = path.join("src").join("source.eucc");
        fs::create_dir_all(config_path.parent().unwrap())?;
        fs::write(&config_path, ron_str).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(())
    }

    /// # Parameters
    /// - path: The location to the **source.eucc** file
    pub fn read_from(path: &PathBuf) -> anyhow::Result<Self> {
        let config_path = path.join("src").join("source.eucc");
        let ron_str = fs::read_to_string(&config_path)?;
        let config: SourceConfig = ron::de::from_str(&ron_str)
            .map_err(|e| anyhow::anyhow!("RON deserialization error: {}", e))?;
        Ok(config)
    }
}

fn collect_nodes(dir: &PathBuf, project_path: &PathBuf, exclude_list: &[&str]) -> Vec<Node> {
    let mut nodes = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            let name = entry_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            if entry_path.is_dir() && exclude_list.iter().any(|ex| &ex.to_string() == &name) {
                log::debug!("Skipped past folder {:?}", name);
                continue;
            }

            if entry_path.is_dir() {
                let folder_nodes = collect_nodes(&entry_path, project_path, exclude_list);
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

                nodes.push(Node::File(File {
                    name,
                    path: entry_path.clone(),
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
            let name = adopted.model().label.clone();

            // grouped entity (entity and script)
            nodes.push(EntityNode::Group {
                name: name.clone(),
                children: vec![
                    EntityNode::Entity {
                        id,
                        name: name.clone(),
                    },
                    EntityNode::Script {
                        name: script.name.clone(),
                        path: script.path.clone(),
                    },
                ],
                collapsed: false,
            });
            handled.insert(id);
        }

        // single entity
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
            let name = adopted.model().label.clone();

            nodes.push(EntityNode::Entity { id, name });
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
            follow_target_entity_label: if let Some(target) = follow_target {
                Some(target.follow_target.clone())
            } else {
                None
            },
            follow_offset: if let Some(target) = follow_target {
                Some(target.offset.to_array())
            } else {
                None
            },
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
    pub fn new(scene_name: String, path: PathBuf) -> Self {
        Self {
            scene_name,
            path,
            entities: Vec::new(),
            cameras: Vec::new(),
            lights: Vec::new(),
        }
    }

    /// Write the scene config to a .eucs file
    pub fn write_to(&self, project_path: &PathBuf) -> anyhow::Result<()> {
        let ron_str = ron::ser::to_string_pretty(&self, PrettyConfig::default())
            .map_err(|e| anyhow::anyhow!("RON serialization error: {}", e))?;

        let scenes_dir = project_path.join("scenes");
        fs::create_dir_all(&scenes_dir)?;

        let config_path = scenes_dir.join(format!("{}.eucs", self.scene_name));
        fs::write(&config_path, ron_str).map_err(|e| anyhow::anyhow!(e.to_string()))?;
        Ok(())
    }

    /// Read a scene config from a .eucs file
    pub fn read_from(scene_path: &PathBuf) -> anyhow::Result<Self> {
        let ron_str = fs::read_to_string(scene_path)?;
        let mut config: SceneConfig = ron::de::from_str(&ron_str)
            .map_err(|e| anyhow::anyhow!("RON deserialization error: {}", e))?;

        config.path = scene_path.clone();
        Ok(config)
    }

    pub fn load_into_world(
        &self,
        world: &mut hecs::World,
        graphics: &Graphics,
    ) -> anyhow::Result<hecs::Entity> {
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

        let mut model_handles = Vec::new();
        let mut pending_entities = Vec::new();

        for entity_config in &self.entities {
            log::debug!("Loading entity: {}", entity_config.label);
            match &entity_config.model_path.ref_type {
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

                    let pending_model = PendingModel {
                        path: Some(path),
                        bytes: None,
                        label: entity_config.label.clone(),
                        model_type: crate::model_ext::ModelLoadType::File,
                    };

                    let handle = crate::model_ext::GLOBAL_MODEL_LOADER.push(Box::new(pending_model));
                    model_handles.push((handle.id, entity_config.clone()));
                    pending_entities.push(entity_config.clone());
                }
                ResourceReferenceType::Bytes(bytes) => {
                    log::info!("Queuing entity from bytes [Len: {}]", bytes.len());
                    
                    let pending_model = PendingModel {
                        path: None,
                        bytes: Some(bytes.to_owned()),
                        label: entity_config.label.clone(),
                        model_type: crate::model_ext::ModelLoadType::Memory,
                    };

                    let handle = crate::model_ext::GLOBAL_MODEL_LOADER.push(Box::new(pending_model));
                    model_handles.push((handle.id, entity_config.clone()));
                    pending_entities.push(entity_config.clone());
                }
                ResourceReferenceType::Plane => {
                    // this can be loaded in immediately as it doesn't use IO to load
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

                    let plane = PlaneBuilder::new()
                        .with_size(*width as f32, *height as f32)
                        .with_tiles(*tiles_x as u32, *tiles_z as u32)
                        .build(
                            graphics,
                            PROTO_TEXTURE,
                            Some(entity_config.label.clone().as_str()),
                        )?;
                    let transform = entity_config.transform;

                    if let Some(script_config) = &entity_config.script {
                        let script = ScriptComponent {
                            name: script_config.name.clone(),
                            path: script_config.path.clone(),
                        };
                        world.spawn((plane, transform, script, entity_config.properties.clone()));
                    } else {
                        world.spawn((plane, transform, entity_config.properties.clone()));
                    }
                }
                ResourceReferenceType::None => panic!(
                    "Entity has a resource reference of None, which cannot be loaded or referenced"
                ),
            }
        }

        log::info!("Processing {} models in parallel", model_handles.len());
        crate::model_ext::GLOBAL_MODEL_LOADER.process(graphics);

        for (handle_id, entity_config) in model_handles {
            match crate::model_ext::GLOBAL_MODEL_LOADER.get_status(handle_id) {
                Some(crate::model_ext::ModelLoadingStatus::Loaded) => {
                    log::debug!("Model loaded successfully: {}", entity_config.label);
                    
                    let model = crate::model_ext::GLOBAL_MODEL_LOADER.exchange_by_id(handle_id)?;
                    let adopted = AdoptedEntity::adopt(graphics, model, Some(&entity_config.label));
                    
                    self.spawn_entity_with_components(world, &entity_config, adopted);
                }
                Some(crate::model_ext::ModelLoadingStatus::Failed(error)) => {
                    log::error!("Failed to load model {}: {}", entity_config.label, error);
                    return Err(anyhow::anyhow!("Failed to load model {}: {}", entity_config.label, error));
                }
                _ => {
                    log::error!("Model loading status unknown for: {}", entity_config.label);
                    return Err(anyhow::anyhow!("Model loading failed for: {}", entity_config.label));
                }
            }
        }

        crate::model_ext::GLOBAL_MODEL_LOADER.clear_completed();

        for light_config in &self.lights {
            log::debug!("Loading light: {}", light_config.label);

            let light = Light::new(
                graphics,
                &light_config.light_component,
                &light_config.transform,
                Some(&light_config.label),
            );

            world.spawn((
                light_config.light_component.clone(),
                light_config.transform,
                light,
                ModelProperties::default(),
            ));
        }

        for camera_config in &self.cameras {
            log::debug!(
                "Loading camera {} of type {:?}",
                camera_config.label,
                camera_config.camera_type
            );

            let camera = Camera::new(
                graphics,
                DVec3::from_array(camera_config.position),
                DVec3::from_array(camera_config.target),
                DVec3::from_array(camera_config.up),
                camera_config.aspect,
                camera_config.fov as f64,
                camera_config.near as f64,
                camera_config.far as f64,
                camera_config.speed as f64,
                camera_config.sensitivity as f64,
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
                world.spawn((camera, component, follow_target));
            } else {
                world.spawn((camera, component));
            }
        }

        if world
            .query::<(&LightComponent, &Light)>()
            .iter()
            .next()
            .is_none()
        {
            log::info!("No lights in scene, spawning default light");
            let default_transform = Transform {
                position: glam::DVec3::new(2.0, 4.0, 2.0),
                ..Default::default()
            };
            let default_component = LightComponent::directional(glam::DVec3::ONE, 1.0);
            let default_light = Light::new(
                graphics,
                &default_component,
                &default_transform,
                Some("Default Light"),
            );
            world.spawn((
                default_component,
                default_transform,
                default_light,
                ModelProperties::default(),
            ));
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
            let debug_camera = world
                .query::<(&Camera, &CameraComponent)>()
                .iter()
                .find_map(|(entity, (_, component))| {
                    if matches!(component.camera_type, CameraType::Debug) {
                        Some(entity)
                    } else {
                        None
                    }
                });

            if let Some(camera_entity) = debug_camera {
                log::info!("Using existing debug camera for editor");
                Ok(camera_entity)
            } else {
                log::info!("No debug camera found, creating viewport camera for editor");
                let camera = Camera::predetermined(graphics, Some("Viewport Camera"));
                let component = DebugCamera::new();
                let camera_entity = world.spawn((camera, component));
                Ok(camera_entity)
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

    fn spawn_entity_with_components(
        &self,
        world: &mut hecs::World,
        entity_config: &SceneEntity,
        adopted: AdoptedEntity,
    ) {
        let transform = entity_config.transform;

        if let Some(script_config) = &entity_config.script {
            let script = ScriptComponent {
                name: script_config.name.clone(),
                path: script_config.path.clone(),
            };
            world.spawn((adopted, transform, script, entity_config.properties.clone()));
        } else {
            world.spawn((adopted, transform, entity_config.properties.clone()));
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
