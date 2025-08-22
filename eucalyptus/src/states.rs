//! In this module, it will describe all the different types for
//! storing configuration files (.eucp for project and .eucc for config files for subdirectories).
//!
//! There is a singleton that is used for other crates to access,
//! as well as public structs related to that config and docs (hopefully).

use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    fs,
    path::PathBuf,
    sync::RwLock,
};

use bincode::{Encode, Decode};
use chrono::Utc;
use dropbear_engine::{
    camera::Camera,
    entity::{AdoptedEntity, Transform},
    graphics::Graphics, lighting::{Light, LightComponent},
};

#[cfg(feature = "editor")]
use egui_dock_fork::DockState;

use hecs;
use log;
use once_cell::sync::Lazy;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use crate::camera::CameraType;
#[cfg(feature = "editor")]
use crate::editor::EditorTab;

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
#[cfg(feature = "editor")]
pub struct ProjectConfig {
    pub project_name: String,
    pub project_path: PathBuf,
    pub date_created: String,
    pub date_last_accessed: String,
    #[serde(default)]
    pub dock_layout: Option<DockState<EditorTab>>,
}

#[derive(Debug, Deserialize, Serialize, Default)]
#[cfg(not(feature = "editor"))]
pub struct ProjectConfig {
    pub project_name: String,
    pub project_path: PathBuf,
    pub date_created: String,
    pub date_last_accessed: String,
    // #[serde(default)]
    // pub dock_layout: Option<DockState<EditorTab>>,
}

impl ProjectConfig {
    /// Creates a new instance of the ProjectConfig. This function is typically used when creating
    /// a new project, with it creating new defaults for everything.
    pub fn new(project_name: String, project_path: &PathBuf) -> Self {
        let date_created = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        let date_last_accessed = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        #[cfg(not(feature = "editor"))]
        {
            let mut result = Self {
                project_name,
                project_path: project_path.to_path_buf(),
                date_created,
                date_last_accessed,
            };
            let _ = result.load_config_to_memory(); // TODO: Deal with later...
            result
        }

        #[cfg(feature = "editor")]
        {
            let mut result = Self {
                project_name,
                project_path: project_path.to_path_buf(),
                date_created,
                date_last_accessed,
                dock_layout: None,
            };
            let _ = result.load_config_to_memory(); // TODO: Deal with later...
            result
        }
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
                let mut cfg = RESOURCES.write().unwrap();
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
                            let mut cfg = RESOURCES.write().unwrap();
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
        let mut source_config = SOURCE.write().unwrap();
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
        let mut scene_configs = SCENES.write().unwrap();
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
            let resources_config = RESOURCES.read().unwrap();
            resources_config.write_to(&path)?;
        }

        {
            let source_config = SOURCE.read().unwrap();
            source_config.write_to(&path)?;
        }

        {
            let scene_configs = SCENES.read().unwrap();
            for scene in scene_configs.iter() {
                scene.write_to(&path)?;
            }
        }

        self.write_to(&path)?;
        Ok(())
    }
}

