use std::path::PathBuf;

use dropbear_engine::{
    entity::{AdoptedEntity, Transform}, graphics::{Graphics, Shader}, lighting::{Light, LightComponent}, model::{DrawLight, DrawModel}, scene::{Scene, SceneCommand}
};
use log;
use parking_lot::Mutex;
use wgpu::Color;
use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode};

use super::*;
use crate::{
    states::{Node, RESOURCES},
    utils::PendingSpawn,
};

pub static PENDING_SPAWNS: LazyLock<Mutex<Vec<PendingSpawn>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

impl Scene for Editor {
    fn load(&mut self, graphics: &mut Graphics) {
        self.load_project_config(graphics).unwrap();

        let shader = Shader::new(
            graphics,
            include_str!("../shader.wgsl"),
            Some("viewport_shader"),
        );
        if self.world.len() == 0 {
            let cube_path = {
                #[allow(unused_assignments)]
                let mut path = PathBuf::new();
                let resources = RESOURCES.read().unwrap();
                let mut matches = Vec::new();
                crate::utils::search_nodes_recursively(
                    &resources.nodes,
                    &|node| match node {
                        Node::File(file) => file.name.contains("cube"),
                        Node::Folder(folder) => folder.name.contains("cube"),
                    },
                    &mut matches,
                );
                match matches.get(0) {
                    Some(thing) => match thing {
                        Node::File(file) => path = file.path.clone(),
                        Node::Folder(folder) => path = folder.path.clone(),
                    },
                    None => path = PathBuf::new(),
                }
                path
            };

            if cube_path != PathBuf::new() {
                let cube = AdoptedEntity::new(graphics, &cube_path, Some("default_cube")).unwrap();
                self.world
                    .spawn((cube, Transform::default(), ModelProperties::default()));
                log::info!("Added default cube since no entities were loaded from scene");
            } else {
                log::warn!("cube path is empty :(")
            }
        } else {
            log::info!(
                "Scene loaded with {} entities, skipping default cube",
                self.world.len()
            );
        }

        self.light_manager.create_light_array_resources(graphics);

        // let main_light_transform = Transform {
        //     position: DVec3::new(2.0, 4.0, 2.0),
        //     ..Default::default()
        // };
        // let main_light_component = LightComponent::directional(DVec3::ONE, 1.0);
        // let main_adopted_light = Light::new(graphics, &main_light_component, &main_light_transform, Some("Main Light"));

        // let second_light_transform = Transform {
        //     position: DVec3::new(-10.0, 3.0, -1.0),
        //     ..Default::default()
        // };
        // let second_light_component = LightComponent::point(DVec3::new(1.0, 0.5, 1.0), 0.8);
        // let second_adopted_light = Light::new(graphics, &second_light_component, &second_light_transform, Some("Second Light"));
        
        let texture_bind_group = &graphics.texture_bind_group().clone();
        if let Some(camera) = self.camera_manager.get_active() {
            let pipeline = graphics.create_render_pipline(
                &shader,
                vec![
                    texture_bind_group, 
                    camera.layout(),
                    self.light_manager.layout()
                ],
                None,
            );
            self.render_pipeline = Some(pipeline);
            
            self.light_manager.create_render_pipeline(
                graphics, 
                include_str!("../light.wgsl"), 
                camera,
                Some("Light Pipeline")
            );
        }

        // self.world.spawn((
        //     main_light_component,
        //     main_light_transform,
        //     main_adopted_light,
        // ));

        // self.world.spawn((
        //     second_light_component,
        //     second_light_transform,
        //     second_adopted_light,
        // ));

        self.window = Some(graphics.state.window.clone());
    }

