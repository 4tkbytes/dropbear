pub mod entity;
pub mod math;
pub mod input;

use chrono::DateTime;
use dropbear_engine::entity::{AdoptedEntity, Transform};
use glam::{DQuat, DVec3};
use hecs::World;
use rustyscript::{Runtime, RuntimeOptions};
use specta::{Type, TypeCollection};
use specta_typescript::Typescript;
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

// #[cfg(feature = "editor")]
// pub fn generate_ts() -> anyhow::Result<()> {
//     let mut types = TypeCollection::default();
//     types.register::<Transform>();

//     let utc: DateTime<Utc> = Utc::now();
//     let header = format!("
// // @generated by dropbear-engine on {} UTC time
// // Hello there! This is the generated stubs for the Eucalyptus Editor scripting component. 
// // This file purely exists for the LSP and for TypeScripts type safety. Running this library 
// // most likely won't do anything, but go ahead and be my guest. 
// // EDIT THIS LIBRARY AT YOUR OWN WILL. IT IS RECOMMENDED TO NOT TOUCH THIS FILE
//     ", utc);
    
//     if let Ok(project) = PROJECT.read() {
//         Typescript::new().header(header).export_to(project.project_path.clone().join("src/dropbear-engine.d.ts"), types)?;
//     } else {
//         anyhow::bail!("Unable to retain Project Config lock, could not generate typescript bindings");
//     }
//     Ok(())
// }

pub struct ScriptManager {
    pub runtime: Runtime,
    compiled_scripts: HashMap<String, AST>,
    script_scopes: HashMap<hecs::Entity, Scope<'static>>,
}

