use dropbear_engine::camera::Camera;
use dropbear_engine::entity::{AdoptedEntity, Transform};
use dropbear_engine::lighting::{Light, LightComponent};
use glam::DVec3;
use hecs::World;
use rustyscript::{serde_json, Module, ModuleHandle, Runtime, RuntimeOptions};
use std::path::PathBuf;
use std::{collections::HashMap, fs};
use crate::camera::CameraComponent;
use crate::input::InputState;
use crate::states::{EntityNode, ModelProperties, ScriptComponent, PROJECT, SOURCE};

/// A trait that describes a module that can be registered. 
pub trait ScriptableModule {
    /// Registers the functions for the dropbear typescript API
    fn register(runtime: &mut Runtime) -> anyhow::Result<()>;
    // /// Gathers the information into a serializable format that can be sent over to the 
    // /// dropbear typescript API
    // fn gather(world: &World, entity_id: hecs::Entity);
    // /// Fetches the mutated information from the dropbear typescript module and applys it to the world
    // fn release(world: &mut World, entity_id: hecs::Entity, data: &serde_json::Value) -> anyhow::Result<()>;
}

pub const TEMPLATE_SCRIPT: &'static str = include_str!("../../resources/template.ts");

pub enum ScriptAction {
    AttachScript {
        script_path: PathBuf,
        script_name: String,
    },
    CreateAndAttachScript {
        script_path: PathBuf,
        script_name: String,
    },
    RemoveScript,
    EditScript,
}

pub struct ScriptManager {
    pub runtime: Runtime,
    compiled_scripts: HashMap<String, ModuleHandle>,
    entity_script_data: HashMap<hecs::Entity, serde_json::Value>,
}

impl ScriptManager {
    pub fn new() -> anyhow::Result<Self> {
        todo!();
    }

    pub fn load_script_from_source(&mut self, script_name: &String, script_content: &String) -> anyhow::Result<String> {
        todo!();
    }

    pub fn load_script(&mut self, script_path: &PathBuf) -> anyhow::Result<String> {
        todo!();
    }

    pub fn init_entity_script(
        &mut self,
        entity_id: hecs::Entity,
        script_name: &str,
        world: &mut World,
        input_state: &InputState,
    ) -> anyhow::Result<()> {
        log_once::debug_once!("init_entity_script: {} for {:?}", script_name, entity_id);

        if let Some(module) = self.compiled_scripts.get(script_name).cloned() {
            todo!();
            Ok(())
        } else {
            Err(anyhow::anyhow!("Script '{}' not found", script_name))
        }
    }

    pub fn update_entity_script(
        &mut self,
        entity_id: hecs::Entity,
        script_name: &str,
        world: &mut World,
        input_state: &InputState,
        dt: f32,
    ) -> anyhow::Result<()> {
        log_once::debug_once!("Update entity script name: {}", script_name);

        if let Some(module) = self.compiled_scripts.get(script_name).cloned() {
            todo!();
        } else {
            log_once::error_once!("Unable to fetch compiled scripts for entity {:?}. Script Name: {}", entity_id, script_name);
        }
        Ok(())
    }

    pub fn remove_entity_script(&mut self, entity_id: hecs::Entity) {
        self.entity_script_data.remove(&entity_id);
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
    world: &World,
    entity_id: hecs::Entity,
) -> anyhow::Result<EntityNode> {
    if let Ok(mut query) = world.query_one::<(&AdoptedEntity, &Transform)>(entity_id) {
        if let Some((adopted, _transform)) = query.get() {
            let entity_name = adopted.model().label.clone();

            let script_node = if let Ok(script) = world.get::<&ScriptComponent>(entity_id) {
                Some(EntityNode::Script {
                    name: script.name.clone(),
                    path: script.path.clone(),
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
    if let Err(e) = world.insert_one(entity_id, script_component) {
        return Err(anyhow::anyhow!("Failed to attach script to entity: {}", e));
    }

    log::info!("Successfully attached script to entity {:?}", entity_id);
    Ok(())
}