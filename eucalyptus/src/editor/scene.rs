use egui::Align2;
use dropbear_engine::{
    entity::{AdoptedEntity, Transform}, graphics::{Graphics, Shader}, lighting::{Light, LightComponent}, model::{DrawLight, DrawModel}, scene::{Scene, SceneCommand}
};
use log;
use parking_lot::Mutex;
use wgpu::Color;
use wgpu::util::DeviceExt;
use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode};
use dropbear_engine::graphics::InstanceRaw;
use dropbear_engine::model::Model;
use dropbear_engine::starter::plane::PlaneBuilder;
use super::*;
use crate::{
    camera::PlayerCamera, utils::PendingSpawn
};
use crate::states::PropertyValue;
use crate::utils::PROTO_TEXTURE;

pub static PENDING_SPAWNS: LazyLock<Mutex<Vec<PendingSpawn>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

impl Scene for Editor {
    fn load(&mut self, graphics: &mut Graphics) {
        if self.active_camera.is_none() {
            self.load_project_config(graphics).unwrap();
        }

        let shader = Shader::new(
            graphics,
            include_str!("../shader.wgsl"),
            Some("viewport_shader"),
        );

        self.light_manager.create_light_array_resources(graphics);

        let texture_bind_group = &graphics.texture_bind_group().clone();
        if let Some(active_camera) = self.active_camera {
            if let Ok(mut q) = self.world.query_one::<(&Camera, &CameraComponent, Option<&CameraFollowTarget>)>(active_camera) {
                if let Some((camera, _component, _follow_target)) = q.get() {
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
                } else {
                    log_once::warn_once!("Unable to fetch the query result of camera: {:?}", active_camera)
                }
            } else {
                log_once::warn_once!("Unable to query camera, component and option<camerafollowtarget> for active camera: {:?}", active_camera);
            }
        } else {
            log_once::warn_once!("No active camera found");
        }

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
            for (entity_id, script) in self.world.query::<&mut ScriptComponent>().iter() {
                log_once::debug_once!("Script Entity -> id: {:?}, component: {:?}", entity_id, script);
                script.name = script.path.file_name().unwrap().to_str().unwrap().to_string();
                script_entities.push((entity_id, script.name.clone()));
            }

            if script_entities.is_empty() {
                log_once::warn_once!("Script entities is empty");
            }

            for (entity_id, script_name) in script_entities {
                if let Err(e) = self.script_manager.update_entity_script(entity_id, &script_name, &mut self.world, &self.input_state, dt) {
                    log_once::warn_once!("Failed to update script '{}' for entity {:?}: {}", script_name, entity_id, e);
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

            // Handle camera input through ECS
            if let Some(active_camera) = self.active_camera {
                if let Ok(mut query) = self.world.query_one::<(&mut Camera, &CameraComponent)>(active_camera) {
                    if let Some((camera, component)) = query.get() {
                        // Handle keyboard input based on camera type
                        match component.camera_type {
                            CameraType::Debug => {
                                DebugCamera::handle_keyboard_input(camera, &movement_keys);
                                DebugCamera::handle_mouse_input(camera, component, self.input_state.mouse_delta);
                            }
                            CameraType::Player => {
                                PlayerCamera::handle_keyboard_input(camera, &movement_keys);
                                PlayerCamera::handle_mouse_input(camera, component, self.input_state.mouse_delta);
                            }
                            CameraType::Normal => {
                                // Handle normal camera input if needed
                                DebugCamera::handle_keyboard_input(camera, &movement_keys);
                                DebugCamera::handle_mouse_input(camera, component, self.input_state.mouse_delta);
                            }
                        }
                    }
                }
            }
        }

        match &self.signal {
            Signal::Paste(scene_entity) => {
                        match AdoptedEntity::new(
                            graphics,
                            &scene_entity.model_path.to_project_path(self.project_path.clone().unwrap()).unwrap(),
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
                            let is_viewport_cam = if let Ok(mut q) = self.world.query_one::<&CameraComponent>(*sel_e) { if let Some(c) = q.get() { if matches!(c.camera_type, CameraType::Debug) { true } else { false } } else { false } } else { false };
                            if is_viewport_cam {
                                crate::warn!("You can't delete the viewport camera");
                                self.signal = Signal::None;
                            } else {
                                match self.world.despawn(*sel_e) {
                                    Ok(_) => {
                                        crate::info!("Decimated entity");
                                        self.signal = Signal::None;
                                    }
                                    Err(e) => {
                                        crate::warn!("Failed to delete entity: {}", e);
                                        self.signal = Signal::None;
                                    }
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
                                if let Ok(script) = self.world.remove_one::<ScriptComponent>(selected_entity) {
                                    crate::success!("Removed script from entity {:?}", selected_entity);

                                    if let Err(e) = crate::scripting::convert_entity_to_group(
                                        &self.world,
                                        selected_entity,
                                    ) {
                                        log::warn!("convert_entity_to_group failed (non-fatal): {}", e);
                                    }
                                    log::debug!("Pushing remove component to undo stack");
                                    UndoableAction::push_to_undo(&mut self.undo_stack, UndoableAction::RemoveComponent(selected_entity, ComponentType::Script(script)));
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
                // Check if a player camera target exists
                let has_player_camera_target = self.world
                    .query::<(&Camera, &CameraComponent, &CameraFollowTarget)>()
                    .iter()
                    .any(|(_, (_, comp, _))| matches!(comp.camera_type, CameraType::Player));

                if has_player_camera_target {
                    if let Err(e) = PlayModeBackup::create_backup(self) {
                        crate::fatal!("Failed to create play mode backup: {}", e);
                        self.signal = Signal::None;
                        return;
                    }

                    self.editor_state = EditorState::Playing;

                    self.switch_to_player_camera();

                    let mut script_entities = Vec::new();
                    for (entity_id, script) in self.world.query::<&ScriptComponent>().iter() {
                        script_entities.push((entity_id, script.clone()));
                    }

                    for (entity_id, script) in script_entities {
                        log::debug!("Initialising entity script [{}] from path: {}", script.name, script.path.display());
                        match self.script_manager.load_script(&script.path) {
                            Ok(script_name) => {
                                if let Err(e) = self.script_manager.init_entity_script(entity_id, &script_name, &mut self.world, &self.input_state) {
                                    log::warn!("Failed to initialise script '{}' for entity {:?}: {}", script.name, entity_id, e);
                                    self.signal = Signal::StopPlaying;
                                } else {
                                    crate::success_without_console!("You are in play mode now! Press Escape to exit");
                                    log::info!("You are in play mode now! Press Escape to exit");
                                }
                            }
                            Err(e) => {
                                // todo: proper error menu
                                crate::fatal!("Failed to load script '{}': {}", script.name, e);
                                self.signal = Signal::StopPlaying;
                            }
                        }
                    }
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

                self.switch_to_debug_camera();

                for (entity_id, _) in self.world.query::<&ScriptComponent>().iter() {
                    self.script_manager.remove_entity_script(entity_id);
                }

                crate::success!("Exited play mode");
                log::info!("Back to the editor you go...");

                self.signal = Signal::None;
            },
            Signal::CameraAction(action) => match action {
                CameraAction::SetPlayerTarget { entity, offset } => {
                    // Find player camera and add/update CameraFollowTarget component
                    let player_camera = self.world
                        .query::<(&Camera, &CameraComponent)>()
                        .iter()
                        .find_map(|(e, (_, comp))| {
                            if matches!(comp.camera_type, CameraType::Player) {
                                Some(e)
                            } else {
                                None
                            }
                        });

                    if let Some(camera_entity) = player_camera {
                        let mut follow_target = (false, CameraFollowTarget::default());
                        // Find the target entity label
                        if let Ok(mut query) = self.world.query_one::<&AdoptedEntity>(*entity) {
                            if let Some(adopted) = query.get() {
                                follow_target = (true, CameraFollowTarget {
                                    follow_target: adopted.label().to_string(),
                                    offset: *offset,
                                });
                            }
                        }

                        if follow_target.0 {
                            let _ = self.world.insert_one(camera_entity, follow_target);
                            crate::info!("Set player camera target to entity {:?}", entity);
                        }
                    }
                    self.signal = Signal::None;
                }
                CameraAction::ClearPlayerTarget => {
                    let player_camera = self.world
                        .query::<(&Camera, &CameraComponent)>()
                        .iter()
                        .find_map(|(e, (_, comp))| {
                            if matches!(comp.camera_type, CameraType::Player) {
                                Some(e)
                            } else {
                                None
                            }
                        });

                    if let Some(camera_entity) = player_camera {
                        let _ = self.world.remove_one::<CameraFollowTarget>(camera_entity);
                    }
                    crate::info!("Cleared player camera target");
                    self.signal = Signal::None;
                }
            },
            Signal::AddComponent(entity, e_type) => {
                match e_type {
                    EntityType::Entity => {
                        if let Ok(e) = self.world.query_one_mut::<&AdoptedEntity>(*entity) {
                            let mut local_signal: Option<Signal> = None;
                            let label = e.label().clone();
                            let mut show = true;
                            egui::Window::new(format!("Add component for {}", label))
                                .title_bar(true)
                                .open(&mut show)
                                .scroll([false, true])
                                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                                .enabled(true)
                                .show(&graphics.get_egui_context(), |ui| {
                                if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Scripting")).clicked() {
                                    log::debug!("Adding scripting component to entity [{}]", label);
                                    if let Err(e) = self.world.insert_one(*entity, ScriptComponent::default()) {
                                        crate::warn!("Failed to add scripting component to entity: {}", e);
                                    } else {
                                        crate::success!("Added the scripting component");
                                    }
                                    local_signal = Some(Signal::None);
                                }
                                if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Camera")).clicked() {
                                    log::debug!("Adding camera component to entity [{}]", label);
                                    log::debug!("Not implemented yet :(");
                                    local_signal = Some(Signal::None);
                                }
                            });
                            if !show {
                                self.signal = Signal::None;
                            }
                            if let Some(signal) = local_signal {
                                self.signal = signal
                            }
                        } else {
                            log_once::warn_once!("Failed to add component to entity: no entity component found");
                        }
                    }
                    EntityType::Light => {
                        if let Ok(light) = self.world.query_one_mut::<&Light>(*entity) {
                            let mut show = true;
                            egui::Window::new(format!("Add component for {}", light.label))
                                .scroll([false, true])
                                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                                .enabled(true)
                                .open(&mut show)
                                .title_bar(true)
                                .show(&graphics.get_egui_context(), |ui| {
                                    if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Scripting")).clicked() {
                                        log::debug!("Adding scripting component to light [{}]", light.label);

                                        crate::success!("Added the scripting component to light [{}]", light.label);
                                        self.signal = Signal::None;
                                    }
                                });
                            if !show {
                                self.signal = Signal::None;
                            }
                        } else {
                            log_once::warn_once!("Failed to add component to light: no light component found");
                        }
                    }
                }
            },
            Signal::RemoveComponent(entity, c_type) => {
                match c_type {
                    ComponentType::Script(_) => {
                        match self.world.remove_one::<ScriptComponent>(*entity) {
                            Ok(component) => {
                                crate::success!("Removed script component from entity {:?}", entity);
                                UndoableAction::push_to_undo(&mut self.undo_stack, UndoableAction::RemoveComponent(*entity, ComponentType::Script(component)));
                            }
                            Err(e) => {
                                crate::warn!("Failed to remove script component from entity: {}", e);
                            }
                        }
                    },
                }
            }
            Signal::CreateEntity => {
                // self.show_add_entity_window = true;
                let mut show = true;
                egui::Window::new("Add Entity")
                    .scroll([false, true])
                    .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                    .enabled(true)
                    .open(&mut show)
                    .title_bar(true)
                    .show(&graphics.get_egui_context(), |ui| {
                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Model")).clicked() {
                            log::debug!("Creating new model");
                            crate::warn!("Instead of using the `Add Entity` window, double click on the imported model in the asset \n\
                            viewer to import a new model, then tweak the settings to how you wish after!");
                            self.signal = Signal::None;
                        }

                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Light")).clicked() {
                            log::debug!("Creating new lighting");
                            let transform = Transform::new();
                            let component = LightComponent::default();
                            let light = Light::new(graphics, &component, &transform, Some("Light"));
                            self.world.spawn((light, component, transform));
                            crate::success!("Created new light");

                            // always ensure the signal is reset after action is dun
                            self.signal = Signal::None;
                        }

                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Plane")).clicked() {
                            log::debug!("Creating new plane");
                            let plane = PlaneBuilder::new()
                                .with_size(500.0, 200.0)
                                .build(
                                    graphics,
                                    PROTO_TEXTURE,
                                    Some("Plane")
                                ).unwrap();
                            let transform = Transform::new();
                            let mut props = ModelProperties::new();
                            props.custom_properties.insert("width".to_string(), PropertyValue::Float(500.0));
                            props.custom_properties.insert("height".to_string(), PropertyValue::Float(200.0));
                            props.custom_properties.insert("tiles_x".to_string(), PropertyValue::Int(500));
                            props.custom_properties.insert("tiles_z".to_string(), PropertyValue::Int(200));
                            self.world.spawn((plane, transform, props));
                            crate::success!("Created new plane");

                            self.signal = Signal::None;
                        }

                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Cube")).clicked() {
                            log::debug!("Creating new cube");
                            let model = Model::load_from_memory(
                                graphics,
                                include_bytes!("../../../resources/cube.obj").to_vec(),
                                Some("Cube")
                            );
                            match model {
                                Ok(model) => {
                                    let cube = AdoptedEntity::adopt(
                                        graphics,
                                        model,
                                        Some("Cube")
                                    );
                                    self.world.spawn((cube, Transform::new(), ModelProperties::new()));
                                }
                                Err(e) => {
                                    crate::fatal!("Failed to load cube model: {}", e);
                                }
                            }
                            crate::success!("Created new cube");

                            self.signal = Signal::None;
                        }
                    });
                if !show {
                    self.signal = Signal::None;
                }
            },
            Signal::LogEntities => {
                log::info!("====================");
                for entity in self.world.iter() {
                    if let Some(entity) = entity.get::<&AdoptedEntity>() {
                        log::info!("Model: {:?}", entity.label());
                    }

                    if let Some(entity) = entity.get::<&Light>() {
                        log::info!("Light: {:?}", entity.label());
                    }
                }
                log::info!("====================");
                self.signal = Signal::None;
            }
        }

