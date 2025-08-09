use dropbear_engine::entity::{AdoptedEntity, Transform};
use glam::{DQuat, DVec3};
use hecs::World;
use rhai::{Scope, AST};
use std::{collections::HashMap, fs};
use std::path::PathBuf;

use crate::states::{EntityNode, PROJECT, SOURCE, ScriptComponent};

pub const TEMPLATE_SCRIPT: &'static str = include_str!("../template.rhai");

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
        log::info!("Script file already exists at {:?}, returning existing path", dest_path);
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
                log::warn!("Script file already exists at {:?}, continuing anyway", dest_path);
                last_err = None;
                break;
            }
            Err(e) => {
                // Windows sharing violation = raw_os_error 32
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
    pub engine: rhai::Engine,
    compiled_scripts: HashMap<String, AST>,
    script_scopes: HashMap<hecs::Entity, Scope<'static>>,
}

impl ScriptManager {
    pub fn new() -> Self {
        let mut engine = rhai::Engine::new();

        engine.register_type_with_name::<Transform>("Transform");
        engine.register_type_with_name::<DQuat>("Quaternion");

        // transform
        engine.register_fn("new_transform", Transform::new);
        engine.register_get_set("position", 
            |t: &mut Transform| t.position,
            |t: &mut Transform, pos: DVec3| t.position = pos
        );
        engine.register_get_set("rotation",
            |t: &mut Transform| t.rotation,
            |t: &mut Transform, rot: DQuat| t.rotation = rot
        );
        engine.register_get_set("scale",
            |t: &mut Transform| t.scale,
            |t: &mut Transform, scale: DVec3| t.scale = scale
        );

        // vector methods
        engine.register_type_with_name::<DVec3>("Vector3");
        engine.register_fn("vec3", |x: f64, y: f64, z: f64| DVec3::new(x, y, z));
        engine.register_get_set("x", 
            |v: &mut DVec3| v.x,
            |v: &mut DVec3, x: f64| v.x = x
        );
        engine.register_get_set("y",
            |v: &mut DVec3| v.y, 
            |v: &mut DVec3, y: f64| v.y = y
        );
        engine.register_get_set("z",
            |v: &mut DVec3| v.z,
            |v: &mut DVec3, z: f64| v.z = z
        );

        // utils
        engine.register_fn("log", |msg: &str| {
            log::info!("[Script] {}", msg);
        });
        
        engine.register_fn("sin", |x: f64| x.sin());
        engine.register_fn("cos", |x: f64| x.cos());
        engine.register_fn("time", || {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f64()
        });

        Self {
            engine,
            compiled_scripts: HashMap::new(),
            script_scopes: HashMap::new(),
        }
    }

    pub fn init_entity_script(&mut self, entity_id: hecs::Entity, script_name: &str, world: &mut World) -> anyhow::Result<()> {
        if let Some(ast) = self.compiled_scripts.get(script_name) {
            let mut scope = Scope::new();
            
            if let Ok(transform) = world.query_one_mut::<&mut Transform>(entity_id) {
                scope.push("transform", *transform);
            }
            
            if let Ok(_) = self.engine.call_fn::<()>(&mut scope, ast, "init", ()) {
                log::debug!("Called init for entity {:?}", entity_id);
            }
            
            self.script_scopes.insert(entity_id, scope);
            Ok(())
        } else {
            Err(anyhow::anyhow!("Script '{}' not found", script_name))
        }
    }

    pub fn update_entity_script(&mut self, entity_id: hecs::Entity, script_name: &str, world: &mut World, dt: f32) -> anyhow::Result<()> {
        if let Some(ast) = self.compiled_scripts.get(script_name) {
            if let Some(scope) = self.script_scopes.get_mut(&entity_id) {
                if let Ok(transform) = world.query_one_mut::<&mut Transform>(entity_id) {
                    scope.set_value("transform", *transform);
                }
                
                if let Ok(_) = self.engine.call_fn::<()>(scope, ast, "update", (dt,)) {
                    if let Some(modified_transform) = scope.get_value::<Transform>("transform") {
                        if let Ok(mut transform) = world.get::<&mut Transform>(entity_id) {
                            *transform = modified_transform;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn remove_entity_script(&mut self, entity_id: hecs::Entity) {
        self.script_scopes.remove(&entity_id);
    }
    
    pub fn reload_script(&mut self, script_name: &str, script_path: &PathBuf) -> anyhow::Result<()> {
        let script_content = fs::read_to_string(script_path)?;
        let ast = self.engine.compile(&script_content)?;
        self.compiled_scripts.insert(script_name.to_string(), ast);
        
        log::info!("Reloaded script: {}", script_name);
        Ok(())
    }

    pub fn load_script(&mut self, script_path: &PathBuf) -> anyhow::Result<String> {
        let script_content = fs::read_to_string(script_path)?;
        let script_name = script_path.file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
            
        let ast = self.engine.compile(&script_content)?;
        self.compiled_scripts.insert(script_name.clone(), ast);
        
        log::info!("Loaded script: {}", script_name);
        Ok(script_name)
    }
}