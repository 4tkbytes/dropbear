use crate::traits::SerializableComponent;
use crate::camera::{CameraComponent, CameraType};
use crate::hierarchy::{Parent, SceneHierarchy};
use crate::utils::{ResolveReference, PROTO_TEXTURE};
use chrono::Utc;
use dropbear_engine::asset::ASSET_REGISTRY;
use dropbear_engine::camera::{Camera, CameraBuilder, CameraSettings};
use dropbear_engine::entity::{MaterialOverride, MeshRenderer, Transform};
use dropbear_engine::graphics::SharedGraphicsContext;
use dropbear_engine::lighting::{Light, LightComponent};
use dropbear_engine::model::Model;
use dropbear_engine::procedural::plane::PlaneBuilder;
use dropbear_engine::utils::{ResourceReference, ResourceReferenceType};
use egui::Ui;
use egui_dock::DockState;
use glam::{DQuat, DVec3};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use rayon::prelude::*;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::{fmt, fs};
use tokio::sync::mpsc::UnboundedSender;
use dropbear_macro::SerializableComponent;
use crate::scene::SceneConfig;

pub static PROJECT: Lazy<RwLock<ProjectConfig>> =
    Lazy::new(|| RwLock::new(ProjectConfig::default()));

pub static RESOURCES: Lazy<RwLock<ResourceConfig>> =
    Lazy::new(|| RwLock::new(ResourceConfig::default()));

pub static SOURCE: Lazy<RwLock<SourceConfig>> = Lazy::new(|| RwLock::new(SourceConfig::default()));

pub static SCENES: Lazy<RwLock<Vec<SceneConfig>>> = Lazy::new(|| RwLock::new(Vec::new()));

/// Removes a scene with the provided name from the in-memory scene cache.
/// Returns `true` when a scene was removed and `false` when no matching scene existed.
pub fn unload_scene(scene_name: &str) -> bool {
    let mut scenes = SCENES.write();
    let initial_len = scenes.len();
    scenes.retain(|scene| scene.scene_name != scene_name);
    let removed = scenes.len() != initial_len;

    if removed {
        log::info!("Unloaded scene '{}' from memory", scene_name);
    } else {
        log::debug!("Scene '{}' was not loaded; nothing to unload", scene_name);
    }

    removed
}

/// Reads a scene configuration from disk based on the active project's path.
pub fn load_scene(scene_name: &str) -> anyhow::Result<SceneConfig> {
    let scene_path = {
        let project = PROJECT.read();
        if project.project_path.as_os_str().is_empty() {
            return Err(anyhow::anyhow!(
                "Project path is not set; cannot load scenes"
            ));
        }

        project
            .project_path
            .join("scenes")
            .join(format!("{}.eucs", scene_name))
    };

    let scene = SceneConfig::read_from(&scene_path)?;
    log::info!(
        "Loaded scene '{}' from {}",
        scene_name,
        scene_path.display()
    );
    Ok(scene)
}

/// Reloads a scene into the in-memory cache by unloading any existing copy first.
pub fn load_scene_into_memory(scene_name: &str) -> anyhow::Result<()> {
    unload_scene(scene_name);

    let scene = load_scene(scene_name)?;
    {
        let mut scenes = SCENES.write();
        scenes.insert(0, scene);
    }

    log::info!("Scene '{}' loaded into memory", scene_name);

    Ok(())
}

