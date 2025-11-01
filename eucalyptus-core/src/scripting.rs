pub mod jni;
pub mod native;

use crate::input::InputState;
use crate::ptr::{GraphicsPtr, InputStatePtr, WorldPtr};
use crate::scripting::jni::JavaContext;
use crate::scripting::native::NativeLibrary;
use crate::states::ScriptComponent;
use anyhow::Context;
use crossbeam_channel::Sender;
use hecs::{Entity, World};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

/// The target of the script. This can be either a JVM or a native library.
#[derive(Default, Clone)]
pub enum ScriptTarget {
    #[default]
    /// The default target. Using this will always return an error.
    None,
    /// JVM target. This will load the script into a dropbear hosted JVM instance.  
    JVM {
        /// Path to the JAR file. This is the file that will be loaded into the JVM.
        library_path: PathBuf,
    },
    /// Native target. This will load the library_path of this enum.
    Native {
        /// Path to the library. This is the file that will be loaded into the JVM.
        library_path: PathBuf,
    },
}

/// An enum representing the status of the build process.
///
/// This is used for cross-thread [`crossbeam_channel::unbounded`] channels
#[derive(Debug, Clone)]
pub enum BuildStatus {
    Started,
    Building(String),
    Completed,
    Failed(String),
}

pub struct ScriptManager {
    /// The JVM instance. This is only set if the [`ScriptTarget`] is [`ScriptTarget::JVM`].
    jvm: Option<JavaContext>,
    /// The library instance. This is only set if the [`ScriptTarget`] is [`ScriptTarget::Native`].
    library: Option<NativeLibrary>,
    /// The target of the script. This can be either a JVM or a native library (or None, but why
    /// would you set it as that?)
    script_target: ScriptTarget,
    /// The entity tag database. This is a map of tag<->list of entities.
    entity_tag_database: HashMap<String, Vec<Entity>>,
    /// Whether or not the JVM has been created.
    ///
    /// This bool is required as the JNI specifications only allow for one JVM per process.
    jvm_created: bool,
    /// The path to the library. This is set if the [`ScriptTarget`] is [`ScriptTarget::Native`] or
    /// [`ScriptTarget::JVM`]
    lib_path: Option<PathBuf>,
}

impl ScriptManager {
    /// Creates a new [`ScriptManager`] uninitialised instance, as well as a new
    /// JVM instance.
    pub fn new() -> anyhow::Result<Self> {
        #[allow(unused_mut)]
        let mut result = Self {
            jvm: None,
            library: None,
            script_target: Default::default(),
            entity_tag_database: HashMap::new(),
            jvm_created: false,
            lib_path: None,
        };

        #[cfg(feature = "jvm")]
        // using this feature is automatically supported by the "editor" feature flag
        {
            // JavaContext will only be created if developer explicitly specifies.
            let jvm = JavaContext::new()?;
            result.jvm = Some(jvm);
            result.jvm_created = true;
            log::debug!("Created new JVM instance");
        }

        Ok(result)
    }

    /// Initialises the library by loading it into memory or into the JVM depending on the
    /// target.
    ///
    /// This function required a [`HashMap<String, Vec<Entity>>`], which has a tag<->list of entities
    /// link. It is stored in memory until the script is reinitialised.
    ///
    /// This function is only required to be run once at the start of the session.
    pub fn init_script(
        &mut self,
        entity_tag_database: HashMap<String, Vec<Entity>>,
        target: ScriptTarget,
    ) -> anyhow::Result<()> {
        self.entity_tag_database = entity_tag_database.clone();
        self.script_target = target.clone();

        match &target {
            ScriptTarget::JVM { library_path } => {
                self.lib_path = Some(library_path.clone());

                if !self.jvm_created {
                    let jvm = JavaContext::new()?;
                    self.jvm = Some(jvm);
                    self.jvm_created = true;
                    log::debug!("Created new JVM instance");
                } else {
                    log::debug!("Reusing existing JVM instance");
                    if let Some(jvm) = &mut self.jvm {
                        jvm.jar_path = library_path.clone();
                    }
                }

                if let Some(jvm) = &mut self.jvm {
                    jvm.clear_engine()?;
                }
            }
            ScriptTarget::Native { library_path } => {
                self.library = Some(NativeLibrary::new(library_path)?);
                self.lib_path = Some(library_path.clone());
            }
            ScriptTarget::None => {
                self.jvm = None;
                self.library = None;
                self.jvm_created = false;
                self.lib_path = None;
            }
        }

        Ok(())
    }

