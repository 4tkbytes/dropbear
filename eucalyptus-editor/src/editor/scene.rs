
use super::*;
use dropbear_engine::graphics::{InstanceRaw, RenderContext};
use dropbear_engine::model::{Model, MODEL_CACHE};
use dropbear_engine::procedural::plane::PlaneBuilder;
use dropbear_engine::{
    entity::{AdoptedEntity, Transform},
    lighting::{Light, LightComponent},
    model::{DrawLight, DrawModel},
    scene::{Scene, SceneCommand},
};
use egui::{Align2, Image};
use eucalyptus_core::camera::PlayerCamera;
use eucalyptus_core::states::{Value, WorldLoadingStatus};
use eucalyptus_core::utils::{PROTO_TEXTURE, PendingSpawn};
use eucalyptus_core::{logging, scripting, success_without_console, warn_without_console};
use hecs::Entity;
use log;
use parking_lot::Mutex;
use tokio::sync::mpsc::unbounded_channel;
use wgpu::Color;
use wgpu::util::DeviceExt;
use winit::{event_loop::ActiveEventLoop, keyboard::KeyCode};
use std::sync::LazyLock;

pub static PENDING_SPAWNS: LazyLock<Mutex<Vec<PendingSpawn>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

impl Scene for Editor {
    fn load(&mut self, graphics: &mut RenderContext) {
        let (tx, rx) = unbounded_channel::<WorldLoadingStatus>();
        self.progress_tx = Some(rx);

        let graphics_shared = graphics.shared.clone();
        let world_clone = self._temp_world.clone();
        let active_camera_clone = self.active_camera.clone();
        let project_path_clone = self.project_path.clone();
        let dock_state_clone = Arc::new(Mutex::new(self.dock_state.clone()));

        let handle = self.queue.push(async move {
            if let Err(e) = Self::load_project_config(graphics_shared, Some(tx), world_clone, active_camera_clone, project_path_clone, dock_state_clone).await {
                log::error!("Failed to load project config: {}", e);
            }
        });

        self.world_load_handle = Some(handle);

        self.window = Some(graphics.shared.window.clone());
        self.is_world_loaded.mark_scene_loaded();
    }

