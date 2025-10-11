pub mod jni;
pub mod kmp;

use crate::input::InputState;
use crate::scripting::jni::JavaContext;
use crate::states::{EntityNode, PROJECT, SOURCE, ScriptComponent};
use dropbear_engine::entity::{AdoptedEntity, Transform};
use hecs::{Entity, World};
use libloading::Library;
use std::path::{Path, PathBuf};
use std::{collections::HashMap, fs};
use anyhow::Context;
use crossbeam_channel::Sender;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

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
}

impl ScriptManager {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self {
            jvm: None,
            library: None,
            script_target: Default::default(),
            entity_tag_database: HashMap::new(),
        })
    }

    pub async fn build_jvm(
        &mut self,
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

        // Determine the gradle command to use
        let gradle_cmd = if cfg!(target_os = "windows") {
            // On Windows, prefer gradlew.bat if it exists, otherwise use gradle
            let gradlew = project_root.join("gradlew.bat");
            if gradlew.exists() {
                gradlew.to_string_lossy().to_string()
            } else {
                "gradle.bat".to_string()
            }
        } else {
            // On Unix-like systems, prefer ./gradlew if it exists
            let gradlew = project_root.join("gradlew");
            if gradlew.exists() {
                "./gradlew".to_string()
            } else {
                "gradle".to_string()
            }
        };

        let _ = status_sender.send(BuildStatus::Building(format!("Running: {}", gradle_cmd)));

        let mut child = Command::new(&gradle_cmd)
            .current_dir(project_root)
            .args(["--console=plain", "jvmJar"])
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .context(format!("Failed to spawn `{} jvmJar`", gradle_cmd))?;

        let stdout = child.stdout.take().expect("Stdout was piped");
        let stderr = child.stderr.take().expect("Stderr was piped");

        let tx_out = status_sender.clone();
        let stdout_task = tokio::spawn(async move {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                // if let Ok(line) = line {
                    let _ = tx_out.send(BuildStatus::Building(line));
                // }
            }
        });

        let tx_err = status_sender.clone();
        let stderr_task = tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                // if let Ok(line) = line {
                    let _ = tx_err.send(BuildStatus::Building(line));
                // }
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
            let err = "No JAR artifact found in 'build/libs' (looked for non-source, non-javadoc JARs)".to_string();
            let _ = status_sender.send(BuildStatus::Failed(err.clone()));
            return Err(anyhow::anyhow!(err));
        }

        let jar_path = jar_files
            .into_iter()
            .max_by_key(|path| {
                std::fs::metadata(path)
                    .and_then(|m| Ok(m.len()))
                    .unwrap_or(0)
            })
            .unwrap();

        let _ = status_sender.send(BuildStatus::Completed);
        Ok(jar_path)
    }

    pub fn init_script(
        &mut self,
        entity_tag_database: HashMap<String, Vec<Entity>>,
        target: ScriptTarget,
    ) -> anyhow::Result<()> {
        self.entity_tag_database = entity_tag_database;
        let target_clone = target.clone();
        self.script_target = target_clone;

        match target {
            ScriptTarget::JVM { library_path } => {
                let jvm = JavaContext::new(library_path)?;
                self.jvm = Some(jvm);
            }
            ScriptTarget::Native { library_path } => {
                let library = unsafe { Library::new(library_path)? };

                self.library = Some(library);
            }
            _ => {
                anyhow::bail!("Invalid script target, must be either JVM or Native");
            }
        }

        Ok(())
    }

    pub fn load_script(
        &mut self,
        entity_id: hecs::Entity,
        tag: String,
        world: &mut World,
        input_state: &InputState,
    ) -> anyhow::Result<()> {
        #[cfg(feature = "jvm")]
        {
            if let Some(jvm) = &mut self.jvm {
                jvm.init(world)?;
            }
        }

        Err(anyhow::anyhow!("it aint ready yet bozo"))
    }

    pub fn update_script(
        &mut self,
        entity_id: hecs::Entity,
        world: &mut World,
        input_state: &InputState,
        dt: f32,
    ) -> anyhow::Result<()> {
        Err(anyhow::anyhow!("it aint ready yet bozo"))
    }
}