    /// Loads and initialises the script for the specified script target.
    ///
    /// This function only needs to be called once at the start of the session.
    ///
    /// # ScriptTarget behaviours
    /// - [`ScriptTarget::JVM`] - This initialises the JVM by setting specific contexts such
    ///   as necessary pointer/handles with [`JavaContext::load_systems_for_tag`]. After it
    ///   loads each system for each tag.
    /// - [`ScriptTarget::Native`] - This initialises the library using [`NativeLibrary::init`].
    ///   After it loads the necessary system with the tag.
    /// - [`ScriptTarget::None`] - This returns an [`Err`], as no script target would have been
    ///   set.
    pub fn load_script(
        &mut self,
        world: WorldPtr,
        input_state: InputStatePtr,
        graphics: GraphicsPtr,
    ) -> anyhow::Result<()> {
        match &self.script_target {
            ScriptTarget::JVM { .. } => {
                if let Some(jvm) = &mut self.jvm {
                    jvm.init(world, input_state, graphics)?;
                    for tag in self.entity_tag_database.keys() {
                        log::trace!("Loading systems for tag: {}", tag);
                        jvm.load_systems_for_tag(tag)?;
                    }
                    return Ok(());
                }
            }
            ScriptTarget::Native { .. } => {
                if let Some(library) = &mut self.library {
                    library.init(world, input_state, graphics)?;
                    for tag in self.entity_tag_database.keys() {
                        log::trace!("Loading systems for tag: {}", tag);
                        library.load_systems(tag.to_string())?;
                    }
                    return Ok(());
                }
            }
            ScriptTarget::None => {
                return Err(anyhow::anyhow!("No script target set"));
            }
        }

        Err(anyhow::anyhow!("Invalid script target configuration"))
    }

    /// Updates the script as loaded into [`ScriptManager`].
    ///
    /// This function needs to be called every frame.
    ///
    /// # ScriptTarget behaviours
    /// - [`ScriptTarget::JVM`] - This runs [`JavaContext::update_all_systems`] if the database is
    ///   empty, [`JavaContext::update_systems_for_tag`] if there are tags but no entities, and
    ///   [`JavaContext::update_systems_for_entities`] if there are entities.
    /// - [`ScriptTarget::Native`] - This runs [`NativeLibrary::update_all`] if the database is
    ///   empty, [`NativeLibrary::update_systems_for_tag`] if there are tags but no entities, and
    ///   [`NativeLibrary::update_systems_for_entities`] if there are entities.
    /// - [`ScriptTarget::None`] - This returns an error.
    ///
    /// # Safety
    /// This function is marked unsafe because clippy forced me to, but also
    /// world is rebuilt from the pointer.
    pub unsafe fn update_script(
        &mut self,
        _world: WorldPtr,
        _input_state: &InputState,
        dt: f32,
    ) -> anyhow::Result<()> {
        if let Some(world) = unsafe { _world.as_ref() } {
            self.rebuild_entity_tag_database(world);
        }

        match self.script_target {
            ScriptTarget::None => Err(anyhow::anyhow!(
                "ScriptTarget is set to None. Either set to JVM or Native"
            )),
            ScriptTarget::JVM { .. } => {
                if let Some(jvm) = &self.jvm {
                    if self.entity_tag_database.is_empty() {
                        jvm.update_all_systems(dt)?;
                    } else {
                        for (tag, entities) in &self.entity_tag_database {
                            let entity_ids: Vec<i64> = entities
                                .iter()
                                .map(|entity| entity.to_bits().get() as i64)
                                .collect();

                            if entity_ids.is_empty() {
                                jvm.update_systems_for_tag(tag, dt)?;
                            } else {
                                jvm.update_systems_for_entities(tag, &entity_ids, dt)?;
                            }
                        }
                    }
                    return Ok(());
                }
                Err(anyhow::anyhow!(
                    "ScriptTarget is set to JVM but JVM is None"
                ))
            }
            ScriptTarget::Native { .. } => {
                if let Some(library) = &mut self.library {
                    library.update_all(dt)?;
                    return Ok(());
                }
                Err(anyhow::anyhow!(
                    "ScriptTarget is set to Native but library is None"
                ))
            }
        }
    }

    /// Reloads the .jar file by unloading the previous classes and reloading them back in,
    /// allowing for hot reloading.
    ///
    /// # ScriptTarget behaviours
    /// - [`ScriptTarget::JVM`] - This target is the only target that allows this function.
    /// - [`ScriptTarget::Native`] - This target does not do anything, but does not result in an
    ///   error (returns [`Ok`])
    /// - [`ScriptTarget::None`] - This target does not do anything, but does not result in an
    ///   error (returns [`Ok`])
    pub fn reload(&mut self, world_ptr: WorldPtr) -> anyhow::Result<()> {
        if let Some(jvm) = &mut self.jvm {
            jvm.reload(world_ptr)?
        }
        Ok(())
    }