impl ScriptManager {
    pub fn new() -> anyhow::Result<Self> {
        let mut runtime = Runtime::new(RuntimeOptions::default())?;
        

        // // REGISTER FUNCTIONS HERE
        // math::register_math_functions(&mut runtime);
        // input::InputState::register_input_modules(&mut runtime);
        // entity::register_model_props_module(&mut runtime);

        // // register types used by scripts
        // runtime.register_type_with_name::<Transform>("Transform");
        // runtime.register_type_with_name::<DQuat>("Quaternion");
        // runtime.register_type_with_name::<DVec3>("Vector3");

        // runtime.mod

        // // constructors / helpers (alias names to match helper.ts)
        // runtime.register_function("createTransform", Transform::new);
        // runtime.register_function("createVec3", |x: f64, y: f64, z: f64| DVec3::new(x, y, z));
        // // keep the original alias if you used "vec3" elsewhere
        // runtime.register_function("vec3", |x: f64, y: f64, z: f64| DVec3::new(x, y, z));

        // runtime.register_function("createQuatIdentity", || DQuat::IDENTITY);
        // // from-euler: adapt to your exact DQuat API if different
        // runtime.register_function("createQuatFromEuler", |x: f64, y: f64, z: f64| {
        //     // uses EulerRot::XYZ — adjust if your glam API differs
        //     DQuat::from_euler(glam::EulerRot::XYZ, x, y, z)
        // });

        // // transform helpers: simple implementations that return an updated Transform
        // runtime.register_function(
        //     "transformTranslate",
        //     |mut t: Transform, v: DVec3| {
        //         t.position += v;
        //         t
        //     },
        // );
        // runtime.register_function(
        //     "transformScale",
        //     |mut t: Transform, s: DVec3| {
        //         t.scale *= s;
        //         t
        //     },
        // );
        // runtime.register_function(
        //     "transformMatrix",
        //     |t: Transform| {
        //         // return whatever representation your scripts expect (e.g. a 4x4 matrix type)
        //         t.matrix() // replace with actual method if different
        //     },
        // );

        // // axis rotations — simple wrappers that multiply quaternions
        // runtime.register_function("transformRotateX", |mut t: Transform, a: f64| {
        //     let r = DQuat::from_rotation_x(a);
        //     t.rotation = r * t.rotation;
        //     t
        // });
        // runtime.register_function("transformRotateY", |mut t: Transform, a: f64| {
        //     let r = DQuat::from_rotation_y(a);
        //     t.rotation = r * t.rotation;
        //     t
        // });
        // runtime.register_function("transformRotateZ", |mut t: Transform, a: f64| {
        //     let r = DQuat::from_rotation_z(a);
        //     t.rotation = r * t.rotation;
        //     t
        // });

        // // utils already in your file — log and time
        // runtime.register_function("log", |msg: String| {
        //     println!("[Script] {}", msg);
        // });
        // runtime.register_function("time", || {
        //     std::time::SystemTime::now()
        //         .duration_since(std::time::UNIX_EPOCH)
        //         .unwrap()
        //         .as_secs_f64()
        // });

        Self {
            runtime,
            compiled_scripts: HashMap::new(),
            script_scopes: HashMap::new(),
        }
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

        if let Some(ast) = self.compiled_scripts.get(script_name) {
            let mut scope = Scope::new();

            if let Ok(transform) = world.query_one_mut::<&mut Transform>(entity_id) {
                scope.push("transform", *transform);
            }

            if let Ok(properties) = world.query_one_mut::<&mut ModelProperties>(entity_id) {
                scope.push("entity", properties.clone());
            } else {
                let default_props = ModelProperties::default();
                // default_props.set_property(String::from("speed"), PropertyValue::Float(1.0));
                scope.push("entity", default_props);
            }

            scope.push("input", input_state.clone());

            if let Ok(_) = self.engine.call_fn::<()>(&mut scope, ast, "init", ()) {
                log::debug!("Called init for entity {:?}", entity_id);

                if let Some(properties_from_scope) = scope.get_value::<ModelProperties>("entity") {
                    if let Ok(properties) = world.query_one_mut::<&mut ModelProperties>(entity_id) {
                        *properties = properties_from_scope;
                    } else {
                        let _ = world.insert_one(entity_id, properties_from_scope);
                    }
                }

                if let Some(transform_from_scope) = scope.get_value::<Transform>("transform") {
                    if let Ok(transform) = world.query_one_mut::<&mut Transform>(entity_id) {
                        *transform = transform_from_scope;
                    }
                }
            }

            self.script_scopes.insert(entity_id, scope);
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
        if let Some(ast) = self.compiled_scripts.get(script_name) {
            if let Some(scope) = self.script_scopes.get_mut(&entity_id) {
                if let Ok(transform) = world.query_one_mut::<&mut Transform>(entity_id) {
                    scope.set_value("transform", *transform);
                }

                if let Ok(mut properties_query) = world.query_one::<&ModelProperties>(entity_id) {
                    if let Some(properties) = properties_query.get() {
                        scope.set_value("entity", properties.clone());
                    }
                }

                scope.set_value("input", input_state.clone());

                match self.engine.call_fn::<()>(scope, ast, "update", (dt,)) {
                    Ok(_) => {
                        if let Some(transform_from_scope) = scope.get_value::<Transform>("transform") {
                            if let Ok(transform) = world.query_one_mut::<&mut Transform>(entity_id) {
                                *transform = transform_from_scope;
                            }
                        }

                        if let Some(properties_from_scope) = scope.get_value::<ModelProperties>("entity") {
                            if let Ok(properties) = world.query_one_mut::<&mut ModelProperties>(entity_id) {
                                *properties = properties_from_scope;
                            }
                        }
                    }
                    Err(e) => {
                        log_once::error_once!("Script execution error for entity {:?}: {}", entity_id, e);
                    }
                }
            } else {
                log_once::error_once!("Unable to get scope of entity {:?}", entity_id);
            }
        } else {
            log_once::error_once!("Unable to fetch compiled scripts for entity {:?}. Script Name: {}", entity_id, script_name);
        }
        Ok(())
    }

    pub fn load_script_from_source(&mut self, script_name: &String, script_content: &String) -> anyhow::Result<String> {
        let ast = self.engine.compile(&script_content).map_err(|e| {
            log::error!("Compiling AST for script [{}] returned an error: {}", script_name, e);
            e
        })?;
        self.compiled_scripts.insert(script_name.clone(), ast);

        log::info!("Loaded script: {}", script_name);
        Ok(script_name.to_string())
    }

    pub fn load_script(&mut self, script_path: &PathBuf) -> anyhow::Result<String> {
        let script_content = fs::read_to_string(script_path)?;
        let script_name = script_path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let ast = self.engine.compile(&script_content).map_err(|e| {
            log::error!("Compiling AST for script path [{}] returned an error: {}", script_path.display(), e);
            e
        })?;
        self.compiled_scripts.insert(script_name.clone(), ast);

        log::info!("Loaded script: {}", script_name);
        Ok(script_name)
    }

    pub fn remove_entity_script(&mut self, entity_id: hecs::Entity) {
        self.script_scopes.remove(&entity_id);
    }

    // maybe useful later???
    // pub fn reload_script(
    //     &mut self,
    //     script_name: &str,
    //     script_path: &PathBuf,
    // ) -> anyhow::Result<()> {
    //     let script_content = fs::read_to_string(script_path)?;
    //     let ast = self.engine.compile(&script_content)?;
    //     self.compiled_scripts.insert(script_name.to_string(), ast);

    //     log::info!("Reloaded script: {}", script_name);
    //     Ok(())
    // }
}