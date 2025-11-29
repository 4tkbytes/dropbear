use crate::scene::SceneConfig;
use crate::states::{
    EditorSettings, EditorTab, File, Folder, Node, RESOURCES, ResourceType, SCENES, SOURCE,
};
use chrono::Utc;
use egui_dock::DockState;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use crate::runtime::RuntimeSettings;

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
    pub runtime_settings: RuntimeSettings,

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
            runtime_settings: Default::default(),
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

        fn deal_with_bad_scene(
            path: &Path,
            e: &anyhow::Error,
            project_root: &Path,
        ) -> Option<SceneConfig> {
            #[cfg(feature = "editor")]
            {
                let msg = format!(
                    "Failed to load scene file: {:?}\n\nError: {}\n\nWould you like to backup the corrupted file and create a new blank scene?\n(Select 'No' to exit the application)",
                    path.file_name().unwrap_or_default(),
                    e
                );

                let should_recover = rfd::MessageDialog::new()
                    .set_title("Scene loading error")
                    .set_description(&msg)
                    .set_buttons(rfd::MessageButtons::YesNo)
                    .set_level(rfd::MessageLevel::Error)
                    .show();

                match should_recover {
                    rfd::MessageDialogResult::Yes | rfd::MessageDialogResult::Ok => {
                        let backup_path = path.with_extension("eucs.bak");
                        log::info!("Backing up bad scene to {:?}", backup_path);
                        if let Err(err) = fs::rename(path, &backup_path) {
                            log::error!("Failed to backup scene: {}", err);
                            rfd::MessageDialog::new()
                                .set_title("Backup Error")
                                .set_description(&format!("Failed to backup file: {}", err))
                                .show();
                            std::process::exit(1);
                        }

                        let name = path
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        let new_scene = SceneConfig::new(name, path.to_path_buf());

                        if let Err(err) = new_scene.write_to(project_root) {
                            log::error!("Failed to write new scene: {}", err);
                            rfd::MessageDialog::new()
                                .set_title("Write Error")
                                .set_description(&format!("Failed to create new scene file: {}", err))
                                .show();
                            std::process::exit(1);
                        }

                        return Some(new_scene);
                    },
                    _ => {
                        std::process::exit(1);
                    }
                }
            }
            #[cfg(not(feature = "editor"))]
            {
                panic!("Failed to load scene {:?}: {}", path, e);
            }
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
                                if let Some(scene) = deal_with_bad_scene(&path, &e, &project_root) {
                                    scene_configs.push(scene);
                                }
                            }
                        } else {
                            if let Some(scene) = deal_with_bad_scene(&path, &e, &project_root) {
                                scene_configs.push(scene);
                            }
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

/// The resource config.
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