#[allow(dead_code)]
pub(crate) fn path_contains_folder(path: &PathBuf, folder: &str) -> bool {
    path.components().any(|comp| comp.as_os_str() == folder)
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

/// This is the resource config.
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
    Group {
        name: String,
        children: Vec<EntityNode>,
        collapsed: bool,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

            // grouped entity (entity + script)
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

        nodes
    }
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct SceneConfig {
    pub scene_name: String,
    pub entities: Vec<SceneEntity>,
    pub camera: HashMap<CameraType, SceneCameraConfig>,
    pub lights: Vec<LightConfig>,
    // todo later
    // pub settings: SceneSettings,
    #[serde(skip)]
    pub path: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SceneCameraConfig {
    pub position: [f64; 3],
    pub target: [f64; 3],
    pub up: [f64; 3],
    pub aspect: f64,
    pub fov: f32,
    pub near: f32,
    pub far: f32,

    pub follow_target_entity_label: Option<String>,
    pub follow_offset: Option<[f64; 3]>,
}

impl Default for SceneCameraConfig {
    fn default() -> Self {
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
        }
    }
}

impl SceneCameraConfig {
    pub fn into_camera(&self, graphics: &mut Graphics) -> Camera {
        Camera::new(graphics, self.position.into(), self.target.into(), self.up.into(), self.aspect.into(), self.fov.into(), self.near.into(), self.far.into(), 5.0, 0.0125)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SceneEntity {
    pub model_path: PathBuf,
    pub label: String,
    pub transform: Transform,
    pub properties: ModelProperties,
    pub script: Option<ScriptComponent>,

    #[serde(skip)]
    #[allow(dead_code)]
    pub entity_id: Option<hecs::Entity>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelProperties {
    pub custom_properties: HashMap<String, PropertyValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyValue {
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

    pub fn set_property(&mut self, key: String, value: PropertyValue) {
        self.custom_properties.insert(key, value);
    }

    pub fn get_property(&self, key: &str) -> Option<&PropertyValue> {
        self.custom_properties.get(key)
    }
}

impl Default for ModelProperties {
    fn default() -> Self {
        Self::new()
    }
}

impl SceneConfig {
    /// Creates a new instance of the scene config
    pub fn new(scene_name: String, path: PathBuf) -> Self {
        let mut camera_configs = HashMap::new();

        camera_configs.insert(
            CameraType::Debug,
            SceneCameraConfig {
                position: [0.0, 5.0, 10.0],
                target: [0.0, 0.0, 0.0],
                up: [0.0, 1.0, 0.0],
                aspect: 16.0 / 9.0,
                fov: 45.0,
                near: 0.1,
                far: 100.0,
                ..Default::default()
            },
        );

        camera_configs.insert(
            CameraType::Player,
            SceneCameraConfig {
                position: [0.0, 2.0, 5.0],
                target: [0.0, 0.0, 0.0],
                up: [0.0, 1.0, 0.0],
                aspect: 16.0 / 9.0,
                fov: 45.0,
                near: 0.1,
                far: 100.0,
                ..Default::default()
            },
        );

        Self {
            scene_name,
            path,
            entities: Vec::new(),
            camera: camera_configs,
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
    ) -> anyhow::Result<()> {
        log::info!(
            "Loading scene [{}], clearing world with {} entities",
            self.scene_name,
            world.len()
        );
        world.clear();

        log::info!("World cleared, now has {} entities", world.len());

        for entity_config in &self.entities {
            log::debug!("Loading entity: {}", entity_config.label);

            let adopted = AdoptedEntity::new(
                graphics,
                &entity_config.model_path,
                Some(&entity_config.label),
            )?;

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

        for light_config in &self.lights {
            log::debug!("Loading light: {}", light_config.label);

            let light = Light::new(
                graphics,
                &light_config.light_component,
                &light_config.transform,
                Some(&light_config.label),
            );

            world.spawn((light_config.light_component.clone(), light_config.transform, light, ModelProperties::default()));
        }

        if world.query::<(&LightComponent, &Light)>().iter().next().is_none() {
            log::info!("No lights in scene, spawning default light");
            let default_transform = Transform {
                position: glam::DVec3::new(2.0, 4.0, 2.0),
                ..Default::default()
            };
            let default_component = LightComponent::directional(glam::DVec3::ONE, 1.0);
            let default_light = Light::new(graphics, &default_component, &default_transform, Some("Default Light"));
            world.spawn((default_component, default_transform, default_light, ModelProperties::default()));
        }

        log::info!("Loaded {} entities and {} lights", self.entities.len(), self.lights.len());
        Ok(())
    }

    pub fn load_cameras_into_manager(
        &self,
        camera_manager: &mut crate::camera::CameraManager,
        graphics: &Graphics,
        world: &hecs::World,
    ) -> anyhow::Result<()> {
        use crate::camera::{DebugCameraController, PlayerCameraController};

        if let Some(debug_config) = self.camera.get(&CameraType::Debug) {
            let debug_camera = Camera::new(
                graphics,
                glam::DVec3::from_array(debug_config.position),
                glam::DVec3::from_array(debug_config.target),
                glam::DVec3::from_array(debug_config.up),
                debug_config.aspect,
                debug_config.fov as f64,
                debug_config.near as f64,
                debug_config.far as f64,
                0.125,
                0.002,
            );
            let debug_controller = Box::new(DebugCameraController::new());
            camera_manager.add_camera(CameraType::Debug, debug_camera, debug_controller);
        }

        if let Some(player_config) = self.camera.get(&CameraType::Player) {
            let player_camera = Camera::new(
                graphics,
                glam::DVec3::from_array(player_config.position),
                glam::DVec3::from_array(player_config.target),
                glam::DVec3::from_array(player_config.up),
                player_config.aspect,
                player_config.fov as f64,
                player_config.near as f64,
                player_config.far as f64,
                0.1,
                0.001,
            );
            let player_controller = Box::new(PlayerCameraController::new());
            camera_manager.add_camera(CameraType::Player, player_camera, player_controller);

            if let (Some(target_label), Some(offset_array)) = (
                &player_config.follow_target_entity_label,
                &player_config.follow_offset,
            ) {
                for (entity_id, adopted_entity) in world.query::<&AdoptedEntity>().iter() {
                    log::debug!(
                        "World entity {:?} -> label='{}' path='{}'",
                        entity_id,
                        adopted_entity.label(),
                        adopted_entity.model().path.display()
                    );
                }

                let target_entity = world
                    .query::<&AdoptedEntity>()
                    .iter()
                    .find_map(|(entity_id, adopted_entity)| {
                        if adopted_entity.label() == target_label {
                            Some(entity_id)
                        } else {
                            let stem_match = adopted_entity
                                .model()
                                .path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .map(|s| s == target_label)
                                .unwrap_or(false);
                            if stem_match {
                                Some(entity_id)
                            } else {
                                None
                            }
                        }
                    });

                if let Some(entity_id) = target_entity {
                    let offset = glam::DVec3::from_array(*offset_array);
                    camera_manager.set_player_camera_target(entity_id, offset);
                    log::info!(
                        "Restored player camera follow target: {} with offset {:?}",
                        target_label,
                        offset
                    );
                } else {
                    log::warn!(
                        "Could not find entity '{}' to restore camera follow target",
                        target_label
                    );
                }
            }
        }

        Ok(())
    }

    /// Save cameras from camera manager to scene config
    pub fn save_cameras_from_manager(
        &mut self,
        camera_manager: &crate::camera::CameraManager,
        world: &hecs::World,
    ) {
        self.camera.clear();

        if let Some(debug_camera) = camera_manager.get_camera(&CameraType::Debug) {
            self.camera.insert(
                CameraType::Debug,
                SceneCameraConfig {
                    position: [debug_camera.eye.x, debug_camera.eye.y, debug_camera.eye.z],
                    target: [
                        debug_camera.target.x,
                        debug_camera.target.y,
                        debug_camera.target.z,
                    ],
                    up: [debug_camera.up.x, debug_camera.up.y, debug_camera.up.z],
                    aspect: debug_camera.aspect,
                    fov: debug_camera.fov_y as f32,
                    near: debug_camera.znear as f32,
                    far: debug_camera.zfar as f32,
                    follow_target_entity_label: None,
                    follow_offset: None,
                },
            );
        }

        if let Some(player_camera) = camera_manager.get_camera(&CameraType::Player) {
            let (follow_entity_label, follow_offset) =
                if let Some(target_entity) = camera_manager.get_player_camera_target() {
                    let entity_label = world
                        .query_one::<&AdoptedEntity>(target_entity)
                        .ok()
                        .and_then(|mut entity_ref| {
                            entity_ref
                                .get()
                                .map(|adopted_entity| adopted_entity.label().clone())
                        });

                    let offset = camera_manager
                        .get_player_camera_offset()
                        .map(|offset| [offset.x, offset.y, offset.z]);

                    (entity_label, offset)
                } else {
                    (None, None)
                };

            self.camera.insert(
                CameraType::Player,
                SceneCameraConfig {
                    position: [
                        player_camera.eye.x,
                        player_camera.eye.y,
                        player_camera.eye.z,
                    ],
                    target: [
                        player_camera.target.x,
                        player_camera.target.y,
                        player_camera.target.z,
                    ],
                    up: [player_camera.up.x, player_camera.up.y, player_camera.up.z],
                    aspect: player_camera.aspect,
                    fov: player_camera.fov_y as f32,
                    near: player_camera.znear as f32,
                    far: player_camera.zfar as f32,
                    follow_target_entity_label: follow_entity_label,
                    follow_offset: follow_offset,
                },
            );
        }
    }
}

#[derive(Decode, Encode, serde::Serialize, serde::Deserialize, Debug)]
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