    fn update(&mut self, dt: f32, graphics: &mut Graphics) {
        if let Some((_, tab)) = self.dock_state.find_active_focused() {
            self.is_viewport_focused = matches!(tab, EditorTab::Viewport);
        } else {
            self.is_viewport_focused = false;
        }

        {
            let mut pending_spawns = PENDING_SPAWNS.lock();
            for spawn in pending_spawns.drain(..) {
                match AdoptedEntity::new(graphics, &spawn.asset_path, Some(&spawn.asset_name)) {
                    Ok(adopted) => {
                        let entity_id =
                            self.world
                                .spawn((adopted, spawn.transform, spawn.properties));
                        self.selected_entity = Some(entity_id);

                        UndoableAction::push_to_undo(
                            &mut self.undo_stack,
                            UndoableAction::Spawn(entity_id),
                        );

                        log::info!(
                            "Successfully spawned {} with ID {:?}",
                            spawn.asset_name,
                            entity_id
                        );
                    }
                    Err(e) => {
                        log::error!("Failed to spawn {}: {}", spawn.asset_name, e);
                    }
                }
            }
        }

        if matches!(self.editor_state, EditorState::Playing) {
            if self.input_state.pressed_keys.contains(&KeyCode::Escape) {
                self.signal = Signal::StopPlaying;
            }
            
            let mut script_entities = Vec::new();
            for (entity_id, script) in self.world.query::<&ScriptComponent>().iter() {
                script_entities.push((entity_id, script.name.clone()));
            }

            if script_entities.is_empty() {
                log::warn!("Script entities is empty");
            }
            
            for (entity_id, script_name) in script_entities {
                if let Err(e) = self.script_manager.update_entity_script(entity_id, &script_name, &mut self.world, &self.input_state, dt) {
                    log::warn!("Failed to update script '{}' for entity {:?}: {}", script_name, entity_id, e);
                }
            }
        }

        if self.is_viewport_focused
            && matches!(self.viewport_mode, crate::utils::ViewportMode::CameraMove)
        // && self.is_using_debug_camera()
        {
            let movement_keys: std::collections::HashSet<KeyCode> = self
                .input_state
                .pressed_keys
                .iter()
                .filter(|&&key| {
                    matches!(
                        key,
                        KeyCode::KeyW
                            | KeyCode::KeyA
                            | KeyCode::KeyS
                            | KeyCode::KeyD
                            | KeyCode::Space
                            | KeyCode::ShiftLeft
                    )
                })
                .copied()
                .collect();

            self.camera_manager
                .handle_input(&movement_keys, self.input_state.mouse_delta);
        }

        match &self.signal {
            Signal::Paste(scene_entity) => {
                        match AdoptedEntity::new(
                            graphics,
                            &scene_entity.model_path,
                            Some(&scene_entity.label),
                        ) {
                            Ok(adopted) => {
                                let entity_id = self.world.spawn((
                                    adopted,
                                    scene_entity.transform,
                                    ModelProperties::default(),
                                ));
                                self.selected_entity = Some(entity_id);
                                log::debug!(
                                    "Successfully paste-spawned {} with ID {:?}",
                                    scene_entity.label,
                                    entity_id
                                );

                                crate::success_without_console!("Paste!");
                                self.signal = Signal::Copy(scene_entity.clone());
                            }
                            Err(e) => {
                                crate::warn!("Failed to paste-spawn {}: {}", scene_entity.label, e);
                            }
                        }
                    }
            Signal::Delete => {
                        if let Some(sel_e) = &self.selected_entity {
                            match self.world.despawn(*sel_e) {
                                Ok(_) => {
                                    crate::info!("Decimated entity");
                                    self.signal = Signal::None;
                                }
                                Err(e) => {
                                    crate::warn!("Failed to delete entity: {}", e);
                                }
                            }
                        }
                    }
            Signal::Undo => {
                        if let Some(action) = self.undo_stack.pop() {
                            match action.undo(&mut self.world) {
                                Ok(_) => {
                                    crate::info!("Undid action");
                                }
                                Err(e) => {
                                    crate::warn!("Failed to undo action: {}", e);
                                }
                            }
                        } else {
                            crate::warn_without_console!("Nothing to undo");
                            log::debug!("No undoable actions in stack");
                        }
                        self.signal = Signal::None;
                    }
            Signal::None => {}
            Signal::Copy(_) => {}
            Signal::ScriptAction(action) => match action {
                        ScriptAction::AttachScript {
                            script_path,
                            script_name,
                        } => {
                            if let Some(selected_entity) = self.selected_entity {
                                match crate::scripting::move_script_to_src(script_path) {
                                    Ok(moved_path) => {
                                        let new_script = ScriptComponent {
                                            name: script_name.clone(),
                                            path: moved_path.clone(),
                                        };

                                        let replaced = if let Ok(mut sc) =
                                            self.world.get::<&mut ScriptComponent>(selected_entity)
                                        {
                                            sc.name = new_script.name.clone();
                                            sc.path = new_script.path.clone();
                                            true
                                        } else {
                                            match crate::scripting::attach_script_to_entity(
                                                &mut self.world,
                                                selected_entity,
                                                new_script.clone(),
                                            ) {
                                                Ok(_) => false,
                                                Err(e) => {
                                                    crate::fatal!(
                                                        "Failed to attach script to entity {:?}: {}",
                                                        selected_entity,
                                                        e
                                                    );
                                                    self.signal = Signal::None;
                                                    return;
                                                }
                                            }
                                        };

                                        if let Err(e) = crate::scripting::convert_entity_to_group(
                                            &self.world,
                                            selected_entity,
                                        ) {
                                            log::warn!("convert_entity_to_group failed (non-fatal): {}", e);
                                        }

                                        crate::success!(
                                            "{} script '{}' at {} to entity {:?}",
                                            if replaced { "Reattached" } else { "Attached" },
                                            script_name,
                                            moved_path.display(),
                                            selected_entity
                                        );
                                    }
                                    Err(e) => {
                                        crate::fatal!("Move failed: {}", e);
                                    }
                                }
                            } else {
                                crate::fatal!("AttachScript requested but no entity is selected");
                            }

                            self.signal = Signal::None;
                        }
                        ScriptAction::CreateAndAttachScript {
                            script_path,
                            script_name,
                        } => {
                            if let Some(selected_entity) = self.selected_entity {
                                let new_script = ScriptComponent {
                                    name: script_name.clone(),
                                    path: script_path.clone(),
                                };

                                let replaced = if let Ok(mut sc) =
                                    self.world.get::<&mut ScriptComponent>(selected_entity)
                                {
                                    sc.name = new_script.name.clone();
                                    sc.path = new_script.path.clone();
                                    true
                                } else {
                                    match crate::scripting::attach_script_to_entity(
                                        &mut self.world,
                                        selected_entity,
                                        new_script.clone(),
                                    ) {
                                        Ok(_) => false,
                                        Err(e) => {
                                            crate::fatal!("Failed to attach new script: {}", e);
                                            self.signal = Signal::None;
                                            return;
                                        }
                                    }
                                };

                                if let Err(e) =
                                    crate::scripting::convert_entity_to_group(&self.world, selected_entity)
                                {
                                    log::warn!("convert_entity_to_group failed (non-fatal): {}", e);
                                }

                                crate::success!(
                                    "{} new script '{}' at {} to entity {:?}",
                                    if replaced { "Replaced" } else { "Attached" },
                                    script_name,
                                    script_path.display(),
                                    selected_entity
                                );
                            } else {
                                crate::warn_without_console!("No selected entity to attach new script");
                                log::warn!("CreateAndAttachScript requested but no entity is selected");
                            }
                            self.signal = Signal::None;
                        }
                        ScriptAction::RemoveScript => {
                            if let Some(selected_entity) = self.selected_entity {
                                if let Ok(_) = self.world.remove_one::<ScriptComponent>(selected_entity) {
                                    crate::success!("Removed script from entity {:?}", selected_entity);

                                    if let Err(e) = crate::scripting::convert_entity_to_group(
                                        &self.world,
                                        selected_entity,
                                    ) {
                                        log::warn!("convert_entity_to_group failed (non-fatal): {}", e);
                                    }
                                } else {
                                    crate::warn!(
                                        "No script component found on entity {:?}",
                                        selected_entity
                                    );
                                }
                            } else {
                                crate::warn!("No entity selected to remove script from");
                            }

                            self.signal = Signal::None;
                        }
                        ScriptAction::EditScript => {
                            if let Some(selected_entity) = self.selected_entity {
                                if let Ok(mut q) = self.world.query_one::<&ScriptComponent>(selected_entity)
                                {
                                    if let Some(script) = q.get() {
                                        match open::that(script.path.clone()) {
                                            Ok(()) => {
                                                crate::success!("Opened {}", script.name)
                                            }
                                            Err(e) => {
                                                crate::warn!("Error while opening {}: {}", script.name, e);
                                            }
                                        }
                                    }
                                } else {
                                    crate::warn!(
                                        "No script component found on entity {:?}",
                                        selected_entity
                                    );
                                }
                            } else {
                                crate::warn!("No entity selected to edit script");
                            }
                            self.signal = Signal::None;
                        }
            },
            Signal::Play => {
                // verify that a player camera is attached to an entity
                if let Some(_) = self.camera_manager.get_player_camera_target() {
                    if let Err(e) = PlayModeBackup::create_backup(self) {
                        crate::fatal!("Failed to create play mode backup: {}", e);
                        self.signal = Signal::None;
                        return;
                    }

                    self.editor_state = EditorState::Playing;
                    
                    self.camera_manager.set_active(CameraType::Player);
                    
                    let mut script_entities = Vec::new();
                    for (entity_id, script) in self.world.query::<&ScriptComponent>().iter() {
                        script_entities.push((entity_id, script.clone()));
                    }
                    
                    for (entity_id, script) in script_entities {
                        match self.script_manager.load_script(&script.path) {
                            Ok(script_name) => {
                                if let Err(e) = self.script_manager.init_entity_script(entity_id, &script_name, &mut self.world, &self.input_state) {
                                    log::warn!("Failed to initialise script '{}' for entity {:?}: {}", script.name, entity_id, e);
                                }
                            }
                            Err(e) => {
                                log::warn!("Failed to load script '{}': {}", script.name, e);
                            }
                        }
                    }
                    crate::success_without_console!("You are in play mode now! Press Escape to exit");
                    log::info!("You are in play mode now. Press Escape to exit");
                } else {
                    crate::fatal!("Unable to build: Player camera not attached to an entity");
                }

                self.signal = Signal::None;
            }
            Signal::StopPlaying => {
                if let Err(e) = PlayModeBackup::restore(self) {
                    crate::warn!("Failed to restore from play mode backup: {}", e);
                    log::warn!("Failed to restore scene state: {}", e);
                }

                self.editor_state = EditorState::Editing;
    
                self.camera_manager.set_active(CameraType::Debug);
                
                for (entity_id, _) in self.world.query::<&ScriptComponent>().iter() {
                    self.script_manager.remove_entity_script(entity_id);
                }
                
                crate::success!("Exited play mode");
                log::info!("Back to the editor you go...");

                self.signal = Signal::None;
            },
            Signal::CameraAction(action) => match action {
                        CameraAction::SetPlayerTarget { entity, offset } => {
                            self.camera_manager
                                .set_player_camera_target(*entity, *offset);
                            self.signal = Signal::None;
                        }
                        CameraAction::ClearPlayerTarget => {
                            self.camera_manager.clear_player_camera_target();
                            crate::info!("Cleared player camera target");
                            self.signal = Signal::None;
                        }
                    },
            
        }

        let current_size = graphics.state.viewport_texture.size;
        self.size = current_size;
        
        let new_aspect = current_size.width as f64 / current_size.height as f64;
        if let Some(camera) = self.camera_manager.get_active_mut() {
            camera.aspect = new_aspect;
        }

        self.camera_manager.update_camera_following(&self.world, dt);
        self.camera_manager.update_all(dt, graphics);

        let query = self.world.query_mut::<(&mut AdoptedEntity, &Transform)>();
        for (_, (entity, transform)) in query {
            entity.update(&graphics, transform);
        }

        let light_query = self.world.query_mut::<(&LightComponent, &Transform, &mut Light)>();
        for (_, (light_component, transform, light)) in light_query {
            light.update(graphics, light_component, transform);
        }

        self.light_manager.update(graphics, &self.world);
    }

