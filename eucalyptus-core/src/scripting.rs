pub mod dropbear;

use crate::input::InputState;
use crate::scripting::dropbear::DropbearAPI;
use crate::states::{EntityNode, PROJECT, SOURCE, ScriptComponent, Value};
use dropbear_engine::entity::{AdoptedEntity, Transform};
use hecs::{Entity, World};
use wasmer::{FunctionEnv, Imports, Instance, Module, Store, imports};
use std::path::PathBuf;
use std::sync::Arc;
use std::{collections::HashMap, fs};

/// A trait that describes a module that can be registered.
pub trait ScriptableModule {
    type Data;

    fn register(data: &Self::Data, imports: &mut Imports, store: &mut Store) -> anyhow::Result<()>;

    fn module_name() -> &'static str;
}

pub trait ScriptableModuleWithEnv {
    type T;

    fn register(env: &FunctionEnv<Self::T>, imports: &mut Imports, store: &mut Store) -> anyhow::Result<()>;

    fn module_name() -> &'static str;
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

#[derive(Clone)]
pub struct DropbearScriptingAPIContext {
    pub current_entity: Option<Entity>,
    current_world: Option<Arc<World>>,
    pub current_input: Option<InputState>,
    pub persistent_data: HashMap<String, Value>,
    pub frame_data: HashMap<String, Value>,
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

    pub fn set_context(&mut self, entity: Entity, world: Arc<World>, input: &InputState) {
        self.current_entity = Some(entity);
        self.current_world = Some(world);
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
        self.persistent_data.retain(|k, _| !k.starts_with(&entity_prefix));
    }
}

pub struct ScriptManager {
    pub store: Store,
    compiled_scripts: HashMap<String, Module>,
    entity_script_data: HashMap<hecs::Entity, u32>,
    script_context: DropbearScriptingAPIContext,
}

impl ScriptManager {
    pub fn new() -> anyhow::Result<Self> {
        let store = Store::default();
        
        let result = Self {
            store,
            compiled_scripts: HashMap::new(),
            entity_script_data: HashMap::new(),
            script_context: DropbearScriptingAPIContext::new(),
        };

        log::debug!("Initialised ScriptManager");
        Ok(result)
    }

    pub fn load_script(&mut self, script_name: &String, script_content: impl AsRef<[u8]>) -> anyhow::Result<String> {
        let module = Module::new(self.store.engine(), script_content)?;
        self.compiled_scripts.insert(script_name.clone(), module);
        log::debug!("Loaded script [{}]", script_name);
        Ok(script_name.clone())
    }

    pub fn init_entity_script(
        &mut self,
        entity_id: hecs::Entity,
        script_name: &str,
        world: &mut Arc<World>,
        input_state: &InputState
    ) -> anyhow::Result<()> {
        log_once::debug_once!("init_entity_script: {} for {:?}", script_name, entity_id);

        if let Some(module) = self.compiled_scripts.get(script_name).cloned() {
            self.script_context.set_context(entity_id, world.clone(), input_state);

            let import_obj = self.create_imports()?;
            let instance = Instance::new(&mut self.store, &module, &import_obj)?;

            if let Ok(alloc_func) = instance.exports.get_function("__alloc") {
                let size = std::mem::size_of::<Transform>() as i32;
                let result = alloc_func.call(&mut self.store, &[size.into()])?;
                if let Some(wasmer::Value::I32(ptr)) = result.get(0) {
                    self.entity_script_data.insert(entity_id, *ptr as u32);
                }
            }

            self.sync_entity_to_memory(entity_id, world, &instance)?;

            if let Ok(init_func) = instance.exports.get_function("init") {
                init_func.call(&mut self.store, &[])?;
            }

            self.sync_memory_to_entity(entity_id, world, &instance)?;

            self.script_context.clear_context();

            Ok(())
        } else {
            Err(anyhow::anyhow!("Script '{}' not found", script_name))
        }
    }

    pub fn update_entity_script(
        &mut self,
        entity_id: hecs::Entity,
        script_name: &str,
        world: &mut Arc<World>,
        input_state: &InputState,
        dt: f32,
    ) -> anyhow::Result<()> {
        log_once::debug_once!("Update entity script name: {}", script_name);

        if let Some(module) = self.compiled_scripts.get(script_name).cloned() {
            self.script_context.set_context(entity_id, world.clone(), input_state);
            
            let import_object = self.create_imports()?;
            let instance = Instance::new(&mut self.store, &module, &import_object)?;
            
            self.sync_entity_to_memory(entity_id, world, &instance)?;
            
            if let Ok(update_func) = instance.exports.get_function("update") {
                let dt_value = wasmer::Value::F32(dt);
                update_func.call(&mut self.store, &[dt_value])?;
            }
            
            self.sync_memory_to_entity(entity_id, world, &instance)?;
            
            self.script_context.clear_context();
            
        } else {
            log_once::error_once!("Unable to fetch compiled scripts for entity {:?}. Script Name: {}", entity_id, script_name);
        }
        Ok(())
    }

    pub fn remove_entity_script(&mut self, entity_id: hecs::Entity) {
        self.entity_script_data.remove(&entity_id);
    }

    fn sync_entity_to_memory(&self, entity_id: hecs::Entity, world: &mut Arc<World>, instance: &Instance) -> anyhow::Result<()> {
        if let Some(&memory_offset) = self.entity_script_data.get(&entity_id) {
            let memory = instance.exports.get_memory("memory")?;
            let memory_view = memory.view(&self.store);

            if let Ok(transform) = Arc::get_mut(world).unwrap().query_one_mut::<&Transform>(entity_id) {
                let transform_bytes = unsafe { std::slice::from_raw_parts(
                    transform as *const Transform as *const u8, 
                    std::mem::size_of::<Transform>()
                )};
                
                for (i, &byte) in transform_bytes.iter().enumerate() {
                    memory_view.write_u8((memory_offset + i as u32).into(), byte)?;
                }
            }
        }
        Ok(())
    }

    fn sync_memory_to_entity(&self, entity_id: hecs::Entity, world: &mut Arc<World>, instance: &Instance) -> anyhow::Result<()> {
        if let Some(&memory_offset) = self.entity_script_data.get(&entity_id) {
            let memory = instance.exports.get_memory("memory")?;
            let memory_view = memory.view(&self.store);

            Self::update::<Transform>(memory_offset, &memory_view, world, &entity_id)?;
        }
        Ok(())
    }

    fn update<T: Send + Sync + 'static>(memory_offset: u32, memory_view: &wasmer::MemoryView<'_>, world: &mut Arc<World>, entity_id: &hecs::Entity) -> anyhow::Result<()> {
        let mut obj_bytes = vec![0u8; std::mem::size_of::<T>()];
        for (i, byte) in obj_bytes.iter_mut().enumerate() {
            *byte = memory_view.read_u8((memory_offset + i as u32).into())?;
        }

        let obj = unsafe {
            std::ptr::read(obj_bytes.as_ptr() as *const T)
        };

        Arc::get_mut(world).unwrap().insert_one(*entity_id, obj)?;
        Ok(())
    }

    fn create_imports(&mut self) -> anyhow::Result<Imports> {
        let mut imports = imports! {};

        DropbearAPI::register(&self.script_context, &mut imports, &mut self.store)?;

        Ok(imports)
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
