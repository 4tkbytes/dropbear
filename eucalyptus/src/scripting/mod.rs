pub mod camera;
pub mod entity;
pub mod math;
pub mod input;
pub mod lighting;

use dropbear_engine::camera::Camera;
use dropbear_engine::entity::{AdoptedEntity, Transform};
use dropbear_engine::lighting::{Light, LightComponent};
use glam::DVec3;
use hecs::World;
use rustyscript::{serde_json, Module, ModuleHandle, Runtime, RuntimeOptions};
use std::path::PathBuf;
use std::{collections::HashMap, fs};

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

use crate::camera::CameraComponent;
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

        let mut result = Self {
            runtime,
            compiled_scripts: HashMap::new(),
            entity_script_data: HashMap::new(),
        };

        result = result
            .register_module::<input::InputState>()?
            .register_module::<math::Math>()?
            .register_module::<ModelProperties>()?
            .register_module::<camera::SerializableCamera>()?
            .register_module::<lighting::Lighting>()?;

        // Register utility functions
        result.runtime.register_function("log", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            let msg = args.get(0)
                .and_then(|v| v.as_str())
                .unwrap_or("undefined");
            println!("[Script] {}", msg);
            Ok(serde_json::Value::Null)
        })?;

        result.runtime.register_function("time", |_args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            let time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64();
            Ok(serde_json::Value::Number(serde_json::Number::from_f64(time).unwrap()))
        })?;
        log::debug!("Initialised ScriptManager");
        Ok(result)
    }

    /// Allows for the registering of other modules to use with the dropbear typescript
    /// API
    /// 
    /// # Parameters
    /// * A turbofish to a struct that uses the ScriptableModule trait
    /// 
    /// # Examples
    /// ```rust
    /// let script_manager = ScriptManager::new()?
    ///     .register_module::<input::InputState>()?
    ///     .register_module::<camera::Camera>()?
    ///     .register_module::<lighting::LightingModule>()?;
    /// ```
    pub fn register_module<T: ScriptableModule>(mut self) -> anyhow::Result<Self> {
        T::register(&mut self.runtime)?;
        Ok(self)
    }

    pub fn init_entity_script(
        &mut self,
        entity_id: hecs::Entity,
        script_name: &str,
        world: &mut World,
        input_state: &input::InputState,
    ) -> anyhow::Result<()> {
        log_once::debug_once!("init_entity_script: {} for {:?}", script_name, entity_id);

        if let Some(module) = self.compiled_scripts.get(script_name).cloned() {
            // construct RawSceneData-like payload
            let mut scene_map = serde_json::Map::new();

            // entities array
            let mut entities_arr: Vec<serde_json::Value> = Vec::new();
            for (id, (adopted, transform)) in world.query::<(&AdoptedEntity, &Transform)>().iter() {
                // try to get ModelProperties for this entity
                let props_value = if let Ok(mut pq) = world.query_one::<&ModelProperties>(id) {
                    if let Some(p) = pq.get() {
                        serde_json::to_value(p)?
                    } else {
                        serde_json::Value::Object(serde_json::Map::new())
                    }
                } else {
                    serde_json::Value::Object(serde_json::Map::new())
                };

                let mut ent_obj = serde_json::Map::new();
                ent_obj.insert("label".to_string(), serde_json::Value::String(adopted.label().clone()));

                // properties object expected shape: { custom_properties: Record<string, any> } or full ModelProperties
                ent_obj.insert("properties".to_string(), props_value);

                ent_obj.insert("transform".to_string(), serde_json::to_value(transform)?);

                entities_arr.push(serde_json::Value::Object(ent_obj));
            }
            scene_map.insert("entities".to_string(), serde_json::Value::Array(entities_arr));

            // cameras snapshot (keep minimal fields)
            let mut cameras_arr: Vec<serde_json::Value> = Vec::new();
            for (_id, (cam, _comp)) in world.query::<(&Camera, &CameraComponent)>().iter() {
                cameras_arr.push(serde_json::json!({
                    "label": cam.label,
                    "data": {
                        "eye": cam.eye.to_array(),
                        "target": cam.target.to_array(),
                        "up": cam.up.to_array(),
                        "aspect": cam.aspect,
                        "fov": cam.fov_y,
                        "near": cam.znear,
                        "far": cam.zfar,
                        "yaw": cam.yaw,
                        "pitch": cam.pitch,
                        "speed": cam.speed,
                        "sensitivity": cam.sensitivity
                    }
                }));
            }
            scene_map.insert("cameras".to_string(), serde_json::Value::Array(cameras_arr));

            // lights snapshot (label only)
            let mut lights_arr: Vec<serde_json::Value> = Vec::new();
            for (_id, (_transform, _light_comp, light)) in world.query::<(&Transform, &LightComponent, &Light)>().iter() {
                lights_arr.push(serde_json::json!({
                    "label": light.label(),
                }));
            }
            scene_map.insert("lights".to_string(), serde_json::Value::Array(lights_arr));

            // current entity label (if available)
            if let Ok(mut q) = world.query_one::<&AdoptedEntity>(entity_id) {
                if let Some(adopted) = q.get() {
                    scene_map.insert("current_entity".to_string(), serde_json::Value::String(adopted.label().clone()));
                }
            }

            // input state
            let serializable_input = input::SerializableInputState::from(input_state);
            let mut payload = serde_json::Map::new();
            payload.insert("scene".to_string(), serde_json::Value::Object(scene_map));
            payload.insert("input".to_string(), serde_json::to_value(&serializable_input)?);

            let script_data_value = serde_json::Value::Object(payload);

            // call onLoad (module scripts call dropbear.start(s) and return dropbear.end())
            match self.runtime.call_function::<serde_json::Value>(Some(&module), "onLoad", &vec![script_data_value.clone()]) {
                Ok(result) => {
                    log::debug!("Called onLoad for entity {:?}", entity_id);
                    // apply whatever the script returned (expect Partial RawSceneData shape)
                    let _ = self.apply_script_data_to_world(entity_id, &result, world);
                    // store initial data (script-managed)
                    self.entity_script_data.insert(entity_id, result);
                }
                Err(e) => {
                    log::warn!("onLoad missing/failed for entity {:?}: {}", entity_id, e);
                    // store the payload we sent so future updates have context
                    self.entity_script_data.insert(entity_id, script_data_value);
                }
            }

            Ok(())
        } else {
            Err(anyhow::anyhow!("Script '{}' not found", script_name))
        }
    }