        let current_size = graphics.state.viewport_texture.size;
        self.size = current_size;
        
        let new_aspect = current_size.width as f64 / current_size.height as f64;
        if let Some(active_camera) = self.active_camera {
            if let Ok(mut query) = self.world.query_one::<&mut Camera>(active_camera) {
                if let Some(camera) = query.get() {
                    camera.aspect = new_aspect;
                }
            }
        }

        for (_entity_id, (camera, _component, follow_target)) in self
            .world
            .query::<(&mut Camera, &CameraComponent, Option<&CameraFollowTarget>)>()
            .iter()
        {
            if let Some(target) = follow_target {
                for (_target_entity_id, (adopted, transform)) in self
                    .world
                    .query::<(&AdoptedEntity, &Transform)>()
                    .iter()
                {
                    if adopted.label() == &target.follow_target {
                        let target_pos = transform.position;
                        camera.eye = target_pos + target.offset;
                        camera.target = target_pos;
                        break;
                    }
                }
            }
        }

        for (_entity_id, (camera, component)) in self.world.query::<(&mut Camera, &mut CameraComponent)>().iter() {
            component.update(camera);
            camera.update(graphics);
        }

        let query = self.world.query_mut::<(&mut AdoptedEntity, &Transform)>();
        for (_, (entity, transform)) in query {
            entity.update(&graphics, transform);
        }