pub fn move_script_to_src(script_path: &PathBuf) -> anyhow::Result<PathBuf> {
    let project_path = {
        let project = PROJECT.read();
        project.project_path.clone()
    };

    let src_path = {
        let source_config = SOURCE.read();
        source_config.path.clone()
    };

    let scripts_dir = src_path;
    fs::create_dir_all(&scripts_dir)?;

    let filename = script_path
        .file_name()
        .ok_or_else(|| anyhow::anyhow!("Invalid script path: no filename"))?;

    let dest_path = scripts_dir.join(filename);

    if dest_path.exists() {
        log::info!(
            "Script file already exists at {:?}, returning existing path",
            dest_path
        );
        return Ok(dest_path);
    }

    const MAX_RETRIES: usize = 5;
    const RETRY_DELAY_MS: u64 = 60;

    let mut last_err: Option<std::io::Error> = None;
    for attempt in 0..=MAX_RETRIES {
        match fs::copy(script_path, &dest_path) {
            Ok(_) => {
                log::info!("Copied script from {:?} to {:?}", script_path, dest_path);
                last_err = None;
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                log::warn!(
                    "Script file already exists at {:?}, continuing anyway",
                    dest_path
                );
                last_err = None;
                break;
            }
            Err(e) => {
                if e.raw_os_error() == Some(32) && attempt < MAX_RETRIES {
                    log::warn!(
                        "Sharing violation copying script (attempt {}/{}). Retrying in {}ms: {}",
                        attempt + 1,
                        MAX_RETRIES,
                        RETRY_DELAY_MS,
                        e
                    );
                    std::thread::sleep(std::time::Duration::from_millis(RETRY_DELAY_MS));
                    last_err = Some(e);
                    continue;
                } else {
                    return Err(e.into());
                }
            }
        }
    }
    if let Some(e) = last_err {
        return Err(e.into());
    }

    {
        let source_config = SOURCE.read();
        source_config.write_to(&project_path)?;
    }

    log::info!("Moved script from {:?} to {:?}", script_path, dest_path);
    Ok(dest_path)
}

pub fn convert_entity_to_group(
    world: &mut World,
    entity_id: hecs::Entity,
) -> anyhow::Result<EntityNode> {
    if let Ok(mut query) = world.query_one::<(&AdoptedEntity, &Transform)>(entity_id) {
        if let Some((adopted, _transform)) = query.get() {
            let entity_name = adopted.model.label.clone();

            let script_node = if let Ok(script) = world.get::<&ScriptComponent>(entity_id) {
                Some(EntityNode::Script {
                    tags: script.tags.clone(),
                })
            } else {
                None
            };

            let entity_node = EntityNode::Entity {
                id: entity_id,
                name: entity_name.clone(),
            };

            if let Some(script_node) = script_node {
                Ok(EntityNode::Group {
                    name: entity_name,
                    children: vec![entity_node, script_node],
                    collapsed: false,
                })
            } else {
                Ok(entity_node)
            }
        } else {
            Err(anyhow::anyhow!("Failed to get entity components"))
        }
    } else {
        Err(anyhow::anyhow!("Failed to query entity {:?}", entity_id))
    }
}

pub fn attach_script_to_entity(
    world: &mut World,
    entity_id: hecs::Entity,
    script_component: ScriptComponent,
) -> anyhow::Result<()> {
    {
        if let Err(e) = world.insert_one(entity_id, script_component) {
            return Err(anyhow::anyhow!("Failed to attach script to entity: {}", e));
        }
    }

    log::info!("Successfully attached script to entity {:?}", entity_id);
    Ok(())
}

// pub enum ScriptAction {
//     AttachScript {
//         script_path: PathBuf,
//         script_name: String,
//     },
//     CreateAndAttachScript {
//         script_path: PathBuf,
//         script_name: String,
//     },
//     RemoveScript,
//     EditScript,
// }
