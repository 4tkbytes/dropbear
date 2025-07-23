//! In this module, it will describe all the different types for
//! storing configuration files (.eucp for project and .eucc for config files for subdirectories).
//!
//! There is a singleton that is used for other crates to access,
//! as well as public structs related to that config and docs (hopefully).

use std::{fs, path::PathBuf, sync::RwLock};

use chrono::Utc;
use dropbear_engine::log;
use egui_dock::DockState;
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
    pub dock_layout: Option<DockState<EditorTab>>,
}

impl ProjectConfig {
    /// Creates a new instance of the ProjectConfig. This function is typically used when creating
    /// a new project, with it creating new defaults for everything.
    pub fn new(project_name: String, project_path: &PathBuf) -> Self {
        let date_created = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        let project_path = project_path.to_str().unwrap().to_string();
        let date_last_accessed = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        Self {
            project_name,
            project_path,
            date_created,
            date_last_accessed,
            dock_layout: None,
        }
    }

    /// This function writes the [`ProjectConfig`] struct (and other PathBufs) to a file of the choice
    /// under the PathBuf path parameter.
    ///
    /// # Parameters
    /// * path - The root **folder** of the project.
    pub fn write_to(&mut self, path: &PathBuf) -> anyhow::Result<()> {
        self.date_last_accessed = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
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
        log::debug!("Successfully updated!");
        Ok(config)
    }
}
