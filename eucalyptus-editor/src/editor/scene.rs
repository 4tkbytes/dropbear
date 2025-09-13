use super::*;
use dropbear_engine::graphics::InstanceRaw;
use dropbear_engine::model::Model;
use dropbear_engine::starter::plane::PlaneBuilder;
use dropbear_engine::{
    entity::{AdoptedEntity, Transform},
    graphics::{Graphics, Shader},
    lighting::{Light, LightComponent},
    model::{DrawLight, DrawModel},
    scene::{Scene, SceneCommand},
};
use egui::{Align2, Image};
use eucalyptus_core::camera::PlayerCamera;
use eucalyptus_core::states::Value;
use eucalyptus_core::utils::{PROTO_TEXTURE, PendingSpawn};
use eucalyptus_core::{logging, model_ext, scripting, success_without_console, warn_without_console};
use log;
use parking_lot::Mutex;
use wgpu::Color;
use wgpu::util::DeviceExt;
use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode};

pub static PENDING_SPAWNS: LazyLock<Mutex<Vec<PendingSpawn>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

impl Scene for Editor {
    fn load(&mut self, graphics: &mut Graphics) {
        if self.active_camera.is_none() {
            self.load_project_config(graphics).unwrap();
        }

        let shader = Shader::new(
            graphics,
            include_str!("../../../resources/shaders/shader.wgsl"),
            Some("viewport_shader"),
        );

        self.light_manager.create_light_array_resources(graphics);

        let texture_bind_group = &graphics.texture_bind_group().clone();
        if let Some(active_camera) = self.active_camera {
            if let Ok(mut q) = self
                .world
                .query_one::<(&Camera, &CameraComponent, Option<&CameraFollowTarget>)>(
                    active_camera,
                )
            {
                if let Some((camera, _component, _follow_target)) = q.get() {
                    let pipeline = graphics.create_render_pipline(
                        &shader,
                        vec![
                            texture_bind_group,
                            camera.layout(),
                            self.light_manager.layout(),
                        ],
                        None,
                    );
                    self.render_pipeline = Some(pipeline);

                    self.light_manager.create_render_pipeline(
                        graphics,
                        include_str!("../../../resources/shaders/light.wgsl"),
                        camera,
                        Some("Light Pipeline"),
                    );
                } else {
                    log_once::warn_once!(
                        "Unable to fetch the query result of camera: {:?}",
                        active_camera
                    )
                }
            } else {
                log_once::warn_once!(
                    "Unable to query camera, component and option<camerafollowtarget> for active camera: {:?}",
                    active_camera
                );
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
            let mut current_spawn: Option<PendingSpawn> = None;
            for spawn in pending_spawns.drain(..) {
                if let Some(handle_id) = spawn.handle_id {
                    match model_ext::GLOBAL_MODEL_LOADER.get_status(handle_id) {
                        Some(model_ext::ModelLoadingStatus::Loaded) => {
                            match model_ext::GLOBAL_MODEL_LOADER.exchange_by_id(handle_id) {
                                Ok(model) => {
                                    let adopted = AdoptedEntity::adopt(graphics, model, Some(&spawn.asset_name));
                                    let entity_id = Arc::get_mut(&mut self.world).unwrap()
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
                                    log::error!("Failed to exchange model for {}: {}", spawn.asset_name, e);
                                }
                            }
                        }
                        Some(model_ext::ModelLoadingStatus::Failed(error)) => {
                            log::error!("Model loading failed for {}: {}", spawn.asset_name, error);
                        }
                        Some(model_ext::ModelLoadingStatus::Processing) => {
                            current_spawn = Some(spawn);
                        }
                        Some(model_ext::ModelLoadingStatus::NotLoaded) => {
                            log::warn!("Model {} not processed yet", spawn.asset_name);
                            current_spawn = Some(spawn);
                        }
                        None => {
                            log::error!("No handle found for model {}", spawn.asset_name);
                        }
                    }
                } else {
                    match AdoptedEntity::new(graphics, &spawn.asset_path, Some(&spawn.asset_name)) {
                        Ok(adopted) => {
                            let entity_id = Arc::get_mut(&mut self.world).unwrap()
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

            if let Some(s) = current_spawn {
                pending_spawns.push(s);
            }
        }

        if matches!(self.editor_state, EditorState::Playing) {
            if self.input_state.pressed_keys.contains(&KeyCode::Escape) {
                self.signal = Signal::StopPlaying;
            }

            let mut script_entities = Vec::new();
            for (entity_id, script) in Arc::get_mut(&mut self.world).unwrap().query::<&mut ScriptComponent>().iter() {
                log_once::debug_once!(
                    "Script Entity -> id: {:?}, component: {:?}",
                    entity_id,
                    script
                );
                script.name = script
                    .path
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string();
                script_entities.push((entity_id, script.name.clone()));
            }

            if script_entities.is_empty() {
                log_once::warn_once!("Script entities is empty");
            }

            for (entity_id, script_name) in script_entities {
                if let Err(e) = self.script_manager.update_entity_script(
                    entity_id,
                    &script_name,
                    &mut self.world,
                    &self.input_state,
                    dt,
                ) {
                    log_once::warn_once!(
                        "Failed to update script '{}' for entity {:?}: {}",
                        script_name,
                        entity_id,
                        e
                    );
                }
            }
        }

        if self.is_viewport_focused && matches!(self.viewport_mode, ViewportMode::CameraMove)
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
                if let Ok(mut query) = self
                    .world
                    .query_one::<(&mut Camera, &CameraComponent)>(active_camera)
                {
                    if let Some((camera, component)) = query.get() {
                        // Handle keyboard input based on camera type
                        match component.camera_type {
                            CameraType::Debug => {
                                DebugCamera::handle_keyboard_input(camera, &movement_keys);
                                DebugCamera::handle_mouse_input(
                                    camera,
                                    component,
                                    self.input_state.mouse_delta,
                                );
                            }
                            CameraType::Player => {
                                PlayerCamera::handle_keyboard_input(camera, &movement_keys);
                                PlayerCamera::handle_mouse_input(
                                    camera,
                                    component,
                                    self.input_state.mouse_delta,
                                );
                            }
                            CameraType::Normal => {
                                // Handle normal camera input if needed
                                DebugCamera::handle_keyboard_input(camera, &movement_keys);
                                DebugCamera::handle_mouse_input(
                                    camera,
                                    component,
                                    self.input_state.mouse_delta,
                                );
                            }
                        }
                    }
                }
            }
        }

        match &self.signal {
            Signal::Paste(scene_entity) => {
                match &scene_entity.model_path.ref_type {
                    dropbear_engine::utils::ResourceReferenceType::None => {
                        warn!("Resource has a reference type of None");
                        self.signal = Signal::None;
                    },
                    dropbear_engine::utils::ResourceReferenceType::File(reference) => {
                        match &scene_entity.model_path.to_project_path(self.project_path.clone().unwrap()) {
                            Some(v) => {
                                match AdoptedEntity::new(
                                    graphics,
                                    v,
                                    Some(&scene_entity.label),
                                ) {
                                    Ok(adopted) => {
                                        let entity_id = Arc::get_mut(&mut self.world).unwrap().spawn((
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

                                        success_without_console!("Paste!");
                                        self.signal = Signal::Copy(scene_entity.clone());
                                    }
                                    Err(e) => {
                                        warn!("Failed to paste-spawn {}: {}", scene_entity.label, e);
                                    }
                                }
                            },
                            None => {
                                fatal!("Unable to convert resource reference [{}] to project related path", reference);
                                self.signal = Signal::None;
                            },
                        };
                    },
                    dropbear_engine::utils::ResourceReferenceType::Bytes(bytes) => {
                        let model = match Model::load_from_memory(graphics, bytes, Some(&scene_entity.label)) {
                            Ok(v) => v,
                            Err(e) => {
                                fatal!("Unable to load from memory: {}", e);
                                self.signal = Signal::None;
                                return;
                            },
                        };
                        let adopted = AdoptedEntity::adopt(
                            graphics,
                            model,
                            Some(&scene_entity.label),
                        );
                        let entity_id = Arc::get_mut(&mut self.world).unwrap().spawn((
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

                        success_without_console!("Paste!");
                        self.signal = Signal::Copy(scene_entity.clone());
                    },
                    _ => {
                        warn!("Unable to copy, not of bytes or path");
                        self.signal = Signal::None;
                        return;
                    }
                }
            }
            Signal::Delete => {
                if let Some(sel_e) = &self.selected_entity {
                    let is_viewport_cam =
                        if let Ok(mut q) = Arc::get_mut(&mut self.world).unwrap().query_one::<&CameraComponent>(*sel_e) {
                            if let Some(c) = q.get() {
                                if matches!(c.camera_type, CameraType::Debug) {
                                    true
                                } else {
                                    false
                                }
                            } else {
                                false
                            }
                        } else {
                            false
                        };
                    if is_viewport_cam {
                        warn!("You can't delete the viewport camera");
                        self.signal = Signal::None;
                    } else {
                        match Arc::get_mut(&mut self.world).unwrap().despawn(*sel_e) {
                            Ok(_) => {
                                info!("Decimated entity");
                                self.signal = Signal::None;
                            }
                            Err(e) => {
                                warn!("Failed to delete entity: {}", e);
                                self.signal = Signal::None;
                            }
                        }
                    }
                }
            }
            Signal::Undo => {
                if let Some(action) = self.undo_stack.pop() {
                    match action.undo(&mut Arc::get_mut(&mut self.world).unwrap()) {
                        Ok(_) => {
                            info!("Undid action");
                        }
                        Err(e) => {
                            warn!("Failed to undo action: {}", e);
                        }
                    }
                } else {
                    warn_without_console!("Nothing to undo");
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
                        match scripting::move_script_to_src(script_path) {
                            Ok(moved_path) => {
                                let new_script = ScriptComponent {
                                    name: script_name.clone(),
                                    path: moved_path.clone(),
                                };

                                let replaced = if let Ok(mut sc) =
                                    Arc::get_mut(&mut self.world).unwrap().get::<&mut ScriptComponent>(selected_entity)
                                {
                                    sc.name = new_script.name.clone();
                                    sc.path = new_script.path.clone();
                                    true
                                } else {
                                    match scripting::attach_script_to_entity(
                                        &mut Arc::get_mut(&mut self.world).unwrap(),
                                        selected_entity,
                                        new_script.clone(),
                                    ) {
                                        Ok(_) => false,
                                        Err(e) => {
                                            fatal!(
                                                "Failed to attach script to entity {:?}: {}",
                                                selected_entity,
                                                e
                                            );
                                            self.signal = Signal::None;
                                            return;
                                        }
                                    }
                                };

                                if let Err(e) =
                                    scripting::convert_entity_to_group(&Arc::get_mut(&mut self.world).unwrap(), selected_entity)
                                {
                                    log::warn!("convert_entity_to_group failed (non-fatal): {}", e);
                                }

                                success!(
                                    "{} script '{}' at {} to entity {:?}",
                                    if replaced { "Reattached" } else { "Attached" },
                                    script_name,
                                    moved_path.display(),
                                    selected_entity
                                );
                            }
                            Err(e) => {
                                fatal!("Move failed: {}", e);
                            }
                        }
                    } else {
                        fatal!("AttachScript requested but no entity is selected");
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
                            Arc::get_mut(&mut self.world).unwrap().get::<&mut ScriptComponent>(selected_entity)
                        {
                            sc.name = new_script.name.clone();
                            sc.path = new_script.path.clone();
                            true
                        } else {
                            match scripting::attach_script_to_entity(
                                Arc::get_mut(&mut self.world).unwrap(),
                                selected_entity,
                                new_script.clone(),
                            ) {
                                Ok(_) => false,
                                Err(e) => {
                                    fatal!("Failed to attach new script: {}", e);
                                    self.signal = Signal::None;
                                    return;
                                }
                            }
                        };

                        if let Err(e) =
                            scripting::convert_entity_to_group(&Arc::get_mut(&mut self.world).unwrap(), selected_entity)
                        {
                            log::warn!("convert_entity_to_group failed (non-fatal): {}", e);
                        }

                        success!(
                            "{} new script '{}' at {} to entity {:?}",
                            if replaced { "Replaced" } else { "Attached" },
                            script_name,
                            script_path.display(),
                            selected_entity
                        );
                    } else {
                        warn_without_console!("No selected entity to attach new script");
                        log::warn!("CreateAndAttachScript requested but no entity is selected");
                    }
                    self.signal = Signal::None;
                }
                ScriptAction::RemoveScript => {
                    if let Some(selected_entity) = self.selected_entity {
                        if let Ok(script) =
                            Arc::get_mut(&mut self.world).unwrap().remove_one::<ScriptComponent>(selected_entity)
                        {
                            success!("Removed script from entity {:?}", selected_entity);

                            if let Err(e) =
                                scripting::convert_entity_to_group(&Arc::get_mut(&mut self.world).unwrap(), selected_entity)
                            {
                                log::warn!("convert_entity_to_group failed (non-fatal): {}", e);
                            }
                            log::debug!("Pushing remove component to undo stack");
                            UndoableAction::push_to_undo(
                                &mut self.undo_stack,
                                UndoableAction::RemoveComponent(
                                    selected_entity,
                                    ComponentType::Script(script),
                                ),
                            );
                        } else {
                            warn!("No script component found on entity {:?}", selected_entity);
                        }
                    } else {
                        warn!("No entity selected to remove script from");
                    }

                    self.signal = Signal::None;
                }
                ScriptAction::EditScript => {
                    if let Some(selected_entity) = self.selected_entity {
                        if let Ok(mut q) = Arc::get_mut(&mut self.world).unwrap().query_one::<&ScriptComponent>(selected_entity)
                        {
                            if let Some(script) = q.get() {
                                match open::that(script.path.clone()) {
                                    Ok(()) => {
                                        success!("Opened {}", script.name)
                                    }
                                    Err(e) => {
                                        warn!("Error while opening {}: {}", script.name, e);
                                    }
                                }
                            }
                        } else {
                            warn!("No script component found on entity {:?}", selected_entity);
                        }
                    } else {
                        warn!("No entity selected to edit script");
                    }
                    self.signal = Signal::None;
                }
            },
            Signal::Play => {
                // Check if a player camera target exists
                let has_player_camera_target = self
                    .world
                    .query::<(&Camera, &CameraComponent, &CameraFollowTarget)>()
                    .iter()
                    .any(|(_, (_, comp, _))| matches!(comp.camera_type, CameraType::Player));

                if has_player_camera_target {
                    if let Err(e) = PlayModeBackup::create_backup(self) {
                        fatal!("Failed to create play mode backup: {}", e);
                        self.signal = Signal::None;
                        return;
                    }

                    self.editor_state = EditorState::Playing;

                    self.switch_to_player_camera();

                    let mut script_entities = Vec::new();
                    for (entity_id, script) in Arc::get_mut(&mut self.world).unwrap().query::<&ScriptComponent>().iter() {
                        script_entities.push((entity_id, script.clone()));
                    }

                    for (entity_id, script) in script_entities {
                        log::debug!(
                            "Initialising entity script [{}] from path: {}",
                            script.name,
                            script.path.display()
                        );

                        let bytes = match std::fs::read(&script.path) {
                            Ok(val) => val,
                            Err(e) => {
                                fatal!("Unable to read script {} to bytes because {}", &script.path.display(), e);
                                self.signal = Signal::None;
                                return;
                            },
                        };
                        
                        match self.script_manager.load_script(&script.path.file_name().unwrap().to_string_lossy().to_string(), bytes) {
                            Ok(script_name) => {
                                if let Err(e) = self.script_manager.init_entity_script(
                                    entity_id,
                                    &script_name,
                                    &mut self.world,
                                    &self.input_state,
                                ) {
                                    log::warn!(
                                        "Failed to initialise script '{}' for entity {:?}: {}",
                                        script.name,
                                        entity_id,
                                        e
                                    );
                                    self.signal = Signal::StopPlaying;
                                } else {
                                    success_without_console!(
                                        "You are in play mode now! Press Escape to exit"
                                    );
                                    log::info!("You are in play mode now! Press Escape to exit");
                                }
                            }
                            Err(e) => {
                                // todo: proper error menu
                                fatal!("Failed to load script '{}': {}", script.name, e);
                                self.signal = Signal::StopPlaying;
                            }
                        }
                    }
                } else {
                    fatal!("Unable to build: Player camera not attached to an entity");
                }

                self.signal = Signal::None;
            }
            Signal::StopPlaying => {
                if let Err(e) = PlayModeBackup::restore(self) {
                    warn!("Failed to restore from play mode backup: {}", e);
                    log::warn!("Failed to restore scene state: {}", e);
                }

                self.editor_state = EditorState::Editing;

                self.switch_to_debug_camera();

                for (entity_id, _) in Arc::get_mut(&mut self.world).unwrap().query::<&ScriptComponent>().iter() {
                    self.script_manager.remove_entity_script(entity_id);
                }

                success!("Exited play mode");
                log::info!("Back to the editor you go...");

                self.signal = Signal::None;
            }
            Signal::CameraAction(action) => match action {
                CameraAction::SetPlayerTarget { entity, offset } => {
                    // Find player camera and add/update CameraFollowTarget component
                    let player_camera = self
                        .world
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
                        if let Ok(mut query) = Arc::get_mut(&mut self.world).unwrap().query_one::<&AdoptedEntity>(*entity) {
                            if let Some(adopted) = query.get() {
                                follow_target = (
                                    true,
                                    CameraFollowTarget {
                                        follow_target: adopted.label().to_string(),
                                        offset: *offset,
                                    },
                                );
                            }
                        }

                        if follow_target.0 {
                            let _ = Arc::get_mut(&mut self.world).unwrap().insert_one(camera_entity, follow_target);
                            info!("Set player camera target to entity {:?}", entity);
                        }
                    }
                    self.signal = Signal::None;
                }
                CameraAction::ClearPlayerTarget => {
                    let player_camera = self
                        .world
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
                        let _ = Arc::get_mut(&mut self.world).unwrap().remove_one::<CameraFollowTarget>(camera_entity);
                    }
                    info!("Cleared player camera target");
                    self.signal = Signal::None;
                }
            },
            Signal::AddComponent(entity, e_type) => {
                match e_type {
                    EntityType::Entity => {
                        if let Ok(e) = Arc::get_mut(&mut self.world).unwrap().query_one_mut::<&AdoptedEntity>(*entity) {
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
                                    if ui
                                        .add_sized(
                                            [ui.available_width(), 30.0],
                                            egui::Button::new("Scripting"),
                                        )
                                        .clicked()
                                    {
                                        log::debug!(
                                            "Adding scripting component to entity [{}]",
                                            label
                                        );
                                        if let Err(e) = Arc::get_mut(&mut self.world).unwrap()
                                            .insert_one(*entity, ScriptComponent::default())
                                        {
                                            warn!(
                                                "Failed to add scripting component to entity: {}",
                                                e
                                            );
                                        } else {
                                            success!("Added the scripting component");
                                        }
                                        local_signal = Some(Signal::None);
                                    }
                                    if ui
                                        .add_sized(
                                            [ui.available_width(), 30.0],
                                            egui::Button::new("Camera"),
                                        )
                                        .clicked()
                                    {
                                        log::debug!(
                                            "Adding camera component to entity [{}]",
                                            label
                                        );

                                        let has_camera = self
                                            .world
                                            .query_one::<(&Camera, &CameraComponent)>(*entity)
                                            .is_ok();

                                        if has_camera {
                                            warn!(
                                                "Entity [{}] already has a camera component",
                                                label
                                            );
                                        } else {
                                            let camera = Camera::predetermined(
                                                graphics,
                                                Some(&format!("{} Camera", label)),
                                            );
                                            let component = CameraComponent::new();

                                            if let Err(e) =
                                                Arc::get_mut(&mut self.world).unwrap().insert(*entity, (camera, component))
                                            {
                                                warn!(
                                                    "Failed to add camera component to entity: {}",
                                                    e
                                                );
                                            } else {
                                                success!("Added the camera component");
                                            }
                                        }
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
                            log_once::warn_once!(
                                "Failed to add component to entity: no entity component found"
                            );
                        }
                    }
                    EntityType::Light => {
                        if let Ok(light) = Arc::get_mut(&mut self.world).unwrap().query_one_mut::<&Light>(*entity) {
                            let mut show = true;
                            egui::Window::new(format!("Add component for {}", light.label))
                                .scroll([false, true])
                                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                                .enabled(true)
                                .open(&mut show)
                                .title_bar(true)
                                .show(&graphics.get_egui_context(), |ui| {
                                    if ui
                                        .add_sized(
                                            [ui.available_width(), 30.0],
                                            egui::Button::new("Scripting"),
                                        )
                                        .clicked()
                                    {
                                        log::debug!(
                                            "Adding scripting component to light [{}]",
                                            light.label
                                        );

                                        success!(
                                            "Added the scripting component to light [{}]",
                                            light.label
                                        );
                                        self.signal = Signal::None;
                                    }
                                });
                            if !show {
                                self.signal = Signal::None;
                            }
                        } else {
                            log_once::warn_once!(
                                "Failed to add component to light: no light component found"
                            );
                        }
                    }
                    EntityType::Camera => {
                        if let Ok((cam, _comp)) = Arc::get_mut(&mut self.world).unwrap()
                            .query_one_mut::<(&Camera, &CameraComponent)>(*entity)
                        {
                            let mut show = true;
                            egui::Window::new(format!("Add component for {}", cam.label))
                                .scroll([false, true])
                                .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                                .enabled(true)
                                .open(&mut show)
                                .title_bar(true)
                                .show(&graphics.get_egui_context(), |ui| {
                                    egui_extras::install_image_loaders(ui.ctx());
                                    ui.add(Image::from_bytes(
                                        "bytes://theres_nothing",
                                        include_bytes!("../../../resources/theres_nothing.jpg"),
                                    ));
                                    ui.label("Theres nothing...");
                                    // // scripting
                                    // if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Scripting")).clicked() {
                                    //     log::debug!("Adding scripting component to camera [{}]", cam.label);

                                    //     success!("Added the scripting component to camera [{}]", cam.label);
                                    //     self.signal = Signal::None;
                                    // }
                                });
                            if !show {
                                self.signal = Signal::None;
                            }
                        } else {
                            log_once::warn_once!(
                                "Failed to add component to light: no light component found"
                            );
                        }
                    }
                }
            }
            Signal::RemoveComponent(entity, c_type) => match c_type {
                ComponentType::Script(_) => {
                    match Arc::get_mut(&mut self.world).unwrap().remove_one::<ScriptComponent>(*entity) {
                        Ok(component) => {
                            success!("Removed script component from entity {:?}", entity);
                            UndoableAction::push_to_undo(
                                &mut self.undo_stack,
                                UndoableAction::RemoveComponent(
                                    *entity,
                                    ComponentType::Script(component),
                                ),
                            );
                        }
                        Err(e) => {
                            warn!("Failed to remove script component from entity: {}", e);
                        }
                    };
                    self.signal = Signal::None;
                }
                ComponentType::Camera(_, _, follow) => {
                    if let Some(_) = follow {
                        match Arc::get_mut(&mut self.world).unwrap()
                            .remove::<(Camera, CameraComponent, CameraFollowTarget)>(*entity)
                        {
                            Ok(component) => {
                                success!("Removed camera component from entity {:?}", entity);
                                UndoableAction::push_to_undo(
                                    &mut self.undo_stack,
                                    UndoableAction::RemoveComponent(
                                        *entity,
                                        ComponentType::Camera(
                                            component.0,
                                            component.1,
                                            Some(component.2),
                                        ),
                                    ),
                                );
                            }
                            Err(e) => {
                                warn!("Failed to remove camera component from entity: {}", e);
                            }
                        };
                    } else {
                        match Arc::get_mut(&mut self.world).unwrap().remove::<(Camera, CameraComponent)>(*entity) {
                            Ok(component) => {
                                success!("Removed camera component from entity {:?}", entity);
                                UndoableAction::push_to_undo(
                                    &mut self.undo_stack,
                                    UndoableAction::RemoveComponent(
                                        *entity,
                                        ComponentType::Camera(component.0, component.1, None),
                                    ),
                                );
                            }
                            Err(e) => {
                                warn!("Failed to remove script component from entity: {}", e);
                            }
                        };
                    }
                }
            },
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
                            warn!("Instead of using the `Add Entity` window, double click on the imported model in the asset \n\
                            viewer to import a new model, then tweak the settings to how you wish after!");
                            self.signal = Signal::None;
                        }

                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Light")).clicked() {
                            log::debug!("Creating new lighting");
                            let transform = Transform::new();
                            let component = LightComponent::default();
                            let light = Light::new(graphics, &component, &transform, Some("Light"));
                            Arc::get_mut(&mut self.world).unwrap().spawn((light, component, transform));
                            success!("Created new light");

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
                            props.custom_properties.insert("width".to_string(), Value::Float(500.0));
                            props.custom_properties.insert("height".to_string(), Value::Float(200.0));
                            props.custom_properties.insert("tiles_x".to_string(), Value::Int(500));
                            props.custom_properties.insert("tiles_z".to_string(), Value::Int(200));
                            Arc::get_mut(&mut self.world).unwrap().spawn((plane, transform, props));
                            success!("Created new plane");

                            self.signal = Signal::None;
                        }

                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Cube")).clicked() {
                            log::debug!("Creating new cube");
                            let model = Model::load_from_memory(
                                graphics,
                                include_bytes!("../../../resources/cube.glb"),
                                Some("Cube")
                            );
                            match model {
                                Ok(model) => {
                                    let cube = AdoptedEntity::adopt(
                                        graphics,
                                        model,
                                        Some("Cube")
                                    );
                                    Arc::get_mut(&mut self.world).unwrap().spawn((cube, Transform::new(), ModelProperties::new()));
                                }
                                Err(e) => {
                                    fatal!("Failed to load cube model: {}", e);
                                }
                            }
                            success!("Created new cube");

                            self.signal = Signal::None;
                        }

                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Camera")).clicked() {
                            log::debug!("Creating new cube");
                            let camera = Camera::predetermined(graphics, None);
                            let component = CameraComponent::new();
                            Arc::get_mut(&mut self.world).unwrap().spawn((camera, component));
                            success!("Created new camera");

                            self.signal = Signal::None;
                        }
                    });
                if !show {
                    self.signal = Signal::None;
                }
            }
            Signal::LogEntities => {
                log::info!("====================");
                for entity in Arc::get_mut(&mut self.world).unwrap().iter() {
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
            if let Ok(mut query) = Arc::get_mut(&mut self.world).unwrap().query_one::<&mut Camera>(active_camera) {
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
                for (_target_entity_id, (adopted, transform)) in
                    self.world.query::<(&AdoptedEntity, &Transform)>().iter()
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

        for (_entity_id, (camera, component)) in self
            .world
            .query::<(&mut Camera, &mut CameraComponent)>()
            .iter()
        {
            component.update(camera);
            camera.update(graphics);
        }

        let query = Arc::get_mut(&mut self.world).unwrap().query_mut::<(&mut AdoptedEntity, &Transform)>();
        for (_, (entity, transform)) in query {
            entity.update(&graphics, transform);
        }

        let light_query = Arc::get_mut(&mut self.world).unwrap()
            .query_mut::<(&mut LightComponent, &Transform, &mut Light)>();
        for (_, (light_component, transform, light)) in light_query {
            light.update(light_component, transform);
        }

        self.light_manager.update(graphics, &Arc::get_mut(&mut self.world).unwrap());
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
        logging::render(&graphics.get_egui_context());
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
                                        render_pass.set_vertex_buffer(
                                            1,
                                            light.instance_buffer.as_ref().unwrap().slice(..),
                                        );
                                        render_pass.draw_light_model(
                                            light.model(),
                                            camera.bind_group(),
                                            light.bind_group(),
                                        );
                                    }
                                }
                            }

                            let mut model_batches: HashMap<*const Model, Vec<InstanceRaw>> =
                                HashMap::new();

                            for (_, (entity, _)) in entity_query.iter() {
                                let model_ptr = entity.model() as *const Model;
                                let instance_raw = entity.instance.to_raw();
                                model_batches
                                    .entry(model_ptr)
                                    .or_insert(Vec::new())
                                    .push(instance_raw);
                            }

                            render_pass.set_pipeline(pipeline);

                            for (model_ptr, instances) in model_batches {
                                let model = unsafe { &*model_ptr };

                                let instance_buffer = graphics.state.device.create_buffer_init(
                                    &wgpu::util::BufferInitDescriptor {
                                        label: Some("Batched Instance Buffer"),
                                        contents: bytemuck::cast_slice(&instances),
                                        usage: wgpu::BufferUsages::VERTEX,
                                    },
                                );

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
