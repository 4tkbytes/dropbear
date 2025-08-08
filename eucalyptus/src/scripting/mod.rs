use dropbear_engine::entity::{AdoptedEntity, Transform};
use hecs::World;
use std::fs;
use std::path::PathBuf;

use crate::states::{EntityNode, PROJECT, SOURCE, ScriptComponent};

pub const RHAI_TEMPLATE_SCRIPT: &'static str = include_str!("../template.rhai");

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
    ExecuteScript,
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
        return Err(anyhow::anyhow!(
            "Script file already exists in src: {:?}",
            filename
        ));
    }

    match fs::copy(script_path, &dest_path) {
        Ok(_) => {
            log::info!("Copied script from {:?} to {:?}", script_path, dest_path);
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            log::warn!(
                "Script file already exists at {:?}, continuing anyway",
                dest_path
            );
        }
        Err(e) => return Err(e.into()),
    }

    {
        let source_config = SOURCE.read().unwrap();
        source_config.write_to(&project_path)?;
    }

    log::info!("Moved script from {:?} to {:?}", script_path, dest_path);

    Ok(dest_path)
}

pub fn attach_script_and_convert_to_group(
    world: &mut World,
    entity_id: hecs::Entity,
    script_path: PathBuf,
    script_name: String,
) -> anyhow::Result<EntityNode> {
    // First attach the script to the entity
    attach_script_to_entity(world, entity_id, script_path, script_name)?;

    // Then convert the entity to a group node
    convert_entity_to_group(world, entity_id)
}

pub fn convert_entity_to_group(
    world: &World,
    entity_id: hecs::Entity,
) -> anyhow::Result<EntityNode> {
    // Query the entity for its components
    if let Ok(mut query) = world.query_one::<(&AdoptedEntity, &Transform)>(entity_id) {
        if let Some((adopted, _transform)) = query.get() {
            let entity_name = adopted.model().label.clone();

            // Check if entity has a script component
            let script_node = if let Ok(script) = world.get::<&ScriptComponent>(entity_id) {
                Some(EntityNode::Script {
                    name: script.name.clone(),
                    path: script.path.clone(),
                })
            } else {
                None
            };

            // Create the entity node
            let entity_node = EntityNode::Entity {
                id: entity_id,
                name: entity_name.clone(),
            };

            // If there's a script, create a group containing both entity and script
            if let Some(script_node) = script_node {
                Ok(EntityNode::Group {
                    name: entity_name,
                    children: vec![entity_node, script_node],
                    collapsed: false,
                })
            } else {
                // Return just the entity if no script is attached
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
    script_path: PathBuf,
    script_name: String,
) -> anyhow::Result<()> {
    let script_component = ScriptComponent {
        name: script_name,
        path: script_path,
    };

    // Add the script component to the existing entity
    if let Err(e) = world.insert_one(entity_id, script_component) {
        return Err(anyhow::anyhow!("Failed to attach script to entity: {}", e));
    }

    log::info!("Successfully attached script to entity {:?}", entity_id);
    Ok(())
}
