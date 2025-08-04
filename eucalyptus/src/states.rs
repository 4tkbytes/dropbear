//! In this module, it will describe all the different types for
//! storing configuration files (.eucp for project and .eucc for config files for subdirectories).
//!
//! There is a singleton that is used for other crates to access,
//! as well as public structs related to that config and docs (hopefully).

use std::{
    fmt::{self, Display, Formatter},
    fs,
    path::PathBuf,
    sync::RwLock,
};

use chrono::Utc;
use dropbear_engine::{
    camera::Camera,
    entity::{AdoptedEntity, Transform},
    graphics::Graphics,
};
use egui_dock_fork::DockState;
use hecs;
use log;
use once_cell::sync::Lazy;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use crate::editor::EditorTab;

pub static PROJECT: Lazy<RwLock<ProjectConfig>> =
    Lazy::new(|| RwLock::new(ProjectConfig::default()));

pub static RESOURCES: Lazy<RwLock<ResourceConfig>> =
    Lazy::new(|| RwLock::new(ResourceConfig::default()));

pub static SOURCE: Lazy<RwLock<SourceConfig>> = Lazy::new(|| RwLock::new(SourceConfig::default()));

pub static SCENES: Lazy<RwLock<Vec<SceneConfig>>> = 
    Lazy::new(|| RwLock::new(Vec::new()));

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
}

impl ProjectConfig {
    /// Creates a new instance of the ProjectConfig. This function is typically used when creating
    /// a new project, with it creating new defaults for everything.
    pub fn new(project_name: String, project_path: &PathBuf) -> Self {
        let date_created = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        let date_last_accessed = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
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
        log::debug!("Loaded config info: {:?}", config);
        log::debug!("Updating with new content");
        config.load_config_to_memory()?;
        config.write_to_all()?;
        log::debug!("Successfully updated!");
        Ok(config)
    }

    /// This function loads a `source.eucc` or a `resources.eucc` config file into memory, allowing
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
        scene_configs.clear(); // Clear existing scenes before loading new ones
        
        // iterate through each scene file in the folder
        let scene_folder = &project_root.join("scenes");
        
        // Create scenes directory if it doesn't exist
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
        
        // If no scenes were found, create a default scene
        if scene_configs.is_empty() {
            log::info!("No scenes found, creating default scene");
            let default_scene = SceneConfig::new(
                "Default".to_string(),
                scene_folder.join("default.eucs")
            );
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

#[derive(Debug, Serialize, Deserialize)]
pub enum Node {
    File(File),
    Folder(Folder),
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct File {
    pub name: String,
    pub path: PathBuf,
    pub resource_type: Option<ResourceType>,
}

#[derive(Default, Debug, Serialize, Deserialize)]
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

#[derive(Default, Debug, Serialize, Deserialize)]
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
    Group {
        name: String,
        children: Vec<EntityNode>,
        collapsed: bool,
    },
}

#[derive(Debug, Serialize, Deserialize)]
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

        nodes
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct SceneConfig {
    pub scene_name: String,
    pub path: PathBuf,
    pub entities: Vec<SceneEntity>,
    pub camera: SceneCameraConfig,
    // todo later
    // pub settings: SceneSettings,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SceneCameraConfig {
    pub position: [f64; 3],
    pub target: [f64; 3],
    pub up: [f64; 3],
    pub aspect: f64,
    pub fov: f32,
    pub near: f32,
    pub far: f32,
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
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SceneEntity {
    pub model_path: PathBuf,
    pub label: String,
    pub transform: Transform,
    pub script: Option<ScriptComponent>,
    #[serde(skip)]
    #[allow(dead_code)]
    pub entity_id: Option<hecs::Entity>,
}

impl SceneConfig {
    /// Creates a new instance of the scene config
    pub fn new(scene_name: String, path: PathBuf) -> Self {
        Self {
            scene_name,
            path,
            entities: Vec::new(),
            camera: SceneCameraConfig::default(),
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

    #[allow(dead_code)]
    // todo: perhaps delete this if not required?
    pub fn from_world(world: &hecs::World, scene_name: String, camera: &Camera) -> Self {
        let mut entities = Vec::new();

        for (id, (adopted, transform)) in world.query::<(&AdoptedEntity, &Transform)>().iter() {
            let script = world
                .get::<&ScriptComponent>(id)
                .ok()
                .map(|s| ScriptComponent {
                    name: s.name.clone(),
                    path: s.path.clone(),
                });

            let model_path = adopted.model().path.clone();

            entities.push(SceneEntity {
                model_path,
                label: adopted.model().label.clone(),
                transform: *transform,
                script,
                entity_id: Some(id),
            });
        }

        Self {
            scene_name,
            path: PathBuf::new(),
            entities,
            camera: SceneCameraConfig {
                position: [camera.eye.x, camera.eye.y, camera.eye.z],
                target: [camera.target.x, camera.target.y, camera.target.z],
                up: [camera.up.x, camera.up.y, camera.up.z],
                aspect: camera.aspect,
                fov: camera.fov_y as f32,
                near: camera.znear as f32,
                far: camera.zfar as f32,
            },
        }
    }

    pub fn load_into_world(
        &self,
        world: &mut hecs::World,
        graphics: &Graphics,
    ) -> anyhow::Result<Camera> {
        // todo: prompt user about clearing world
        log::info!("Loading scene [{}], clearing world with {} entities", self.scene_name, world.len());
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
                    path: script_config.path.clone()
                };
                world.spawn((adopted, transform, script));
            } else {
                world.spawn((adopted, transform));
            }
        }

        let camera = Camera::new(
            graphics,
            glam::DVec3::from_array(self.camera.position),
            glam::DVec3::from_array(self.camera.target),
            glam::DVec3::from_array(self.camera.up),
            self.camera.aspect,
            self.camera.fov as f64,
            self.camera.near as f64,
            self.camera.far as f64,
            0.125 as f64,
            0.002 as f64,
        );

        Ok(camera)
    }
}
