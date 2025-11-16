use crate::camera::{CameraComponent, CameraType};
use crate::hierarchy::Parent;
use crate::utils::ResolveReference;
use chrono::Utc;
use dropbear_engine::asset::ASSET_REGISTRY;
use dropbear_engine::camera::{Camera, CameraBuilder, CameraSettings};
use dropbear_engine::entity::{LocalTransform, MaterialOverride, MeshRenderer, Transform, WorldTransform};
use dropbear_engine::graphics::SharedGraphicsContext;
use dropbear_engine::lighting::{Light, LightComponent};
use dropbear_engine::model::Model;
use dropbear_engine::utils::ResourceReference;
use egui::Ui;
use egui_dock::DockState;
use glam::{DQuat, DVec3};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
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
use dropbear_derive::Component;
use dropbear_traits::Component;

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

#[derive(Default, Debug, Serialize, Deserialize, Clone, Component)]
pub struct ScriptComponent {
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Component)]
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

impl From<CameraConfig> for CameraBuilder {
    fn from(value: CameraConfig) -> CameraBuilder {
        CameraBuilder {
            eye: value.eye.into(),
            target: value.target.into(),
            up: value.up.into(),
            aspect: value.aspect,
            znear: value.near as f64,
            zfar: value.far as f64,
            settings: CameraSettings {
                speed: value.speed as f64,
                sensitivity: value.sensitivity as f64,
                fov_y: value.fov as f64,
            },
        }
    }
}

/// A type of entity that can be serialized into a scene (which will then be serialized into a file).
///
/// This just contains the parent, the children, the components of that entity and the name of it.
///
/// This should not be added to the world. 
#[derive(Default, Serialize, Deserialize)]
pub struct SceneEntity {
    pub label: Label,
    pub components: Vec<Box<dyn Component>>,
    pub parent: Label,
    pub children: Vec<SceneEntity>,
}

impl Clone for SceneEntity {
    fn clone(&self) -> Self {
        Self {
            label: self.label.clone(),
            components: self.components.iter()
                .map(|c| c.clone_component())
                .collect(),
            children: self.children.clone(),
            parent: self.parent.clone(),
        }
    }
}

impl fmt::Debug for SceneEntity {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("SceneEntity")
            .field("label", &self.label)
            .field("components_count", &self.components.len())
            .field("parent", &self.parent)
            .field("children_count", &self.children.len())
            .finish()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Component)]
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

/// A component that holds basic information for a [`MeshRenderer`].
#[derive(Serialize, Deserialize, Clone, Component)]
pub struct SceneMeshRendererComponent {
    pub model: ResourceReference,
    pub material_overrides: Vec<MaterialOverride>,
}