// ...existing code...

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
            // Build the same RawSceneData-like payload as init
            let mut scene_map = serde_json::Map::new();

            let mut entities_arr: Vec<serde_json::Value> = Vec::new();
            for (id, (adopted, transform)) in world.query::<(&AdoptedEntity, &Transform)>().iter() {
                let props_value = if let Ok(mut pq) = world.query_one::<&ModelProperties>(id) {
                    if let Some(p) = pq.get() {
                        serde_json::to_value(p)?
                    } else {
                        serde_json::Value::Object(serde_json::Map::new())
                    }
                } else {
                    serde_json::Value::Object(serde_json::Map::new())
                };

                let mut ent_obj = serde_json::Map::new();
                ent_obj.insert("label".to_string(), serde_json::Value::String(adopted.label().clone()));
                ent_obj.insert("properties".to_string(), props_value);
                ent_obj.insert("transform".to_string(), serde_json::to_value(transform)?);
                entities_arr.push(serde_json::Value::Object(ent_obj));
            }
            scene_map.insert("entities".to_string(), serde_json::Value::Array(entities_arr));

            // cameras
            let mut cameras_arr: Vec<serde_json::Value> = Vec::new();
            for (_id, (cam, _comp)) in world.query::<(&Camera, &CameraComponent)>().iter() {
                cameras_arr.push(serde_json::json!({
                    "label": cam.label,
                    "data": {
                        "eye": cam.eye.to_array(),
                        "target": cam.target.to_array(),
                        "up": cam.up.to_array(),
                        "aspect": cam.aspect,
                        "fov": cam.fov_y,
                        "near": cam.znear,
                        "far": cam.zfar,
                        "yaw": cam.yaw,
                        "pitch": cam.pitch,
                        "speed": cam.speed,
                        "sensitivity": cam.sensitivity
                    }
                }));
            }
            scene_map.insert("cameras".to_string(), serde_json::Value::Array(cameras_arr));

            // lights
            let mut lights_arr: Vec<serde_json::Value> = Vec::new();
            for (_id, (_transform, _light_comp, light)) in world.query::<(&Transform, &LightComponent, &Light)>().iter() {
                lights_arr.push(serde_json::json!({
                    "label": light.label(),
                }));
            }
            scene_map.insert("lights".to_string(), serde_json::Value::Array(lights_arr));

            // current entity label
            if let Ok(mut q) = world.query_one::<&AdoptedEntity>(entity_id) {
                if let Some(adopted) = q.get() {
                    scene_map.insert("current_entity".to_string(), serde_json::Value::String(adopted.label().clone()));
                }
            }

            let serializable_input = input::SerializableInputState::from(input_state);
            let mut payload = serde_json::Map::new();
            payload.insert("scene".to_string(), serde_json::Value::Object(scene_map));
            payload.insert("input".to_string(), serde_json::to_value(&serializable_input)?);

            let script_data_value = serde_json::Value::Object(payload);
            let dt_value = serde_json::Value::Number(serde_json::Number::from_f64(dt as f64).unwrap());

            // call onUpdate(s, dt)
            match self.runtime.call_function::<serde_json::Value>(Some(&module), "onUpdate", &vec![script_data_value.clone(), dt_value]) {
                Ok(result) => {
                    log::trace!("Called update for entity {:?}", entity_id);
                    // delegate applying of returned data to the helper
                    let _ = self.apply_script_data_to_world(entity_id, &result, world);
                    // store latest returned data
                    self.entity_script_data.insert(entity_id, result);
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
        // Accept either:
        //  - Partial RawSceneData { entities: [...] } where each entity is { label, properties, transform }
        //  - Or a direct object for a single entity containing "transform" and/or "properties"
        if let Some(obj) = script_data.as_object() {
            // If there is an "entities" array, apply each entry by matching labels
            if let Some(entities_val) = obj.get("entities").and_then(|v| v.as_array()) {
                for ent in entities_val {
                    if let Some(ent_obj) = ent.as_object() {
                        if let Some(label) = ent_obj.get("label").and_then(|l| l.as_str()) {
                            // find entity in world by label
                            if let Some(target_entity) = world
                                .query_mut::<(&AdoptedEntity,)>()
                                .into_iter()
                                .find_map(|(e, (adopted,))| {
                                    if adopted.label() == label {
                                        Some(e)
                                    } else {
                                        None
                                    }
                                }) {
                                // apply transform if present
                                if let Some(transform_val) = ent_obj.get("transform") {
                                    if let Ok(new_t) = serde_json::from_value::<Transform>(transform_val.clone()) {
                                        if let Ok(mut tq) = world.query_one::<&mut Transform>(target_entity) {
                                            if let Some(tref) = tq.get() {
                                                *tref = new_t;
                                                log::trace!("apply: updated transform for {:?}", target_entity);
                                            }
                                        }
                                    } else {
                                        log::trace!("apply: failed to deserialize transform for label '{}'", label);
                                    }
                                }
                                // apply properties if present
                                if let Some(props_val) = ent_obj.get("properties") {
                                    if let Ok(new_props) = serde_json::from_value::<ModelProperties>(props_val.clone()) {
                                        if let Ok(mut pq) = world.query_one::<&mut ModelProperties>(target_entity) {
                                            if let Some(p) = pq.get() {
                                                *p = new_props;
                                                log::trace!("apply: updated properties for {:?}", target_entity);
                                            }
                                        } else {
                                            if let Err(e) = world.insert_one(target_entity, new_props) {
                                                log::warn!("apply: failed to insert properties for {:?}: {}", target_entity, e);
                                            }
                                        }
                                    } else {
                                        log::trace!("apply: failed to deserialize properties for label '{}'", label);
                                    }
                                }
                            } else {
                                log::trace!("apply: no entity in world matching label '{}'", label);
                            }
                        }
                    }
                }
            } else {
                // No entities array: treat the object as direct payload for the single entity
                // apply transform if present
                if let Some(transform_value) = obj.get("transform") {
                    if let Ok(updated_transform) = serde_json::from_value::<Transform>(transform_value.clone()) {
                        if let Ok(mut transform_query) = world.query_one::<&mut Transform>(entity_id) {
                            if let Some(transform) = transform_query.get() {
                                *transform = updated_transform;
                                log::trace!("apply: Updated transform for entity {:?}", entity_id);
                            }
                        }
                    }
                }
                // apply properties if present (support "properties" or legacy "entity")
                if let Some(props_value) = obj.get("properties").or_else(|| obj.get("entity")) {
                    if let Ok(updated_properties) = serde_json::from_value::<ModelProperties>(props_value.clone()) {
                        if let Ok(mut properties_query) = world.query_one::<&mut ModelProperties>(entity_id) {
                            if let Some(properties) = properties_query.get() {
                                *properties = updated_properties;
                                log::trace!("apply: Updated properties for entity {:?}", entity_id);
                            }
                        } else {
                            if let Err(e) = world.insert_one(entity_id, updated_properties) {
                                log::warn!("apply: Failed to insert updated properties for entity {:?}: {}", entity_id, e);
                            }
                        }
                    }
                }

                // cameras: if script returned camera array with full data, apply by label
                if let Some(cameras_val) = obj.get("cameras").and_then(|v| v.as_array()) {
                    for cam in cameras_val {
                        if let Some(cam_obj) = cam.as_object() {
                            if let Some(label) = cam_obj.get("label").and_then(|l| l.as_str()) {
                                if let Some(data_obj) = cam_obj.get("data").and_then(|d| d.as_object()) {
                                    // find camera entity by label
                                    if let Some(cam_entity) = world
                                        .query::<&Camera>()
                                        .iter()
                                        .find_map(|(e, camera)| if camera.label == label { Some(e) } else { None }) {
                                        if let Ok(mut cam_q) = world.query_one::<&mut Camera>(cam_entity) {
                                            if let Some(cam_struct) = cam_q.get() {
                                                // update common camera fields if present
                                                if let Some(eye_val) = data_obj.get("eye") {
                                                    if let Ok(arr) = serde_json::from_value::<[f64; 3]>(eye_val.clone()) {
                                                        cam_struct.eye = DVec3::from_array(arr);
                                                    }
                                                }
                                                if let Some(target_val) = data_obj.get("target") {
                                                    if let Ok(arr) = serde_json::from_value::<[f64; 3]>(target_val.clone()) {
                                                        cam_struct.target = DVec3::from_array(arr);
                                                    }
                                                }
                                                if let Some(up_val) = data_obj.get("up") {
                                                    if let Ok(arr) = serde_json::from_value::<[f64; 3]>(up_val.clone()) {
                                                        cam_struct.up = DVec3::from_array(arr);
                                                    }
                                                }
                                                if let Some(aspect) = data_obj.get("aspect").and_then(|v| v.as_f64()) {
                                                    cam_struct.aspect = aspect;
                                                }
                                                if let Some(fov) = data_obj.get("fov").and_then(|v| v.as_f64()) {
                                                    cam_struct.fov_y = fov;
                                                }
                                                if let Some(near) = data_obj.get("near").and_then(|v| v.as_f64()) {
                                                    cam_struct.znear = near;
                                                }
                                                if let Some(far) = data_obj.get("far").and_then(|v| v.as_f64()) {
                                                    cam_struct.zfar = far;
                                                }
                                                if let Some(yaw) = data_obj.get("yaw").and_then(|v| v.as_f64()) {
                                                    cam_struct.yaw = yaw;
                                                }
                                                if let Some(pitch) = data_obj.get("pitch").and_then(|v| v.as_f64()) {
                                                    cam_struct.pitch = pitch;
                                                }
                                                if let Some(speed) = data_obj.get("speed").and_then(|v| v.as_f64()) {
                                                    cam_struct.speed = speed;
                                                }
                                                if let Some(sensitivity) = data_obj.get("sensitivity").and_then(|v| v.as_f64()) {
                                                    cam_struct.sensitivity = sensitivity;
                                                }
                                                log::trace!("apply: Updated camera '{}'", label);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        } else {
            log::trace!("apply: script_data not an object for entity {:?}", entity_id);
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

    pub fn handle_camera_actions(&self, world: &mut hecs::World, active_camera: &mut Option<hecs::Entity>, actions: Vec<serde_json::Value>) -> anyhow::Result<()> {
        use crate::camera::{CameraComponent, CameraType};
        use dropbear_engine::camera::Camera;

        for action_value in actions {
            if let Ok(action) = serde_json::from_value::<crate::scripting::camera::CameraAction>(action_value) {
                match action.action.as_str() {
                    "switch_camera" => {
                        if let Some(label) = action.label {
                            let camera_entity = world
                                .query::<&Camera>()
                                .iter()
                                .find(|(_, camera)| camera.label == label)
                                .map(|(entity, _)| entity);

                            if let Some(entity) = camera_entity {
                                *active_camera = Some(entity);
                                log::info!("Switched to camera with label: {}", label);
                            } else {
                                log::warn!("Camera with label '{}' not found", label);
                            }
                        }
                    }
                    "get_active_camera" => {
                        if let Some(active_entity) = active_camera {
                            if let Ok(camera) = world.get::<&Camera>(*active_entity) {
                                log::info!("Active camera label: {}", camera.label);
                            }
                        }
                    }
                    "get_all_cameras" => {
                        let labels: Vec<String> = world
                            .query::<&Camera>()
                            .iter()
                            .map(|(_, camera)| camera.label.clone())
                            .collect();
                        log::info!("All camera labels: {:?}", labels);
                    }
                    "get_cameras_by_type" => {
                        if let Some(type_str) = action.camera_type {
                            let target_type = match type_str.as_str() {
                                "Normal" => CameraType::Normal,
                                "Debug" => CameraType::Debug,
                                "Player" => CameraType::Player,
                                _ => continue,
                            };

                            let labels: Vec<String> = world
                                .query::<(&Camera, &CameraComponent)>()
                                .iter()
                                .filter_map(|(_, (camera, component))| {
                                    if component.camera_type == target_type {
                                        Some(camera.label.clone())
                                    } else {
                                        None
                                    }
                                })
                                .collect();
                            log::info!("Cameras of type {:?}: {:?}", target_type, labels);
                        }
                    }
                    "manipulate_camera" => {
                        if let (Some(label), Some(property), Some(value)) = (action.label, action.property, action.value) {
                            let camera_entity = world
                                .query::<&Camera>()
                                .iter()
                                .find(|(_, camera)| camera.label == label)
                                .map(|(entity, _)| entity);

                            if let Some(entity) = camera_entity {
                                match property.as_str() {
                                    "position" => {
                                        if let Ok(pos_array) = serde_json::from_value::<[f64; 3]>(value) {
                                            if let Ok(mut camera) = world.get::<&mut Camera>(entity) {
                                                camera.eye = DVec3::from_array(pos_array);
                                                log::info!("Updated camera '{}' position to {:?}", label, pos_array);
                                            }
                                        }
                                    }
                                    "target" => {
                                        if let Ok(target_array) = serde_json::from_value::<[f64; 3]>(value) {
                                            if let Ok(mut camera) = world.get::<&mut Camera>(entity) {
                                                camera.target = DVec3::from_array(target_array);
                                                log::info!("Updated camera '{}' target to {:?}", label, target_array);
                                            }
                                        }
                                    }
                                    "speed" => {
                                        if let Some(speed) = value.as_f64() {
                                            if let Ok(mut camera) = world.get::<&mut Camera>(entity) {
                                                camera.speed = speed;
                                            }
                                            if let Ok(mut component) = world.get::<&mut CameraComponent>(entity) {
                                                component.speed = speed;
                                            }
                                            log::info!("Updated camera '{}' speed to {}", label, speed);
                                        }
                                    }
                                    "sensitivity" => {
                                        if let Some(sensitivity) = value.as_f64() {
                                            if let Ok(mut camera) = world.get::<&mut Camera>(entity) {
                                                camera.sensitivity = sensitivity;
                                            }
                                            if let Ok(mut component) = world.get::<&mut CameraComponent>(entity) {
                                                component.sensitivity = sensitivity;
                                            }
                                            log::info!("Updated camera '{}' sensitivity to {}", label, sensitivity);
                                        }
                                    }
                                    "fov" => {
                                        if let Some(fov) = value.as_f64() {
                                            if let Ok(mut camera) = world.get::<&mut Camera>(entity) {
                                                camera.fov_y = fov;
                                            }
                                            if let Ok(mut component) = world.get::<&mut CameraComponent>(entity) {
                                                component.fov_y = fov;
                                            }
                                            log::info!("Updated camera '{}' FOV to {}", label, fov);
                                        }
                                    }
                                    "yaw" => {
                                        if let Some(yaw) = value.as_f64() {
                                            if let Ok(mut camera) = world.get::<&mut Camera>(entity) {
                                                camera.yaw = yaw;
                                            }
                                            log::info!("Updated camera '{}' yaw to {}", label, yaw);
                                        }
                                    }
                                    "pitch" => {
                                        if let Some(pitch) = value.as_f64() {
                                            if let Ok(mut camera) = world.get::<&mut Camera>(entity) {
                                                camera.pitch = pitch;
                                            }
                                            log::info!("Updated camera '{}' pitch to {}", label, pitch);
                                        }
                                    }
                                    _ => {
                                        log::warn!("Unknown camera property: {}", property);
                                    }
                                }
                            } else {
                                log::warn!("Camera with label '{}' not found for manipulation", label);
                            }
                        }
                    }
                    "get_camera_by_label" => {
                        if let Some(label) = action.label {
                            log::info!("Getting camera data for label: {}", label);
                        }
                    }
                    "set_camera_by_label" => {
                        if let (Some(label), Some(camera_data)) = (action.label, action.camera_data) {
                            // Find and update the entire camera
                            let camera_entity = world
                                .query::<&Camera>()
                                .iter()
                                .find(|(_, camera)| camera.label == label)
                                .map(|(entity, _)| entity);

                            if let Some(entity) = camera_entity {
                                if let Ok(mut camera) = world.get::<&mut Camera>(entity) {
                                    camera.eye = camera_data.eye;
                                    camera.target = camera_data.target;
                                    camera.up = camera_data.up;
                                    camera.aspect = camera_data.aspect;
                                    camera.fov_y = camera_data.fov;
                                    camera.znear = camera_data.near;
                                    camera.zfar = camera_data.far;
                                    camera.yaw = camera_data.yaw;
                                    camera.pitch = camera_data.pitch;
                                    camera.speed = camera_data.speed;
                                    camera.sensitivity = camera_data.sensitivity;
                                    log::info!("Updated complete camera data for label: {}", label);
                                }
                            }
                        }
                    }
                    _ => {
                        log::warn!("Unknown camera action: {}", action.action);
                    }
                }
            }
        }
        Ok(())
    }
}