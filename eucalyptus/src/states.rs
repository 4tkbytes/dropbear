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
use dropbear_engine::log;
use egui_dock_fork::DockState;
use once_cell::sync::Lazy;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use crate::editor::EditorTab;

pub static PROJECT: Lazy<RwLock<ProjectConfig>> =
    Lazy::new(|| RwLock::new(ProjectConfig::default()));

/// The root config file, responsible for building and other metadata.
///
/// # Location
/// This file is {project_name}.eucp and is located at {project_dir}/
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ProjectConfig {
    pub project_name: String,
    pub project_path: String,
    pub date_created: String,
    pub date_last_accessed: String,
    #[serde(default)]
    pub dock_layout: Option<DockState<EditorTab>>,
    #[serde(default)]
    pub resources_config: Option<ResourceConfig>,
    #[serde(default)]
    pub source_config: Option<SourceConfig>,
}

impl ProjectConfig {
    /// Creates a new instance of the ProjectConfig. This function is typically used when creating
    /// a new project, with it creating new defaults for everything.
    pub fn new(project_name: String, project_path: &PathBuf) -> Self {
        let date_created = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        let project_path_str = project_path.to_str().unwrap().to_string();
        let date_last_accessed = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        let mut result = Self {
            project_name,
            project_path: project_path_str,
            date_created,
            date_last_accessed,
            dock_layout: None,
            resources_config: None,
            source_config: None,
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
        self.project_path = path.clone().to_str().unwrap().to_string();

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
        log::info!("Loaded project!");
        log::debug!("Loaded config info: {:#?}", config);
        log::debug!("Updating with new content");
        // config.write_to(&path.parent().unwrap().to_path_buf())?;
        // config.assets = Assets::walk(&path.parent().unwrap().to_path_buf());
        config.load_config_to_memory()?;
        config.write_to_all()?;
        log::debug!("Successfully updated!");
        Ok(config)
    }

    /// This function loads a `source.eucc` or a `resources.eucc` config file into memory, allowing
    /// you to reference and load the nodes located inside them.
    pub fn load_config_to_memory(&mut self) -> anyhow::Result<()> {
        let project_root = PathBuf::from(&self.project_path);

        match ResourceConfig::read_from(&project_root) {
            Ok(resources) => self.resources_config = Some(resources),
            Err(e) => {
                if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                    if io_err.kind() == std::io::ErrorKind::NotFound {
                        log::warn!("resources.eucc not found, creating default.");
                        let default = ResourceConfig {
                            path: project_root.join("resources"),
                            nodes: vec![],
                        };
                        default.write_to(&project_root)?;
                        self.resources_config = Some(default);
                    } else {
                        log::warn!("Failed to load resources.eucc: {}", e);
                        self.resources_config = None;
                    }
                } else {
                    log::warn!("Failed to load resources.eucc: {}", e);
                    self.resources_config = None;
                }
            }
        }

        match SourceConfig::read_from(&project_root) {
            Ok(source) => self.source_config = Some(source),
            Err(e) => {
                if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                    if io_err.kind() == std::io::ErrorKind::NotFound {
                        log::warn!("source.eucc not found, creating default.");
                        let default = SourceConfig {
                            path: project_root.join("src"),
                            nodes: vec![],
                        };
                        default.write_to(&project_root)?;
                        self.source_config = Some(default);
                    } else {
                        log::warn!("Failed to load source.eucc: {}", e);
                        self.source_config = None;
                    }
                } else {
                    log::warn!("Failed to load source.eucc: {}", e);
                    self.source_config = None;
                }
            }
        }

        Ok(())
    }

    /// # Parameters
    /// * path - The root folder of the project
    pub fn write_to_all(&mut self) -> anyhow::Result<()> {
        let path = PathBuf::from(self.project_path.clone());
        if let Some(res) = &self.resources_config {
            res.write_to(&path)?;
        }

        if let Some(src) = &self.source_config {
            src.write_to(&path)?;
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
#[derive(Debug, Serialize, Deserialize)]
pub enum ResourceType {
    Unknown,
    Model(Model),
    Thumbnail,
    Texture,
    Shader,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Model {
    pub thumbnail_location: PathBuf,
}

impl Model {
    pub fn gen_thumbnail(project_path: &PathBuf, model_path: &PathBuf) -> Self {
        let thumbnail_path = model_path
            .parent()
            .unwrap()
            .join("thumbnails")
            .join(format!(
                "{}.png",
                model_path.file_stem().unwrap().to_string_lossy()
            ));
        if !thumbnail_path.exists() {
            crate::utils::convert_model_to_image(project_path, model_path);
        }
        Self {
            thumbnail_location: thumbnail_path,
        }
    }
}

impl Display for ResourceType {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let str = match self {
            ResourceType::Unknown => "unknown",
            ResourceType::Model(_) => "model",
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
    /// # Parameters
    /// - path: The root **folder** of the project
    pub fn write_to(&self, path: &PathBuf) -> anyhow::Result<()> {
        let resource_dir = path.join("resources");
        let updated_config = ResourceConfig {
            path: resource_dir.clone(),
            nodes: collect_nodes(&resource_dir, path, vec!["thumbnails"].as_slice()),
        };

        let ron_str = ron::ser::to_string_pretty(&updated_config, PrettyConfig::default())
            .map_err(|e| anyhow::anyhow!("RON serialisation error: {}", e))?;
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
                    Some(ResourceType::Model(Model::gen_thumbnail(
                        project_path,
                        &entry_path,
                    )))
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