/// The config of a scene. Organises the content for serialization for a scene and for packing
/// into a runtime.
#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct SceneConfig {
    #[serde(default)]
    pub scene_name: String,

    #[serde(default)]
    pub entities: Vec<SceneEntity>,

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

    /// Loads a [`SceneConfig`] into a [`hecs::World`] asynchronously
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
        log::info!("World cleared, now has {} entities", world.len());

        let mut label_to_entity: HashMap<Label, hecs::Entity> = HashMap::new();

        // entities
        for (index, entity_config) in self.entities.iter().enumerate() {
            let label_str = entity_config.label.to_string();
            
            log::debug!("Loading entity: {}", label_str);

            if let Some(ref s) = progress_sender {
                let _ = s.send(WorldLoadingStatus::LoadingEntity {
                    index,
                    name: label_str.clone(),
                    total: self.entities.len(),
                });
            }

            let mut builder = hecs::EntityBuilder::new();
            builder.add(entity_config.label.clone());

            let mut mesh_renderer: Option<MeshRenderer> = None;
            let mut world_transform: Option<WorldTransform> = None;
            let mut local_transform: Option<LocalTransform> = None;
            let mut camera_config: Option<CameraConfig> = None;
            let mut light_config: Option<(LightComponent, Transform)> = None;

            for comp in &entity_config.components {
                if let Some(mesh_comp) = comp.as_any().downcast_ref::<SceneMeshRendererComponent>() {
                    match self.load_mesh_renderer(mesh_comp, &graphics, &label_str).await {
                        Ok(renderer) => mesh_renderer = Some(renderer),
                        Err(e) => {
                            log::error!("Failed to load mesh for entity '{}': {}", label_str, e);
                            continue;
                        }
                    }
                }
                else if let Some(wt) = comp.as_any().downcast_ref::<WorldTransform>() {
                    world_transform = Some(*wt);
                }
                else if let Some(lt) = comp.as_any().downcast_ref::<LocalTransform>() {
                    local_transform = Some(*lt);
                }
                else if let Some(cam_cfg) = comp.as_any().downcast_ref::<CameraConfig>() {
                    camera_config = Some(cam_cfg.clone());
                }
                else if let Some(light_comp) = comp.as_any().downcast_ref::<LightComponent>() {
                    let transform = local_transform
                        .map(|lt| lt.into_inner())
                        .unwrap_or_default();
                    light_config = Some((light_comp.clone(), transform));
                }
                else if let Some(script_comp) = comp.as_any().downcast_ref::<ScriptComponent>() {
                    builder.add(script_comp.clone());
                }
                else if let Some(props) = comp.as_any().downcast_ref::<ModelProperties>() {
                    builder.add(props.clone());
                }
            }

            let lt = local_transform.unwrap_or_default();
            let wt = world_transform.unwrap_or_else(|| WorldTransform::from_transform(lt.into_inner()));

            builder.add(lt);
            builder.add(wt);

            if let Some(mut renderer) = mesh_renderer {
                renderer.update(wt.inner());
                builder.add(renderer);
            }

            // camera
            if let Some(cam_cfg) = camera_config {
                let camera = Camera::new(
                    graphics.clone(),
                    cam_cfg.clone().into(),
                    Some(&cam_cfg.label),
                );

                let camera_component = CameraComponent {
                    settings: CameraSettings::new(
                        cam_cfg.speed as f64,
                        cam_cfg.sensitivity as f64,
                        cam_cfg.fov as f64,
                    ),
                    camera_type: cam_cfg.camera_type,
                    starting_camera: cam_cfg.starting_camera,
                };

                builder.add(camera);
                builder.add(camera_component);
            }

            // light
            if let Some((light_comp, transform)) = light_config {
                let light = Light::new(
                    graphics.clone(),
                    light_comp.clone(),
                    transform,
                    Some(&label_str),
                )
                .await;

                builder.add(light_comp);
                builder.add(light);
            }

            let entity = world.spawn(builder.build());
            
            if let Some(previous) = label_to_entity.insert(entity_config.label.clone(), entity) {
                log::warn!(
                    "Duplicate entity label '{}' detected; previous entity {:?} will be overwritten",
                    label_str,
                    previous
                );
            }

            log::debug!("Loaded entity '{}'", label_str);
        }

        // parent-child relationships
        for entity_config in &self.entities {
            if entity_config.children.is_empty() {
                continue;
            }

            let Some(&parent_entity) = label_to_entity.get(&entity_config.label) else {
                log::warn!(
                    "Unable to resolve parent entity '{}' while rebuilding hierarchy",
                    entity_config.label
                );
                continue;
            };

            let mut resolved_children = Vec::new();
            for child in &entity_config.children {
                if let Some(&child_entity) = label_to_entity.get(&child.label) {
                    resolved_children.push(child_entity);
                } else {
                    log::warn!(
                        "Unable to resolve child '{}' for parent '{}'",
                        child.label,
                        entity_config.label
                    );
                }
            }

            if resolved_children.is_empty() {
                continue;
            }

            match world.query_one_mut::<&mut Parent>(parent_entity) {
                Ok(parent_comp) => {
                    parent_comp.clear();
                    parent_comp.children_mut().extend(resolved_children);
                }
                Err(_) => {
                    if let Err(e) = world.insert_one(parent_entity, Parent::new(resolved_children)) {
                        log::error!(
                            "Failed to attach Parent component to entity {:?}: {}",
                            parent_entity,
                            e
                        );
                    }
                }
            }
        }

        log::info!("Loaded {} entities with hierarchy", self.entities.len());

        {
            let has_light = world
                .query::<(&LightComponent, &Light)>()
                .iter()
                .next()
                .is_some();

            if !has_light {
                log::info!("No lights in scene, spawning default light");
                if let Some(ref s) = progress_sender {
                    let _ = s.send(WorldLoadingStatus::LoadingLight {
                        index: 0,
                        name: String::from("Default Light"),
                        total: 1,
                    });
                }

                let comp = LightComponent::directional(DVec3::ONE, 1.0);
                let light_direction = LightComponent::default_direction();
                let rotation = DQuat::from_rotation_arc(DVec3::new(0.0, 0.0, -1.0), light_direction);
                let transform = Transform {
                    position: DVec3::new(2.0, 4.0, 2.0),
                    rotation,
                    ..Default::default()
                };
                
                let light = Light::new(
                    graphics.clone(),
                    comp.clone(),
                    transform,
                    Some("Default Light"),
                )
                .await;

                let local = LocalTransform::from_transform(transform);
                let world_t = WorldTransform::from_transform(transform);

                world.spawn((
                    Label::from("Default Light"),
                    comp,
                    local,
                    world_t,
                    light,
                    ModelProperties::default(),
                ));
            }
        }

        self.setup_camera(world, &graphics, &progress_sender).await
    }

    /// Helper function to load a mesh renderer from a SceneMeshRendererComponent
    async fn load_mesh_renderer(
        &self,
        comp: &SceneMeshRendererComponent,
        graphics: &Arc<SharedGraphicsContext>,
        label: &str,
    ) -> anyhow::Result<MeshRenderer> {
        use dropbear_engine::utils::ResourceReferenceType;

        let mut renderer = match &comp.model.ref_type {
            ResourceReferenceType::File(reference) => {
                let path = comp.model.resolve()?;
                log::debug!(
                    "Loading model for entity '{}' from path {} (ref: {})",
                    label,
                    path.display(),
                    reference
                );
                MeshRenderer::from_path(graphics.clone(), &path, Some(label)).await?
            }
            ResourceReferenceType::Bytes(bytes) => {
                log::info!("Loading entity '{}' from bytes [Len: {}]", label, bytes.len());
                let model = Model::load_from_memory(
                    graphics.clone(),
                    bytes.clone(),
                    Some(label),
                ).await?;
                MeshRenderer::from_handle(model)
            }
            ResourceReferenceType::Cube => {
                log::info!("Loading entity '{}' as cube", label);
                let model = Model::load_from_memory(
                    graphics.clone(),
                    include_bytes!("../../resources/models/cube.glb").to_vec(),
                    Some(label),
                ).await?;
                MeshRenderer::from_handle(model)
            }
            ResourceReferenceType::None => {
                anyhow::bail!("No model reference provided for entity '{}'", label);
            }
            ResourceReferenceType::Plane => {
                anyhow::bail!("Plane resource type is no longer supported for entity '{}'", label);
            }
        };

        if !comp.material_overrides.is_empty() {
            for override_entry in &comp.material_overrides {
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

                if let Err(err) = renderer.apply_material_override(
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

        Ok(renderer)
    }

    /// Helper function to setup the appropriate camera for the scene
    async fn setup_camera(
        &self,
        world: &mut hecs::World,
        graphics: &Arc<SharedGraphicsContext>,
        progress_sender: &Option<UnboundedSender<WorldLoadingStatus>>,
    ) -> anyhow::Result<hecs::Entity> {
        #[cfg(feature = "editor")]
        {
            // Look for existing debug camera
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
                if let Some(s) = progress_sender.as_ref() {
                    let _ = s.send(WorldLoadingStatus::LoadingCamera {
                        index: 0,
                        name: String::from("Viewport Camera"),
                        total: 1,
                    });
                }
                let camera = Camera::predetermined(graphics.clone(), Some("Viewport Camera"));
                let component = crate::camera::DebugCamera::new();
                let camera_entity = world.spawn((camera, component));
                Ok(camera_entity)
            }
        }

        #[cfg(not(feature = "editor"))]
        {
            // Runtime mode: must have a player camera
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
                anyhow::bail!("Runtime mode requires a player camera, but none was found in the scene!");
            }
        }
    }
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

#[deprecated = "This RuntimeData struct should not be used at all for any projects, a new one is being made"]
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

/// A label. Contains the name of something as a [String]. Nothing fancy ova 'ere. 
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
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
