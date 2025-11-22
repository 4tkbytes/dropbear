use crate::editor::Editor;
use app_dirs2::AppDataType;
use egui::Ui;
use eucalyptus_core::APP_INFO;
use eucalyptus_core::states::PluginInfo;
use eucalyptus_core::traits::component_registry::ComponentRegistry;
use indexmap::IndexMap;
use libloading as lib;
use std::fs::ReadDir;
use std::path::PathBuf;
use std::time::Instant;

pub trait EditorPlugin: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn ui(&mut self, ui: &mut Ui, editor: &mut Editor);
    fn tab_title(&self) -> &str;

    /// Allows you to register the struct [Self] as a component to the component registry.
    /// This will then be listed as an option for a potential component a user could add.
    fn register_component(&mut self, registry: &mut ComponentRegistry);
}

pub type PluginConstructor = fn() -> Box<dyn EditorPlugin>;

pub struct PluginRegistry {
    pub plugins: IndexMap<String, Box<dyn EditorPlugin>>,
    loaded_libraries: Vec<lib::Library>,
    pub(crate) plugins_loaded: bool,
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> Self {
        Self {
            plugins: IndexMap::new(),
            loaded_libraries: Vec::new(),
            plugins_loaded: false,
        }
    }

    pub fn register(&mut self, plugin: Box<dyn EditorPlugin>) {
        let id = plugin.id().to_string();
        self.plugins.insert(id, plugin);
    }

    #[allow(clippy::borrowed_box)]
    pub fn get(&self, id: &str) -> Option<&Box<dyn EditorPlugin>> {
        self.plugins.get(id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Box<dyn EditorPlugin>> {
        self.plugins.get_mut(id)
    }

    pub fn list_plugins(&self) -> Vec<PluginInfo> {
        self.plugins
            .values()
            .map(|p| PluginInfo {
                display_name: p.display_name().to_string(),
            })
            .collect()
    }

    pub fn load_plugins(&mut self) -> anyhow::Result<()> {
        let appdir: PathBuf = app_dirs2::app_root(AppDataType::UserData, &APP_INFO)?;
        let plugins_folder = appdir.join("plugins");

        std::fs::create_dir_all(&plugins_folder)?;

        let contents: ReadDir = std::fs::read_dir(&plugins_folder)?;

        log::info!("Loading plugins from {}", plugins_folder.display());
        let mut index: i32 = -1;
        for (i, entry) in contents.enumerate() {
            match entry {
                Ok(e) => {
                    index = i as i32;
                    let now = Instant::now();
                    let path = e.path();
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_str().unwrap_or("");

                        if !self.is_valid_extension_for_platform(ext_str) {
                            log::warn!(
                                "Skipping plugin {} - incompatible extension for this platform",
                                path.display()
                            );
                            continue;
                        }

                        match self.load_plugin_from_file(&path) {
                            Ok(plugin) => {
                                log::info!("Successfully loaded plugin: {}", path.display());
                                log::debug!(
                                    "Plugin {} loaded in {:?}",
                                    path.display(),
                                    now.elapsed()
                                );
                                self.register(plugin);
                            }
                            Err(e) => {
                                log::error!("Failed to load plugin {}: {}", path.display(), e);
                                continue;
                            }
                        }
                    }
                }
                Err(err) => {
                    log::warn!("Failed to read directory entry: {}", err);
                    continue;
                }
            }
        }

        if index == -1 {
            log::info!("No plugins found");
        }

        self.plugins_loaded = true;
        Ok(())
    }

    fn is_valid_extension_for_platform(&self, ext: &str) -> bool {
        match ext {
            "dll" => cfg!(windows),
            "so" => cfg!(unix) && !cfg!(target_os = "macos"),
            "dylib" => cfg!(target_os = "macos"),
            _ => false,
        }
    }

    fn load_plugin_from_file(&mut self, path: &PathBuf) -> anyhow::Result<Box<dyn EditorPlugin>> {
        let library = unsafe { lib::Library::new(path)? };

        let constructor: lib::Symbol<PluginConstructor> = unsafe { library.get(b"create_plugin")? };

        let plugin = constructor();

        self.loaded_libraries.push(library);

        Ok(plugin)
    }
}