    fn update(&mut self, dt: f32, graphics: &mut RenderContext) {
        if !self.is_world_loaded.is_project_ready() {
            log_once::debug_once!("Project is not loaded yet");
            self.show_project_loading_window(&graphics.shared.get_egui_context());
            return;
        } else {
            {
                let world = self._temp_world.lock();
                self.world = *world;
            }
            log_once::debug_once!("Project has loaded successfully");
        }

        if !self.is_world_loaded.is_fully_loaded() {
            log::debug!("Scene is not fully loaded, initializing...");
            // self.load(graphics).await;
            return;
        } else {
            log_once::debug_once!("Scene has fully loaded");
        }

        if !self.is_world_loaded.rendering_loaded && self.is_world_loaded.is_fully_loaded() {
            self.load_wgpu_nerdy_stuff(graphics);
            return;
        }

        if let Some((_, tab)) = self.dock_state.find_active_focused() {
            self.is_viewport_focused = matches!(tab, EditorTab::Viewport);
        } else {
            self.is_viewport_focused = false;
        }

        let spawns_to_process = {
            let mut pending_spawns = PENDING_SPAWNS.lock();
            std::mem::take(&mut *pending_spawns)
        };

        for spawn in spawns_to_process {
            let graphics_shared = graphics.shared.clone();
            let asset_path = spawn.asset_path;
            let asset_name = spawn.asset_name;

            let handle = self.queue.push(async move {
                match AdoptedEntity::new(
                    graphics_shared,
                    asset_path,
                    Some(&asset_name),
                ).await {
                    Ok(adopted) => {
                        Ok((adopted, transform, properties, asset_name))
                    }
                    Err(e) => {
                        log::error!("Failed to spawn {}: {}", asset_name, e);
                        Err(e)
                    }
                }
            });

            self.pending_spawn_handles.push(handle);
        }

        self.is_spawning = true;

        if matches!(self.editor_state, EditorState::Playing) {
            if self.input_state.pressed_keys.contains(&KeyCode::Escape) {
                self.signal = Signal::StopPlaying;
            }

            let mut script_entities = Vec::new();
            {
                for (entity_id, script) in self.world
                    .query::<&mut ScriptComponent>()
                    .iter()
                {
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
            }

            if script_entities.is_empty() {
                log_once::warn_once!("Script entities is empty");
            }

            for (entity_id, script_name) in script_entities {
                if let Err(e) = self.script_manager.update_entity_script(
                    entity_id,
                    &script_name,
                    self.world.clone(),
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

            let active_cam = self.active_camera.lock();
            if let Some(active_camera) = *active_cam {
                let world = self.world;
                if let Ok(mut query) = world
                    .query_one::<(&mut Camera, &CameraComponent)>(active_camera)
                    && let Some((camera, component)) = query.get() {
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

        match &self.signal {
            Signal::Paste(scene_entity) => match &scene_entity.model_path.ref_type {
                dropbear_engine::utils::ResourceReferenceType::None => {
                    warn!("Resource has a reference type of None");
                    self.signal = Signal::None;
                }
                dropbear_engine::utils::ResourceReferenceType::File(reference) => {
                    let cloned = {
                        let project_path = self.project_path.lock();
                        project_path.clone()
                    };
                    if let Some(proj_root) = cloned {
                        match &scene_entity
                            .model_path
                            .to_project_path(proj_root)
                        {
                            Some(v) => {
                                let entity = {
                                    AdoptedEntity::new(
                                        graphics.shared.clone(),
                                        v,
                                        Some(&scene_entity.label),
                                    ).await
                                };

                                match entity
                                {
                                    Ok(adopted) => {
                                        let entity_id = {
                                            self.world.write().spawn((
                                                adopted,
                                                scene_entity.transform,
                                                ModelProperties::default(),
                                            ))
                                        };


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
                            }
                            None => {
                                fatal!(
                                    "Unable to convert resource reference [{}] to project related path",
                                    reference
                                );
                                self.signal = Signal::None;
                            }
                        };
                    } else {
                        fatal!("Project path is not set");
                        self.signal = Signal::None;
                    }
                }
                dropbear_engine::utils::ResourceReferenceType::Bytes(bytes) => {
                    let model = match Model::load_from_memory(
                        graphics.shared.clone(),
                        bytes,
                        Some(&scene_entity.label),
                    )
                    .await
                    {
                        Ok(v) => v,
                        Err(e) => {
                            fatal!("Unable to load from memory: {}", e);
                            self.signal = Signal::None;
                            return;
                        }
                    };
                    let adopted = AdoptedEntity::adopt(graphics.shared.clone(), model).await;

                    let entity_id = {
                        self.world.write().spawn((
                            adopted,
                            scene_entity.transform,
                            ModelProperties::default(),
                        ))
                    };

                    self.selected_entity = Some(entity_id);
                    log::debug!(
                        "Successfully paste-spawned {} with ID {:?}",
                        scene_entity.label,
                        entity_id
                    );

                    success_without_console!("Paste!");
                    self.signal = Signal::Copy(scene_entity.clone());
                }
                _ => {
                    warn!("Unable to copy, not of bytes or path");
                    self.signal = Signal::None;
                    return;
                }
            },
            Signal::Delete => {
                if let Some(sel_e) = &self.selected_entity {
                    {
                        let is_viewport_cam = if let Ok(mut q) = self.world
                            .query_one::<&CameraComponent>(*sel_e)
                        {
                            if let Some(c) = q.get() {
                                matches!(c.camera_type, CameraType::Debug)
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
                            match self.world.write().despawn(*sel_e) {
                                Ok(_) => {
                                    info!("Decimated entity");
                                    self.signal = Signal::None;
                                }
                                Err(e) => {
                                    warn!("Failed to delete entity: {}", e);
                                    self.signal = Signal::None;
                                }
                            }
                            // println!("is world still locked here [{}]: {}", file!(), self.world.is_locked_exclusive())
                        }
                    }
                }
            }
            Signal::Undo => {
                {
                    if let Some(action) = self.undo_stack.pop() {
                        match action.undo(self.world.clone()) {
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

                                let replaced = {
                                    let world = self.world;
                                    if let Ok(mut sc) = world.get::<&mut ScriptComponent>(selected_entity) {
                                        sc.name = new_script.name.clone();
                                        sc.path = new_script.path.clone();
                                        true
                                    } else {
                                        false
                                    }
                                };

                                if !replaced {
                                    match scripting::attach_script_to_entity(
                                        self.world.clone(),
                                        selected_entity,
                                        new_script.clone(),
                                    ) {
                                        Ok(_) => {
                                        }
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
                                }

                                {
                                    if let Err(e) = scripting::convert_entity_to_group(
                                        self.world.clone(),
                                        selected_entity,
                                    ) {
                                        log::warn!("convert_entity_to_group failed (non-fatal): {}", e);
                                    }
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

                        let replaced = {
                            let world = self.world;
                            if let Ok(mut sc) = world.get::<&mut ScriptComponent>(selected_entity) {
                                sc.name = new_script.name.clone();
                                sc.path = new_script.path.clone();
                                true
                            } else {
                                false
                            }
                        };

                        if !replaced {
                            match scripting::attach_script_to_entity(
                                self.world.clone(),
                                selected_entity,
                                new_script.clone(),
                            ) {
                                Ok(_) => {
                                }
                                Err(e) => {
                                    fatal!("Failed to attach new script: {}", e);
                                    self.signal = Signal::None;
                                    return;
                                }
                            }
                        }

                        {
                            if let Err(e) = scripting::convert_entity_to_group(
                                self.world.clone(),
                                selected_entity,
                            ) {
                                log::warn!("convert_entity_to_group failed (non-fatal): {}", e);
                            }
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
                        let mut success = false;
                        let mut comp = ScriptComponent::default();
                        {
                            if let Ok(script) = self.world.write()
                                .remove_one::<ScriptComponent>(selected_entity)
                            {
                                success!("Removed script from entity {:?}", selected_entity);
                                success = true;
                                comp = script.clone();
                            } else {
                                warn!("No script component found on entity {:?}", selected_entity);
                            }
                        }

                        if success {
                            if let Err(e) = scripting::convert_entity_to_group(
                                self.world.clone(),
                                selected_entity,
                            ) {
                                log::warn!("convert_entity_to_group failed (non-fatal): {}", e);
                            }
                            log::debug!("Pushing remove component to undo stack");
                            UndoableAction::push_to_undo(
                                &mut self.undo_stack,
                                UndoableAction::RemoveComponent(
                                    selected_entity,
                                    Box::new(ComponentType::Script(comp)),
                                ),
                            );
                        }
                    } else {
                        warn!("No entity selected to remove script from");
                    }

                    self.signal = Signal::None;
                }
                ScriptAction::EditScript => {
                    if let Some(selected_entity) = self.selected_entity {
                        let script_opt = {
                            let world = self.world;
                            if let Ok(mut q) = world.query_one::<&ScriptComponent>(selected_entity) {
                                q.get().cloned()
                            } else {
                                None
                            }
                        };

                        if let Some(script) = script_opt {
                            match open::that(script.path.clone()) {
                                Ok(()) => {
                                    success!("Opened {}", script.name)
                                }
                                Err(e) => {
                                    warn!("Error while opening {}: {}", script.name, e);
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
                let has_player_camera_target = self
                    .world.read()
                    .query::<(&Camera, &CameraComponent, &CameraFollowTarget)>()
                    .iter()
                    .any(|(_, (_, comp, _))| matches!(comp.camera_type, CameraType::Player));

                if has_player_camera_target {
                    if let Err(e) = self.create_backup() {
                        fatal!("Failed to create play mode backup: {}", e);
                        self.signal = Signal::None;
                        return;
                    }

                    self.editor_state = EditorState::Playing;

                    self.switch_to_player_camera();

                    let mut script_entities = Vec::new();
                    {
                        for (entity_id, script) in self.world
                            .query::<&ScriptComponent>()
                            .iter()
                        {
                            script_entities.push((entity_id, script.clone()));
                        }
                    }

                    for (entity_id, script) in script_entities {
                        log::debug!(
                            "Initialising entity script [{}] from path: {}",
                            script.name,
                            script.path.display()
                        );

                        let bytes = match std::fs::read_to_string(&script.path) {
                            Ok(val) => val,
                            Err(e) => {
                                fatal!(
                                    "Unable to read script {} to bytes because {}",
                                    &script.path.display(),
                                    e
                                );
                                self.signal = Signal::None;
                                return;
                            }
                        };

                        match self.script_manager.load_script(
                            &script
                                .path
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .to_string(),
                            bytes,
                        ) {
                            Ok(script_name) => {
                                if let Err(e) = self.script_manager.init_entity_script(
                                    entity_id,
                                    &script_name,
                                    self.world.clone(),
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
                if let Err(e) = self.restore() {
                    warn!("Failed to restore from play mode backup: {}", e);
                    log::warn!("Failed to restore scene state: {}", e);
                }

                self.editor_state = EditorState::Editing;

                self.switch_to_debug_camera();

                // already kills itself
                // for (entity_id, _) in Arc::get_mut(&mut self.world).unwrap().query::<&ScriptComponent>().iter() {
                //     self.script_manager.remove_entity_script(entity_id);
                // }

                success!("Exited play mode");
                log::info!("Back to the editor you go...");

                self.signal = Signal::None;
            }
            Signal::CameraAction(action) => match action {
                CameraAction::SetPlayerTarget { entity, offset } => {
                    let player_camera = self
                        .world.read()
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
                        if let Ok(mut query) = self.world
                            .query_one::<&AdoptedEntity>(*entity)
                            && let Some(adopted) = query.get() {
                                follow_target = (
                                    true,
                                    CameraFollowTarget {
                                        follow_target: adopted.model.label.to_string(),
                                        offset: *offset,
                                    },
                                );
                            }

                        {
                            if follow_target.0 {
                                let _ = self.world.write()
                                    .insert_one(camera_entity, follow_target);
                                info!("Set player camera target to entity {:?}", entity);
                            }
                        }
                    }
                    self.signal = Signal::None;
                }
                CameraAction::ClearPlayerTarget => {
                    let player_camera = self
                        .world.read()
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
                        {
                            let _ = self.world.write()
                                .remove_one::<CameraFollowTarget>(camera_entity);
                        }
                    }
                    info!("Cleared player camera target");
                    self.signal = Signal::None;
                }
            },
            Signal::AddComponent(entity, e_type) => {
                match e_type {
                    EntityType::Entity => {
                        if let Ok(mut q) = self.world
                            .query_one::<&AdoptedEntity>(*entity)
                        {
                            if let Some(e) = q.get() {
                                let mut local_signal: Option<Signal> = None;
                                let label = e.model.label.clone();
                                let mut show = true;
                                egui::Window::new(format!("Add component for {}", label))
                                    .title_bar(true)
                                    .open(&mut show)
                                    .scroll([false, true])
                                    .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                                    .enabled(true)
                                    .show(&graphics.shared.get_egui_context(), |ui| {
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
                                            {
                                                if let Err(e) = self.world.write()
                                                    .insert_one(*entity, ScriptComponent::default())
                                                {
                                                    warn!(
                                                "Failed to add scripting component to entity: {}",
                                                e
                                            );
                                                } else {
                                                    success!("Added the scripting component");
                                                }
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

                                            let has_camera = self.world
                                                .query_one::<(&Camera, &CameraComponent)>(*entity)
                                                .is_ok();

                                            if has_camera {
                                                warn!(
                                                "Entity [{}] already has a camera component",
                                                label
                                            );
                                            } else {
                                                let camera = Camera::predetermined(
                                                    graphics.shared.clone(),
                                                    Some(&format!("{} Camera", label)),
                                                );
                                                let component = CameraComponent::new();

                                                {
                                                    if let Err(e) = self.world.write()
                                                        .insert(*entity, (camera, component))
                                                    {
                                                        warn!(
                                                    "Failed to add camera component to entity: {}",
                                                    e
                                                );
                                                    } else {
                                                        success!("Added the camera component");
                                                    }
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
                            }
                        } else {
                            log_once::warn_once!(
                                "Failed to add component to entity: no entity component found"
                            );
                        }
                    }
                    EntityType::Light => {
                        {
                            if let Ok(mut q) = self.world
                                .query_one::<&Light>(*entity)
                            {
                                if let Some(light) = q.get() {
                                    let mut show = true;
                                    egui::Window::new(format!("Add component for {}", light.label))
                                        .scroll([false, true])
                                        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                                        .enabled(true)
                                        .open(&mut show)
                                        .title_bar(true)
                                        .show(&graphics.shared.get_egui_context(), |ui| {
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
                                }
                            } else {
                                log_once::warn_once!(
                                "Failed to add component to light: no light component found"
                            );
                            }
                        }
                    }
                    EntityType::Camera => {
                        {
                            if let Ok(mut q) = self.world.write()
                                .query_one::<(&Camera, &CameraComponent)>(*entity)
                            {
                                if let Some((cam, _comp)) = q.get() {
                                    let mut show = true;
                                    egui::Window::new(format!("Add component for {}", cam.label))
                                        .scroll([false, true])
                                        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                                        .enabled(true)
                                        .open(&mut show)
                                        .title_bar(true)
                                        .show(&graphics.shared.get_egui_context(), |ui| {
                                            egui_extras::install_image_loaders(ui.ctx());
                                            ui.add(Image::from_bytes(
                                                "bytes://theres_nothing.jpg",
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
                                }
                            } else {
                                log_once::warn_once!(
                                "Failed to add component to light: no light component found"
                            );
                            }
                        }
                    }
                }
            }
            Signal::RemoveComponent(entity, c_type) =>
            {match &**c_type {
                ComponentType::Script(_) => {
                    match self.world.write()
                        .remove_one::<ScriptComponent>(*entity)
                    {
                        Ok(component) => {
                            success!("Removed script component from entity {:?}", entity);
                            UndoableAction::push_to_undo(
                                &mut self.undo_stack,
                                UndoableAction::RemoveComponent(
                                    *entity,
                                    Box::new(ComponentType::Script(component)),
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
                    if follow.is_some() {
                        match self.world.write().remove::<(
                            Camera,
                            CameraComponent,
                            CameraFollowTarget,
                        )>(
                            *entity
                        ) {
                            Ok(component) => {
                                success!("Removed camera component from entity {:?}", entity);
                                UndoableAction::push_to_undo(
                                    &mut self.undo_stack,
                                    UndoableAction::RemoveComponent(
                                        *entity,
                                        Box::new(ComponentType::Camera(
                                            Box::new(component.0),
                                            component.1,
                                            Some(component.2),
                                        )),
                                    ),
                                );
                            }
                            Err(e) => {
                                warn!("Failed to remove camera component from entity: {}", e);
                            }
                        };
                    } else {
                        match self.world.write()
                            .remove::<(Camera, CameraComponent)>(*entity)
                        {
                            Ok(component) => {
                                success!("Removed camera component from entity {:?}", entity);
                                UndoableAction::push_to_undo(
                                    &mut self.undo_stack,
                                    UndoableAction::RemoveComponent(
                                        *entity,
                                        Box::new(ComponentType::Camera(Box::new(component.0), component.1, None)),
                                    ),
                                );
                            }
                            Err(e) => {
                                warn!("Failed to remove script component from entity: {}", e);
                            }
                        };
                    }
                }
            }},
            Signal::CreateEntity => {
                let mut show = true;
                egui::Window::new("Add Entity")
                    .scroll([false, true])
                    .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                    .enabled(true)
                    .open(&mut show)
                    .title_bar(true)
                    .show(&graphics.shared.get_egui_context(), |ui| {
                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Model")).clicked() {
                            log::debug!("Creating new model");
                            warn!("Instead of using the `Add Entity` window, double click on the imported model in the asset \n\
                            viewer to import a new model, then tweak the settings to how you wish after!");
                            self.signal = Signal::None;
                        }

                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Light")).clicked() {
                            log::debug!("Creating new lighting");
                            self.signal = Signal::Spawn(PendingSpawn2::Light);
                        }

                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Plane")).clicked() {
                            log::debug!("Creating new plane");
                            self.signal = Signal::Spawn(PendingSpawn2::Plane);
                        }

                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Cube")).clicked() {
                            log::debug!("Creating new cube");
                            self.signal = Signal::Spawn(PendingSpawn2::Cube);
                        }

                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Camera")).clicked() {
                            log::debug!("Creating new cube");
                            self.signal = Signal::Spawn(PendingSpawn2::Camera);
                        }
                    });
                if !show {
                    self.signal = Signal::None;
                }
            }
            Signal::LogEntities => {
                log::debug!("====================");
                let mut counter = 0;
                for entity in self.world.iter() {
                    if let Some(entity) = entity.get::<&AdoptedEntity>() {
                        log::info!("Model: {:?}", entity.model.label);
                        log::info!("  |-> Using model: {:?}", entity.model.id);
                    }

                    if let Some(entity) = entity.get::<&Light>() {
                        log::info!("Light: {:?}", entity.cube_model.label);
                        log::info!("  |-> Using model: {:?}", entity.cube_model.id);
                    }

                    if entity.get::<&Camera>().is_some() {
                        log::info!("Camera");
                    }
                    counter += 1;
                }
                log::debug!("====================");
                info!("Total entity count: {}", counter);
                self.signal = Signal::None;
            }
            Signal::Spawn(entity_type) => {
                match entity_type {
                    crate::editor::PendingSpawn2::Light => {
                        let transform = Transform::new();
                        let component = LightComponent::default();
                        let light = Light::new(
                            graphics.shared.clone(),
                            &component,
                            &transform,
                            Some("Light"),
                        )
                        .await;
                        {
                            self.world.write()
                                .spawn((light, component, transform));
                        }
                        success!("Created new light");
                    }
                    crate::editor::PendingSpawn2::Plane => {
                        let plane = PlaneBuilder::new()
                            .with_size(500.0, 200.0)
                            .build(graphics.shared.clone(), PROTO_TEXTURE, Some("Plane"))
                            .await
                            .unwrap();

                        let transform = Transform::new();
                        let mut props = ModelProperties::new();
                        props
                            .custom_properties
                            .insert("width".to_string(), Value::Float(500.0));
                        props
                            .custom_properties
                            .insert("height".to_string(), Value::Float(200.0));
                        props
                            .custom_properties
                            .insert("tiles_x".to_string(), Value::Int(500));
                        props
                            .custom_properties
                            .insert("tiles_z".to_string(), Value::Int(200));
                        {
                            self.world.write()
                                .spawn((plane, transform, props));
                        }
                        success!("Created new plane");
                    }
                    crate::editor::PendingSpawn2::Cube => {
                        let model = Model::load_from_memory(
                            graphics.shared.clone(),
                            include_bytes!("../../../resources/cube.glb"),
                            Some("Cube"),
                        )
                        .await;
                        match model {
                            Ok(model) => {
                                let cube = AdoptedEntity::adopt(
                                    graphics.shared.clone(),
                                    model,
                                    // Some("Cube")
                                )
                                .await;
                                {
                                    self.world.write().spawn((
                                        cube,
                                        Transform::new(),
                                        ModelProperties::new(),
                                    ));
                                }
                            }
                            Err(e) => {
                                fatal!("Failed to load cube model: {}", e);
                            }
                        }
                        success!("Created new cube");
                    }
                    crate::editor::PendingSpawn2::Camera => {
                        let camera = Camera::predetermined(graphics.shared.clone(), None);
                        let component = CameraComponent::new();
                        {
                            self.world.write()
                                .spawn((camera, component));
                        }
                        success!("Created new camera");
                    }
                }
                self.signal = Signal::None;
            }
        }

        let current_size = graphics.shared.viewport_texture.size;
        self.size = current_size;

        let new_aspect = current_size.width as f64 / current_size.height as f64;

        {
            let active_cam = self.active_camera.lock();
            if let Some(active_camera) = *active_cam {
                let world = self.world;
                if let Ok(mut query) = world.query_one::<&mut Camera>(active_camera)
                    && let Some(camera) = query.get() {
                        camera.aspect = new_aspect;
                    }
            }

        }

        let camera_follow_data: Vec<(Entity, String, glam::Vec3)> = {
            let world = self.world;
            world
                .query::<(&Camera, &CameraComponent, Option<&CameraFollowTarget>)>()
                .iter()
                .filter_map(|(entity_id, (_, _, follow_target))| {
                    follow_target.map(|target| {
                        (
                            entity_id,
                            target.follow_target.clone(),
                            target.offset.as_vec3()
                        )
                    })
                })
                .collect()
        };


        for (camera_entity, target_label, offset) in camera_follow_data {
            let target_position = {
                let world = self.world;
                world
                    .query::<(&AdoptedEntity, &Transform)>()
                    .iter()
                    .find_map(|(_, (adopted, transform))| {
                        if adopted.model.label == target_label {
                            Some(transform.position)
                        } else {
                            None
                        }
                    })
            };


            if let Some(pos) = target_position {
                let world = self.world;
                if let Ok(mut query) = world.query_one::<&mut Camera>(camera_entity)
                    && let Some(camera) = query.get() {
                        camera.eye = pos + offset.as_dvec3();
                        camera.target = pos;
                    }
            }

        }

        {
            let world = self.world;
            for (_entity_id, (camera, component)) in world
                .query::<(&mut Camera, &mut CameraComponent)>()
                .iter()
            {
                component.update(camera);
                camera.update(graphics.shared.clone());
            }
        }

        {
            {
                let mut world = self.world.write();
                let query = world
                    .query_mut::<(&mut AdoptedEntity, &Transform)>();
                for (_, (entity, transform)) in query {
                    entity.update(graphics.shared.clone(), transform);
                }
            }


            {
                let mut world = self.world.write();
                let light_query =
                    world
                        .query_mut::<(&mut LightComponent, &Transform, &mut Light)>();
                for (_, (light_component, transform, light)) in light_query {
                    light.update(light_component, transform);
                }
            }
            
        }

        {
            let world = self.world;
            self.light_manager.update(
                graphics.shared.clone(),
                &world,
            );
        }

        if self.dep_installer.is_installing {
            self.dep_installer
                .show_installation_window(&graphics.shared.get_egui_context());
        }
        self.dep_installer.update_progress();
    }

    fn render(&mut self, graphics: &mut RenderContext) {
        // cornflower blue
        let color = Color {
            r: 100.0 / 255.0,
            g: 149.0 / 255.0,
            b: 237.0 / 255.0,
            a: 1.0,
        };

        self.color = color;
        self.size = graphics.shared.viewport_texture.size;
        self.texture_id = Some(*graphics.shared.texture_id.clone());
        { self.show_ui(&graphics.shared.get_egui_context()).await; }

        self.window = Some(graphics.shared.window.clone());
        logging::render(&graphics.shared.get_egui_context());
        if let Some(pipeline) = &self.render_pipeline {
            log_once::debug_once!("Found render pipeline");
            if let Some(active_camera) = *self.active_camera.lock() {
                let cam = {
                    if let Ok(mut query) = self.world.query_one::<&Camera>(active_camera) {
                        query.get().cloned()
                    } else {
                        None
                    }
                };

                if let Some(camera) = cam {
                    let lights = {
                        let world = self.world;
                        let mut lights = Vec::new();
                        let mut light_query = world.query::<(&Light, &LightComponent)>();
                        for (_, (light, comp)) in light_query.iter() {
                            lights.push((light.clone(), comp.clone()));
                        }
                        lights
                    };


                    let entities = {
                        let world = self.world;
                        let mut entities = Vec::new();
                        let mut entity_query = world.query::<&AdoptedEntity>();
                        for (_, entity) in entity_query.iter() {
                            entities.push(entity.clone());
                        }
                        entities
                    };


                    {
                        let mut render_pass = graphics.clear_colour(color);
                        if let Some(light_pipeline) = &self.light_manager.pipeline {
                            render_pass.set_pipeline(light_pipeline);
                            for (light, _component) in &lights {
                                render_pass.set_vertex_buffer(
                                    1,
                                    light.instance_buffer.as_ref().unwrap().slice(..),
                                );
                                render_pass.draw_light_model(
                                    &light.cube_model,
                                    camera.bind_group(),
                                    light.bind_group(),
                                );
                            }
                        }
                    }

                    let mut model_batches: HashMap<ModelId, Vec<InstanceRaw>> =
                        HashMap::new();
                    for entity in &entities {
                        let model_ptr = entity.model.id;
                        let instance_raw = entity.instance.to_raw();
                        model_batches
                            .entry(model_ptr)
                            .or_default()
                            .push(instance_raw);
                    }

                    for (model_ptr, instances) in model_batches {
                        {
                            let model_opt = {
                                let cache = MODEL_CACHE.lock();
                                cache.values().find(|m| m.id == model_ptr).cloned()
                            };

                            if let Some(model) = model_opt {
                                {
                                    let mut render_pass = graphics.continue_pass();
                                    render_pass.set_pipeline(pipeline);

                                    let instance_buffer = graphics.shared.device.create_buffer_init(
                                        &wgpu::util::BufferInitDescriptor {
                                            label: Some("Batched Instance Buffer"),
                                            contents: bytemuck::cast_slice(&instances),
                                            usage: wgpu::BufferUsages::VERTEX,
                                        },
                                    );
                                    render_pass.set_vertex_buffer(1, instance_buffer.slice(..));
                                    render_pass.draw_model_instanced(
                                        &model,
                                        0..instances.len() as u32,
                                        camera.bind_group(),
                                        self.light_manager.bind_group(),
                                    );
                                }
                                log_once::debug_once!("Rendered {:?}", model_ptr);
                            } else {
                                log_once::error_once!("No such MODEL as {:?}", model_ptr);
                            }
                        }
                    }
                } else {
                    log_once::error_once!("Camera returned None");
                }
            } else {
                log_once::error_once!("No active camera found");
            }
        } else {
            log_once::warn_once!("No render pipeline exists");
        }
    }

    fn exit(&mut self, _event_loop: &ActiveEventLoop) {}

    fn run_command(&mut self) -> SceneCommand {
        std::mem::replace(&mut self.scene_command, SceneCommand::None)
    }
}
