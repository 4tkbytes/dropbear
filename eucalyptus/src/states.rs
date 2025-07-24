//! In this module, it will describe all the different types for
//! storing configuration files (.eucp for project and .eucc for config files for subdirectories).
//!
//! There is a singleton that is used for other crates to access,
//! as well as public structs related to that config and docs (hopefully).

use std::{
    fs,
    path::{Path, PathBuf},
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
    pub assets: Assets,
}

impl ProjectConfig {
    /// Creates a new instance of the ProjectConfig. This function is typically used when creating
    /// a new project, with it creating new defaults for everything.
    pub fn new(project_name: String, project_path: &PathBuf) -> Self {
        let date_created = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        let project_path_str = project_path.to_str().unwrap().to_string();
        let date_last_accessed = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        Self {
            project_name,
            project_path: project_path_str,
            date_created,
            date_last_accessed,
            dock_layout: None,
            assets: Assets::walk(&project_path),
        }
    }

    /// This function writes the [`ProjectConfig`] struct (and other PathBufs) to a file of the choice
    /// under the PathBuf path parameter.
    ///
    /// # Parameters
    /// * path - The root **folder** of the project.
    pub fn write_to(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        self.date_last_accessed = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        self.assets = Assets::walk(path);
        let ron_str = ron::ser::to_string_pretty(&self, PrettyConfig::default())
            .map_err(|e| anyhow::anyhow!("RON serialization error: {}", e))?;
        let config_path = path.join(format!("{}.eucp", self.project_name.clone().to_lowercase()));
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
        config.write_to(&path.parent().unwrap().to_path_buf())?;
        config.assets = Assets::walk(&path.parent().unwrap().to_path_buf());
        log::debug!("Successfully updated!");
        Ok(config)
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Assets {
    nodes: Vec<Node>,
}

pub(crate) fn path_contains_folder(path: &PathBuf, folder: &str) -> bool {
    path.components().any(|comp| comp.as_os_str() == folder)
}

impl Assets {
    /// This function goes into your project directory and "walks" (recursively) / fetches other configuration files
    /// and other assets/scripts for the .eucp project file to look for.
    ///
    /// If there are config files missing, it will generate the config file and populate it, then
    /// create a reference to that folder config file (.eucc) to the .eucp project config file.
    pub fn walk(project_path: &PathBuf) -> Self {
        fn locate_config_in_dir(path: &Path) -> Vec<Node> {
            let mut nodes = Vec::new();

            if let Ok(entries) = fs::read_dir(path) {
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    let name = entry_path.file_name().unwrap().to_str().unwrap();
                    if path_contains_folder(&entry_path, ".git") {
                        continue;
                    }

                    if entry_path.is_dir() {
                        let config_path = entry_path.join(format!("{}.eucc", name));
                        let mut folder = Folder {
                            name: String::from(name),
                            path: entry_path.clone(),
                            nodes: locate_config_in_dir(&entry_path),
                        };

                        if config_path.exists() {
                            folder.nodes.push(Node::File(File {
                                name: format!("{}.eucc", name),
                                path: config_path.clone(),
                            }));
                        }

                        nodes.push(Node::Folder(folder));
                    } else if entry_path.extension().map_or(false, |ext| ext == "eucc") {
                        let parent = entry_path
                            .parent()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_str().unwrap());
                        let expected_name = parent.map(|n| format!("{}.eucc", n));
                        if Some(name) != expected_name.as_deref() {
                            nodes.push(Node::File(File {
                                name: String::from(name),
                                path: entry_path.clone(),
                            }));
                        }
                    }
                }
            }

            nodes
        }

        Assets {
            nodes: locate_config_in_dir(project_path),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Node {
    File(File),
    Folder(Folder),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConfigType {
    None,
    Project(ProjectConfig),
    Resource(ResourceConfig),
    Source(SourceCodeConfig),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResourceConfig {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SourceCodeConfig {
    pub nodes: Vec<Node>,
}

impl Default for ConfigType {
    fn default() -> Self {
        ConfigType::None
    }
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct File {
    pub name: String,
    /// A reference from the root node
    pub path: PathBuf,
}

#[derive(Default, Debug, Serialize, Deserialize)]
pub struct Folder {
    pub name: String,
    pub path: PathBuf,
    pub nodes: Vec<Node>,
}