    fn render(&mut self, graphics: &mut Graphics) {
        // cornflower blue
        let color = Color {
            r: 100.0 / 255.0,
            g: 149.0 / 255.0,
            b: 237.0 / 255.0,
            a: 1.0,
        };

        self.color = color.clone();
        self.size = graphics.state.viewport_texture.size.clone();
        self.texture_id = Some(graphics.state.texture_id.clone());
        self.show_ui(&graphics.get_egui_context());

        self.window = Some(graphics.state.window.clone());
        crate::logging::render(&graphics.get_egui_context());
        if let Some(pipeline) = &self.render_pipeline {
            if let Some(camera) = self.camera_manager.get_active() {
                let mut light_query = self.world.query::<(&Light, &LightComponent)>();
                let mut entity_query = self.world.query::<(&AdoptedEntity, &Transform)>();
                {
                    let mut render_pass = graphics.clear_colour(color);
                    if let Some(light_pipeline) = &self.light_manager.pipeline {
                        render_pass.set_pipeline(light_pipeline);
                        for (_, (light, component)) in light_query.iter() {
                            if component.enabled {
                                render_pass.draw_light_model(
                                    light.model(),
                                    camera.bind_group(), 
                                    light.bind_group(),
                                );
                            }
                        }
                    }

                    render_pass.set_pipeline(pipeline);

                    for (_, (entity, _)) in entity_query.iter() {
                        render_pass.set_vertex_buffer(1, entity.instance_buffer.as_ref().unwrap().slice(..));
                        render_pass.set_bind_group(2, entity.uniform_bind_group.as_ref().unwrap(), &[]);
                        render_pass.draw_model(entity.model(), camera.bind_group(), self.light_manager.bind_group());
                    }
                }

                
            }
        }
    }

    fn exit(&mut self, _event_loop: &ActiveEventLoop) {}

    fn run_command(&mut self) -> SceneCommand {
        std::mem::replace(&mut self.scene_command, SceneCommand::None)
    }
}