    /// Rebuilds the ScriptManagers entity database by parsing a [`World`].
    fn rebuild_entity_tag_database(&mut self, world: &World) {
        let mut new_map: HashMap<String, Vec<Entity>> = HashMap::new();

        for (entity, script) in world.query::<&ScriptComponent>().iter() {
            for tag in &script.tags {
                new_map.entry(tag.clone()).or_default().push(entity);
            }
        }

        self.entity_tag_database = new_map;
    }
}

fn get_gradle_command(project_root: impl AsRef<Path>) -> String {
    let project_root = project_root.as_ref().to_owned();
    if cfg!(target_os = "windows") {
        let gradlew = project_root.join("gradlew.bat");
        if gradlew.exists() {
            gradlew.to_string_lossy().to_string()
        } else {
            "gradle.bat".to_string()
        }
    } else {
        let gradlew = project_root.join("gradlew");
        if gradlew.exists() {
            "./gradlew".to_string()
        } else {
            "gradle".to_string()
        }
    }
}

pub async fn build_jvm(
    project_root: impl AsRef<Path>,
    status_sender: Sender<BuildStatus>,
) -> anyhow::Result<PathBuf> {
    let project_root = project_root.as_ref();

    if !project_root.exists() {
        let err = format!("Project root does not exist: {:?}", project_root);
        let _ = status_sender.send(BuildStatus::Failed(err.clone()));
        return Err(anyhow::anyhow!(err));
    }

    if !(project_root.join("build.gradle").exists()
        || project_root.join("build.gradle.kts").exists())
    {
        let err = format!("No Gradle build script found in: {:?}", project_root);
        let _ = status_sender.send(BuildStatus::Failed(err.clone()));
        return Err(anyhow::anyhow!(err));
    }

    let _ = status_sender.send(BuildStatus::Started);

    let gradle_cmd = get_gradle_command(project_root);

    let _ = status_sender.send(BuildStatus::Building(format!("Running: {}", gradle_cmd)));

    let mut child = Command::new(&gradle_cmd)
        .current_dir(project_root)
        .args(["--console=plain", "fatJar"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .context(format!("Failed to spawn `{} fatJar`", gradle_cmd))?;

    let stdout = child.stdout.take().expect("Stdout was piped");
    let stderr = child.stderr.take().expect("Stderr was piped");

    let tx_out = status_sender.clone();
    let stdout_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = tx_out.send(BuildStatus::Building(line));
        }
    });

    let tx_err = status_sender.clone();
    let stderr_task = tokio::spawn(async move {
        let mut reader = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let _ = tx_err.send(BuildStatus::Building(line));
        }
    });

    let status = child
        .wait()
        .await
        .context("Failed to wait for gradle process")?;

    let _ = tokio::join!(stdout_task, stderr_task);

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        let err_msg = format!("Gradle build failed with exit code {}", code);
        let _ = status_sender.send(BuildStatus::Failed(err_msg.clone()));
        return Err(anyhow::anyhow!(err_msg));
    }

    let libs_dir = project_root.join("build").join("libs");
    if !libs_dir.exists() {
        let err = "Build succeeded but 'build/libs' directory is missing".to_string();
        let _ = status_sender.send(BuildStatus::Failed(err.clone()));
        return Err(anyhow::anyhow!(err));
    }

    let jar_files: Vec<PathBuf> = std::fs::read_dir(&libs_dir)
        .context("Failed to read 'build/libs'")?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| {
            path.extension()
                .map_or(false, |ext| ext.eq_ignore_ascii_case("jar"))
                && !path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .contains("-sources")
                && !path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .contains("-javadoc")
        })
        .collect();

    if jar_files.is_empty() {
        let err = "No JAR artifact found in 'build/libs'".to_string();
        let _ = status_sender.send(BuildStatus::Failed(err.clone()));
        return Err(anyhow::anyhow!(err));
    }

    let fat_jar = jar_files.iter().find(|path| {
        path.file_name()
            .and_then(|n| n.to_str())
            .map_or(false, |name| name.contains("-all"))
    });

    let jar_path = if let Some(fat) = fat_jar {
        fat.clone()
    } else {
        jar_files
            .into_iter()
            .max_by_key(|path| std::fs::metadata(path).map(|m| m.len()).unwrap_or(0))
            .unwrap()
    };

    let _ = status_sender.send(BuildStatus::Completed);
    Ok(jar_path)
}
