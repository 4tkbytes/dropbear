pub mod kmp;
pub mod jni;

use crate::input::InputState;
use crate::states::{EntityNode, PROJECT, SOURCE, ScriptComponent, Value};
use dropbear_engine::entity::{AdoptedEntity, Transform};
use hecs::{Entity, World};
use std::path::{Path, PathBuf};
use std::{collections::HashMap, fs};
use std::ffi::OsString;
use std::sync::{Arc, LazyLock};
use libloading::Library;
use crate::scripting::jni::JavaContext;

pub const TEMPLATE_SCRIPT: &str = include_str!("../../resources/scripting/kotlin/Template.kt");

#[derive(Default)]
pub enum ScriptTarget {
    #[default]
    None,
    JVM { library_path: PathBuf },
    Native { library_path: PathBuf },
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

    pub fn init_script(
        &mut self,
        entity_tag_database: HashMap<String, Vec<Entity>>,
        target: ScriptTarget,
    ) -> anyhow::Result<()> {
        self.entity_tag_database = entity_tag_database;

        match target {
            ScriptTarget::JVM { library_path } => {
                let jvm = JavaContext::new(library_path)?;
                self.jvm = Some(jvm);
            }
            ScriptTarget::Native { library_path } => {
                let library = unsafe {
                    Library::new(library_path)?
                };

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