        let light_query = self.world.query_mut::<(&mut LightComponent, &Transform, &mut Light)>();
        for (_, (light_component, transform, light)) in light_query {
            light.update(light_component, transform);
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
            if let Some(active_camera) = self.active_camera {
                if let Ok(mut query) = self.world.query_one::<&Camera>(active_camera) {
                    if let Some(camera) = query.get() {
                        let mut light_query = self.world.query::<(&Light, &LightComponent)>();
                        let mut entity_query = self.world.query::<(&AdoptedEntity, &Transform)>();
                        {
                            let mut render_pass = graphics.clear_colour(color);

                            if let Some(light_pipeline) = &self.light_manager.pipeline {
                                render_pass.set_pipeline(light_pipeline);
                                for (_, (light, component)) in light_query.iter() {
                                    if component.enabled {
                                        render_pass.set_vertex_buffer(1, light.instance_buffer.as_ref().unwrap().slice(..));
                                        render_pass.draw_light_model(
                                            light.model(),
                                            camera.bind_group(),
                                            light.bind_group(),
                                        );
                                    }
                                }
                            }

                            let mut model_batches: HashMap<*const Model, Vec<InstanceRaw>> = HashMap::new();

                            for (_, (entity, _)) in entity_query.iter() {
                                let model_ptr = entity.model() as *const Model;
                                let instance_raw = entity.instance.to_raw();
                                model_batches.entry(model_ptr).or_insert(Vec::new()).push(instance_raw);
                            }

                            render_pass.set_pipeline(pipeline);

                            for (model_ptr, instances) in model_batches {
                                let model = unsafe { &*model_ptr };

                                let instance_buffer = graphics.state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                    label: Some("Batched Instance Buffer"),
                                    contents: bytemuck::cast_slice(&instances),
                                    usage: wgpu::BufferUsages::VERTEX,
                                });

                                render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                                render_pass.draw_model_instanced(
                                    model,
                                    0..instances.len() as u32,
                                    camera.bind_group(),
                                    self.light_manager.bind_group(),
                                );
                            }
                        }
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