/// The root config file, responsible for building and other metadata.
///
/// # Location
/// This file is {project_name}.eucp and is located at {project_dir}/{project_name}.eucp
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct ProjectConfig {
    pub project_name: String,
    pub project_path: PathBuf,
    pub date_created: String,
    pub date_last_accessed: String,
    #[serde(default)]
    pub dock_layout: Option<DockState<EditorTab>>,
    #[serde(default)]
    pub editor_settings: EditorSettings,
    #[serde(default)]
    pub last_opened_scene: Option<String>,
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
            last_opened_scene: None,
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
        let config_path = path
            .as_ref()
            .join(format!("{}.eucp", self.project_name.clone().to_lowercase()));
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
                                panic!("Failed to load scene file {:?}: {}", path, e);
                            }
                        } else {
                            panic!("Failed to load scene file {:?}: {}", path, e);
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
            self.last_opened_scene = Some(default_scene.scene_name.clone());
            scene_configs.push(default_scene);
        }

        if let Some(ref last_scene_name) = self.last_opened_scene {
            if let Some(pos) = scene_configs
                .iter()
                .position(|scene| &scene.scene_name == last_scene_name)
            {
                if pos != 0 {
                    let scene = scene_configs.remove(pos);
                    scene_configs.insert(0, scene);
                }
            } else if let Some(first) = scene_configs.first() {
                self.last_opened_scene = Some(first.scene_name.clone());
            }
        } else if let Some(first) = scene_configs.first() {
            self.last_opened_scene = Some(first.scene_name.clone());
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
pub enum File {
    #[default]
    Unknown,
    ResourceFile {
        name: String,
        path: PathBuf,
        resource_type: ResourceType,
    },
    SourceFile {
        name: String,
        path: PathBuf,
    },
}

// #[derive(Default, Debug, Serialize, Deserialize, Clone)]
// pub struct File {
//     pub name: String,
//     pub path: PathBuf,
//     pub resource_type: Option<ResourceType>,
// }

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
    Config,
    Script,
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
            ResourceType::Script => "script",
            ResourceType::Config => "eucalyptus project config",
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

fn collect_nodes(
    dir: impl AsRef<Path>,
    project_path: impl AsRef<Path>,
    exclude_list: &[&str],
) -> Vec<Node> {
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
                    ResourceType::Model
                } else if parent_folder.contains("texture") {
                    ResourceType::Texture
                } else if parent_folder.contains("shader") {
                    ResourceType::Shader
                } else if entry_path
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    == Some("kt".to_string())
                {
                    ResourceType::Script
                } else if entry_path
                    .extension()
                    .map(|e| e.to_string_lossy().to_lowercase().contains("eu"))
                    .unwrap_or_default()
                {
                    ResourceType::Config
                } else {
                    ResourceType::Unknown
                };

                // Store relative path from the project root instead of absolute path
                let relative_path = entry_path
                    .strip_prefix(project_path.as_ref())
                    .unwrap_or(&entry_path)
                    .to_path_buf();

                nodes.push(Node::File(File::ResourceFile {
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
        tags: Vec<String>,
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

#[derive(Default, Debug, Serialize, Deserialize, Clone, SerializableComponent)]
pub struct ScriptComponent {
    pub tags: Vec<String>,
}

impl EntityNode {
    pub fn from_world(world: &hecs::World) -> Vec<Self> {
        let mut nodes = Vec::new();
        let mut handled = std::collections::HashSet::new();

        for (id, (label, script, _transform, _renderer)) in world
            .query::<(
                &Label,
                &ScriptComponent,
                &dropbear_engine::entity::Transform,
                &dropbear_engine::entity::MeshRenderer,
            )>()
            .iter()
        {
            let name = label.to_string();
            let mut children = vec![
                EntityNode::Entity {
                    id,
                    name: name.clone(),
                },
                EntityNode::Script {
                    tags: script.tags.clone(),
                },
            ];

            // Check if this entity also has camera components
            if let Ok(mut camera_query) = world.query_one::<(&Camera, &CameraComponent)>(id)
                && let Some((camera, component)) = camera_query.get()
            {
                children.push(EntityNode::Camera {
                    id,
                    name: camera.label.clone(),
                    camera_type: component.camera_type,
                });
            }

            nodes.push(EntityNode::Group {
                name: name.clone(),
                children,
                collapsed: false,
            });
            handled.insert(id);
        }

        // Handle single entities (and potentially cameras)
        for (id, (label, _renderer)) in world
            .query::<(&Label, &dropbear_engine::entity::MeshRenderer)>()
            .iter()
        {
            if handled.contains(&id) {
                continue;
            }
            let name = label.to_string();

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

        // Handle standalone cameras (cameras without MeshRenderer - like viewport cameras)
        for (entity, (camera, component)) in world.query::<(&Camera, &CameraComponent)>().iter() {
            if world.get::<&MeshRenderer>(entity).is_err() {
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

#[derive(Debug, Serialize, Deserialize, Clone, SerializableComponent)]
pub struct CameraConfig {
    pub label: String,
    pub camera_type: CameraType,

    pub eye: [f64; 3],
    pub target: [f64; 3],
    pub up: [f64; 3],
    pub aspect: f64,
    pub fov: f32,
    pub near: f32,
    pub far: f32,

    pub speed: f32,
    pub sensitivity: f32,

    pub starting_camera: bool,
}

impl Default for CameraConfig {
    fn default() -> Self {
        let default = CameraComponent::new();
        Self {
            eye: [0.0, 1.0, 2.0],
            target: [0.0, 0.0, 0.0],
            up: [0.0, 1.0, 0.0],
            aspect: 16.0 / 9.0,
            fov: 45.0,
            near: 0.1,
            far: 100.0,
            label: String::new(),
            camera_type: CameraType::Normal,
            speed: default.settings.speed as f32,
            sensitivity: default.settings.sensitivity as f32,
            starting_camera: false,
        }
    }
}

impl CameraConfig {
    pub fn from_ecs_camera(
        camera: &Camera,
        component: &CameraComponent,
        // follow_target: Option<&CameraFollowTarget>,
    ) -> Self {
        Self {
            eye: camera.position().to_array(),
            target: camera.target.to_array(),
            label: camera.label.clone(),
            camera_type: component.camera_type,
            up: camera.up.to_array(),
            aspect: camera.aspect,
            fov: camera.settings.fov_y as f32,
            near: camera.znear as f32,
            far: camera.zfar as f32,
            speed: component.settings.speed as f32,
            sensitivity: component.settings.sensitivity as f32,
            starting_camera: component.starting_camera,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, SerializableComponent)]
pub struct ModelProperties {
    pub custom_properties: Vec<Property>,
    pub next_id: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Property {
    pub id: u64,
    pub key: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Value {
    String(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Vec3([f32; 3]),
}

impl Default for Value {
    fn default() -> Self {
        Self::String(String::new())
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let string: String = match self {
            Value::String(_) => "String".into(),
            Value::Int(_) => "Int".into(),
            Value::Float(_) => "Float".into(),
            Value::Bool(_) => "Bool".into(),
            Value::Vec3(_) => "Vec3".into(),
        };
        write!(f, "{}", string)
    }
}

impl ModelProperties {
    pub fn new() -> Self {
        Self {
            custom_properties: Vec::new(),
            next_id: 0,
        }
    }

    pub fn set_property(&mut self, key: String, value: Value) {
        if let Some(prop) = self.custom_properties.iter_mut().find(|p| p.key == key) {
            prop.value = value;
        } else {
            self.custom_properties.push(Property {
                id: self.next_id,
                key,
                value,
            });
            self.next_id += 1;
        }
    }

    pub fn get_property(&self, key: &str) -> Option<&Value> {
        self.custom_properties
            .iter()
            .find(|p| p.key == key)
            .map(|p| &p.value)
    }

    pub fn get_float(&self, key: &str) -> Option<f64> {
        match self.get_property(key)? {
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self.get_property(key)? {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    pub fn add_property(&mut self, key: String, value: Value) {
        self.custom_properties.push(Property {
            id: self.next_id,
            key,
            value,
        });
        self.next_id += 1;
    }

    pub fn show_value_editor(ui: &mut Ui, value: &mut Value) -> bool {
        match value {
            Value::String(s) => ui.text_edit_singleline(s).changed(),
            Value::Int(i) => ui
                .add(egui::Slider::new(i, -1000..=1000).text(""))
                .changed(),
            Value::Float(f) => ui
                .add(egui::Slider::new(f, -100.0..=100.0).text(""))
                .changed(),
            Value::Bool(b) => ui.checkbox(b, "").changed(),
            Value::Vec3(vec) => {
                let mut changed = false;
                ui.horizontal(|ui| {
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut vec[0], -10.0..=10.0)
                                .text("X")
                                .fixed_decimals(2),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut vec[1], -10.0..=10.0)
                                .text("Y")
                                .fixed_decimals(2),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut vec[2], -10.0..=10.0)
                                .text("Z")
                                .fixed_decimals(2),
                        )
                        .changed();
                });
                changed
            }
        }
    }
}

impl Default for ModelProperties {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, SerializableComponent)]
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
    ErrorConsole,
    Plugin(usize),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PluginInfo {
    pub display_name: String,
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
    Completed,
}

#[derive(Clone, Debug, Serialize, Deserialize, bincode::Encode, bincode::Decode)]
pub struct RuntimeData {
    #[bincode(with_serde)]
    pub project_config: ProjectConfig,
    #[bincode(with_serde)]
    pub source_config: SourceConfig,
    #[bincode(with_serde)]
    pub scene_data: Vec<SceneConfig>,
    #[bincode(with_serde)]
    pub scripts: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone, SerializableComponent)]
pub struct Label(String);

impl Default for Label {
    fn default() -> Self {
        Self(String::from("No Label"))
    }
}

impl Label {
    /// Creates a new label component from any type that can be converted into a [`String`].
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the underlying string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns a mutable reference to the underlying [`String`].
    pub fn as_mut_string(&mut self) -> &mut String {
        &mut self.0
    }

    /// Replaces the underlying value with the provided one.
    pub fn set(&mut self, value: impl Into<String>) {
        self.0 = value.into();
    }

    /// Consumes the label and returns the owned [`String`].
    pub fn into_inner(self) -> String {
        self.0
    }

    /// Returns whether the underlying label is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Display for Label {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Label {
    fn from(value: String) -> Self {
        Label::new(value)
    }
}

impl From<&str> for Label {
    fn from(value: &str) -> Self {
        Label::new(value)
    }
}

impl AsRef<str> for Label {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl Borrow<str> for Label {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl Deref for Label {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl DerefMut for Label {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_string()
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, SerializableComponent)]
pub struct SerializedMeshRenderer {
    pub handle: ResourceReference,
    pub material_override: Vec<MaterialOverride>,
}