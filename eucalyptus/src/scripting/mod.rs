pub mod entity;
pub mod math;
pub mod input;

use dropbear_engine::entity::{AdoptedEntity, Transform};
use hecs::World;
use rustyscript::{serde_json, Module, ModuleHandle, Runtime, RuntimeOptions};
use std::path::PathBuf;
use std::{collections::HashMap, fs};

use crate::states::{EntityNode, ModelProperties, ScriptComponent, PROJECT, SOURCE};

pub const TEMPLATE_SCRIPT: &'static str = include_str!("../template.ts");

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

pub fn move_script_to_src(script_path: &PathBuf) -> anyhow::Result<PathBuf> {
    let project_path = {
        let project = PROJECT.read().unwrap();
        project.project_path.clone()
    };

    let src_path = {
        let source_config = SOURCE.read().unwrap();
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
        let source_config = SOURCE.read().unwrap();
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

pub struct ScriptManager {
    pub runtime: Runtime,
    compiled_scripts: HashMap<String, ModuleHandle>,
    entity_script_data: HashMap<hecs::Entity, serde_json::Value>,
}

impl ScriptManager {
    pub fn new() -> anyhow::Result<Self> {
        let mut runtime = Runtime::new(RuntimeOptions::default())?;

        let dropbear_content = include_str!("../dropbear.ts");
        let dropbear_module = Module::new("./dropbear.ts", dropbear_content);
        runtime.load_module(&dropbear_module)?;
        log::debug!("Loaded dropbear module");

        // Register modules
        math::register_math_functions(&mut runtime)?;
        input::InputState::register_input_modules(&mut runtime)?;
        entity::register_model_props_module(&mut runtime)?;

        // Register utility functions
        runtime.register_function("log", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            let msg = args.get(0)
                .and_then(|v| v.as_str())
                .unwrap_or("undefined");
            println!("[Script] {}", msg);
            Ok(serde_json::Value::Null)
        })?;

        runtime.register_function("time", |_args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();
            Ok(serde_json::Value::Number(serde_json::Number::from_f64(time).unwrap()))
        })?;
        log::debug!("Initialised ScriptManager");
        Ok(Self {
            runtime,
            compiled_scripts: HashMap::new(),
            entity_script_data: HashMap::new(),
        })
    }

    pub fn init_entity_script(
        &mut self,
        entity_id: hecs::Entity,
        script_name: &str,
        world: &mut World,
        input_state: &input::InputState,
    ) -> anyhow::Result<()> {
        if let Ok(mut q) = world.query_one::<&AdoptedEntity>(entity_id) {
            if let Some(adopted) = q.get() {
                log_once::debug_once!(
                    "init_entity_script: '{}' for entity {:?} -> label='{}' path='{}'",
                    script_name,
                    entity_id,
                    adopted.label(),
                    adopted.model().path
                );
            }
        } else {
            log_once::debug_once!("init_entity_script: '{}' for entity {:?}", script_name, entity_id);
        }

        log::debug!("Init Entity Script module name: {}", script_name);
        if let Some(module) = self.compiled_scripts.get(script_name).cloned() {
            let mut script_data = serde_json::Map::new();

            // transform
            if let Ok(mut transform_query) = world.query_one::<&Transform>(entity_id) {
                if let Some(transform) = transform_query.get() {
                    script_data.insert("transform".to_string(), serde_json::to_value(transform)?);
                }
            }

            // entity props
            if let Ok(mut properties_query) = world.query_one::<&ModelProperties>(entity_id) {
                if let Some(properties) = properties_query.get() {
                    script_data.insert("entity".to_string(), serde_json::to_value(properties)?);
                } else {
                    let default_props = ModelProperties::default();
                    script_data.insert("entity".to_string(), serde_json::to_value(&default_props)?);
                }
            } else {
                let default_props = ModelProperties::default();
                script_data.insert("entity".to_string(), serde_json::to_value(&default_props)?);
            }

            // input state
            let serializable_input = input::SerializableInputState::from(input_state);
            script_data.insert("input".to_string(), serde_json::to_value(&serializable_input)?);

            // call onLoad
            let script_data_value = serde_json::Value::Object(script_data);
            match self.runtime.call_function::<serde_json::Value>(Some(&module), "onLoad", &vec![script_data_value.clone()]) {
                Ok(result) => {
                    log::debug!("Called onLoad for entity {:?}", entity_id);
                    
                    let updated_data = if result.is_object() {
                        result
                    } else {
                        // JIC script isn't returning updated entity
                        script_data_value
                    };
                    
                    self.apply_script_data_to_world(entity_id, &updated_data, world)?;
                    
                    self.entity_script_data.insert(entity_id, updated_data);
                }
                Err(e) => {
                    log::warn!("onLoad function not found or failed for entity {:?}: {}", entity_id, e);
                    self.entity_script_data.insert(entity_id, script_data_value);
                }
            }

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
        input_state: &input::InputState,
        dt: f32,
    ) -> anyhow::Result<()> {
        log_once::debug_once!("Update entity script name: {}", script_name);
        if let Some(module) = self.compiled_scripts.get(script_name).cloned() {
            let mut script_data = serde_json::Map::new();

            // transform
            if let Ok(mut transform_query) = world.query_one::<&Transform>(entity_id) {
                if let Some(transform) = transform_query.get() {
                    script_data.insert("transform".to_string(), serde_json::to_value(transform)?);
                }
            }

            // entity props
            if let Ok(mut properties_query) = world.query_one::<&ModelProperties>(entity_id) {
                if let Some(properties) = properties_query.get() {
                    script_data.insert("entity".to_string(), serde_json::to_value(properties)?);
                } else {
                    let default_props = ModelProperties::default();
                    script_data.insert("entity".to_string(), serde_json::to_value(&default_props)?);
                }
            } else {
                let default_props = ModelProperties::default();
                script_data.insert("entity".to_string(), serde_json::to_value(&default_props)?);
            }

            // input state
            let serializable_input = input::SerializableInputState::from(input_state);
            script_data.insert("input".to_string(), serde_json::to_value(&serializable_input)?);

            let script_data_value = serde_json::Value::Object(script_data);
            let dt_value = serde_json::Value::Number(serde_json::Number::from_f64(dt as f64).unwrap());
            
            match self.runtime.call_function::<serde_json::Value>(Some(&module), "onUpdate", &vec![script_data_value.clone(), dt_value]) {
                Ok(result) => {
                    log::trace!("Called update for entity {:?}", entity_id);
                    
                    if let Some(result_obj) = result.as_object() {
                        if let Some(transform_value) = result_obj.get("transform") {
                            if let Ok(new_transform) = serde_json::from_value::<Transform>(transform_value.clone()) {
                                if let Ok(mut transform_query) = world.query_one::<&mut Transform>(entity_id) {
                                    if let Some(transform) = transform_query.get() {
                                        *transform = new_transform;
                                        log::trace!("Updated transform for entity {:?}", entity_id);
                                    }
                                }
                            }
                        }

                        if let Some(entity_value) = result_obj.get("entity") {
                            if let Ok(new_properties) = serde_json::from_value::<ModelProperties>(entity_value.clone()) {
                                if let Ok(mut properties_query) = world.query_one::<&mut ModelProperties>(entity_id) {
                                    if let Some(properties) = properties_query.get() {
                                        *properties = new_properties;
                                        log::trace!("Updated properties for entity {:?}", entity_id);
                                    }
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    log_once::error_once!("Script execution error for entity {:?}: {}", entity_id, e);
                }
            }
        } else {
            log_once::error_once!("Unable to fetch compiled scripts for entity {:?}. Script Name: {}", entity_id, script_name);
        }
        Ok(())
    }

    fn apply_script_data_to_world(
        &self,
        entity_id: hecs::Entity,
        script_data: &serde_json::Value,
        world: &mut World,
    ) -> anyhow::Result<()> {
        if let Some(data_obj) = script_data.as_object() {
            // Update transform if it exists in the script data
            if let Some(transform_value) = data_obj.get("transform") {
                if let Ok(updated_transform) = serde_json::from_value::<Transform>(transform_value.clone()) {
                    if let Ok(mut transform_query) = world.query_one::<&mut Transform>(entity_id) {
                        if let Some(transform) = transform_query.get() {
                            *transform = updated_transform;
                            log::trace!("Updated transform for entity {:?}", entity_id);
                        }
                    }
                }
            }

            if let Some(entity_value) = data_obj.get("entity") {
                if let Ok(updated_properties) = serde_json::from_value::<ModelProperties>(entity_value.clone()) {
                    if let Ok(mut properties_query) = world.query_one::<&mut ModelProperties>(entity_id) {
                        if let Some(properties) = properties_query.get() {
                            *properties = updated_properties;
                            log::trace!("Updated properties for entity {:?}", entity_id);
                        }
                    } else {
                        if let Err(e) = world.insert_one(entity_id, updated_properties) {
                            log::warn!("Failed to insert updated properties for entity {:?}: {}", entity_id, e);
                        }
                    }
                }
            }
            // input state doesn't get updated, it is only read. 
        }

        Ok(())
    }

    pub fn load_script_from_source(&mut self, script_name: &String, script_content: &String) -> anyhow::Result<String> {
        let module = Module::new(&script_name, script_content);
        
        match self.runtime.load_module(&module) {
            Ok(module) => {
                self.compiled_scripts.insert(script_name.clone(), module);
                log::info!("Loaded script: {}", script_name);
                Ok(script_name.to_string())
            }
            Err(e) => {
                log::error!("Compiling module for script [{}] returned an error: {}", script_name, e);
                Err(e.into())
            }
        }
    }

    pub fn load_script(&mut self, script_path: &PathBuf) -> anyhow::Result<String> {
        log::debug!("Reading script content");
        let script_content = fs::read_to_string(script_path)?;
        log::debug!("Fetching script name");
        let script_name = script_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        log::debug!("Script name: {}", script_name);

        log::debug!("Creating module for typescript runtime");
        let module = Module::new(&script_name, &script_content);
        
        log::debug!("Loading module");
        match self.runtime.load_module(&module) {
            Ok(module) => {
                self.compiled_scripts.insert(script_name.clone(), module);
                log::info!("Loaded script: {}", script_name);
                Ok(script_name)
            }
            Err(e) => {
                log::error!("Compiling module for script path [{}] returned an error: {}", script_path.display(), e);
                Err(e.into())
            }
        }
    }

    pub fn remove_entity_script(&mut self, entity_id: hecs::Entity) {
        self.entity_script_data.remove(&entity_id);
    }
}