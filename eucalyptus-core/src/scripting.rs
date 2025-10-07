mod kmp;
mod jni;

use crate::input::InputState;
use crate::states::{EntityNode, PROJECT, SOURCE, ScriptComponent, Value};
use dropbear_engine::entity::{AdoptedEntity, Transform};
use hecs::{Entity, World};
use std::path::PathBuf;
use std::{collections::HashMap, fs};
use std::sync::LazyLock;
use crate::ptr::SafePointer;

pub const TEMPLATE_SCRIPT: &str = include_str!("../../resources/scripting/kotlin/Template.kt");

static CONTEXT: LazyLock<DropbearScriptingAPIContext> = LazyLock::new(|| DropbearScriptingAPIContext::new());

pub struct DropbearScriptingAPIContext {
    pub current_entity: Option<Entity>,
    // fyi: im pretty sure this is safe because I can just null with [`Option::None`]
    current_world: Option<SafePointer<World>>,
    pub current_input: Option<InputState>,
    pub persistent_data: HashMap<String, Value>,
    pub frame_data: HashMap<String, Value>,
}

impl Default for DropbearScriptingAPIContext {
    fn default() -> Self {
        Self::new()
    }
}

impl DropbearScriptingAPIContext {
    pub fn new() -> Self {
        Self {
            current_entity: None,
            current_world: None,
            current_input: None,
            persistent_data: HashMap::new(),
            frame_data: HashMap::new(),
        }
    }

    pub fn set_context(&mut self, entity: Entity, world: &mut World, input: &InputState) {
        self.current_entity = Some(entity);
        self.current_world = Some(SafePointer::new(world));
        self.current_input = Some(input.clone());
    }

    pub fn clear_context(&mut self) {
        self.current_entity = None;
        self.current_world = None;
        self.current_input = None;
        self.frame_data.clear();
    }

    pub fn get_current_entity(&self) -> Option<Entity> {
        self.current_entity
    }

    pub fn get_input(&self) -> Option<&InputState> {
        self.current_input.as_ref()
    }

    pub fn set_persistent_data(&mut self, key: String, value: Value) {
        self.persistent_data.insert(key, value);
    }

    pub fn get_persistent_data(&self, key: &str) -> Option<&Value> {
        self.persistent_data.get(key)
    }

    pub fn set_frame_data(&mut self, key: String, value: Value) {
        self.frame_data.insert(key, value);
    }

    pub fn get_frame_data(&self, key: &str) -> Option<&Value> {
        self.frame_data.get(key)
    }

    pub fn cleanup_entity_data(&mut self, entity: Entity) {
        let entity_prefix = format!("entity_{:?}_", entity);
        self.persistent_data
            .retain(|k, _| !k.starts_with(&entity_prefix));
    }
}

pub struct ScriptManager;

impl ScriptManager {
    pub fn new() -> anyhow::Result<Self> {
        Err(anyhow::anyhow!("it aint ready yet bozo"))
    }

    pub fn load_script(
        &mut self,
    ) -> anyhow::Result<()> {
        
        Ok(())
    }

    pub fn init_entity_script(
        &mut self,
        entity_id: hecs::Entity,
        tags: &Vec<String>,
        world: &mut World,
        input_state: &InputState,
    ) -> anyhow::Result<()> {

        Err(anyhow::anyhow!("it aint ready yet bozo"))
    }

    pub fn update_entity_script(
        &mut self,
        entity_id: hecs::Entity,
        tags: &Vec<String>,
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
