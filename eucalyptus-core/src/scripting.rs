pub mod jni;
pub mod kmp;

use crate::input::InputState;
use crate::scripting::jni::JavaContext;
use hecs::{Entity};
use libloading::Library;
use std::path::{Path, PathBuf};
use std::{collections::HashMap};
use anyhow::Context;
use crossbeam_channel::Sender;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use crate::ptr::WorldPtr;

pub const TEMPLATE_SCRIPT: &str = include_str!("../../resources/scripting/kotlin/Template.kt");

#[derive(Default, Clone)]
pub enum ScriptTarget {
    #[default]
    None,
    JVM {
        library_path: PathBuf,
    },
    Native {
        library_path: PathBuf,
    },
}

#[derive(Debug, Clone)]
pub enum BuildStatus {
    Started,
    Building(String),
    Completed,
    Failed(String),
}

pub struct ScriptManager {
    jvm: Option<JavaContext>,
    library: Option<Library>,
    script_target: ScriptTarget,
    entity_tag_database: HashMap<String, Vec<Entity>>,
    jvm_created: bool,
    lib_path: Option<PathBuf>,
}

impl ScriptManager {
    pub fn new() -> anyhow::Result<Self> {
        let mut result = Self {
            jvm: None,
            library: None,
            script_target: Default::default(),
            entity_tag_database: HashMap::new(),
            jvm_created: false,
            lib_path: None,
        };

        let jvm = JavaContext::new()?;
        result.jvm = Some(jvm);
        result.jvm_created = true;
        log::debug!("Created new JVM instance");
        
        Ok(result)
    }

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
                let library = unsafe { Library::new(library_path)? };
                self.library = Some(library);
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

    pub fn load_script(
        &mut self,
        world: WorldPtr,
        _input_state: &InputState,
    ) -> anyhow::Result<()> {
        match &self.script_target {
            ScriptTarget::JVM { library_path } => {
                if let Some(jvm) = &mut self.jvm {
                    jvm.init(library_path, world)?;
                    for tag in self.entity_tag_database.keys() {
                        log::trace!("Loading systems for tag: {}", tag);
                        jvm.load_systems_for_tag(tag)?;
                    }
                    return Ok(());
                }
            }
            ScriptTarget::Native { library_path: _ } => {
                return Err(anyhow::anyhow!("Native library loading not implemented yet"));
            }
            ScriptTarget::None => {
                return Err(anyhow::anyhow!("No script target set"));
            }
        }

        Err(anyhow::anyhow!("Invalid script target configuration"))
    }


    pub fn update_script(
        &mut self,
        _world: WorldPtr,
        _input_state: &InputState,
        dt: f32,
    ) -> anyhow::Result<()> {
        if matches!(self.script_target, ScriptTarget::JVM { .. })
            && let Some(jvm) = &self.jvm
        {
            jvm.update_all_systems(dt)?;
            return Ok(());
        }

        Err(anyhow::anyhow!("Native implementation not implemented yet"))
    }


    pub fn reload(&mut self, world_ptr: WorldPtr) -> anyhow::Result<()> {
        if let Some(jvm) = &mut self.jvm {
            jvm.reload(world_ptr)?
        }
        Ok(())
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
    status_sender: Sender<BuildStatus>
) -> anyhow::Result<PathBuf> {
    let project_root = project_root.as_ref();

    if !project_root.exists() {
        let err = format!("Project root does not exist: {:?}", project_root);
        let _ = status_sender.send(BuildStatus::Failed(err.clone()));
        return Err(anyhow::anyhow!(err));
    }

    if !(project_root.join("build.gradle").exists() || project_root.join("build.gradle.kts").exists()) {
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

    let status = child.wait().await.context("Failed to wait for gradle process")?;

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
            path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("jar"))
                && !path.file_name().unwrap_or_default().to_string_lossy().contains("-sources")
                && !path.file_name().unwrap_or_default().to_string_lossy().contains("-javadoc")
        })
        .collect();

    if jar_files.is_empty() {
        let err = "No JAR artifact found in 'build/libs'".to_string();
        let _ = status_sender.send(BuildStatus::Failed(err.clone()));
        return Err(anyhow::anyhow!(err));
    }

    let fat_jar = jar_files
        .iter()
        .find(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map_or(false, |name| name.contains("-all"))
        });

    let jar_path = if let Some(fat) = fat_jar {
        fat.clone()
    } else {
        jar_files
            .into_iter()
            .max_by_key(|path| {
                std::fs::metadata(path).map(|m| m.len())
                    .unwrap_or(0)
            })
            .unwrap()
    };

    let _ = status_sender.send(BuildStatus::Completed);
    Ok(jar_path)
}