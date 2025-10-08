use std::sync::Arc;
use egui::{Align2, Image};
use dropbear_engine::camera::Camera;
use dropbear_engine::entity::{AdoptedEntity, Transform};
use dropbear_engine::graphics::SharedGraphicsContext;
use dropbear_engine::lighting::{Light, LightComponent};
use dropbear_engine::utils::{ResourceReference, ResourceReferenceType};
use eucalyptus_core::states::{ModelProperties, ScriptComponent, Value};
use eucalyptus_core::{fatal, info, scripting, success, success_without_console, warn, warn_without_console};
use eucalyptus_core::camera::{CameraAction, CameraComponent, CameraType};
use eucalyptus_core::spawn::{push_pending_spawn, PendingSpawn};
use crate::editor::{ComponentType, Editor, EditorState, EntityType, PendingSpawn2, Signal, UndoableAction};

pub trait SignalController {
    fn run_signal(&mut self, graphics: Arc<SharedGraphicsContext>) -> anyhow::Result<()>;
}

impl SignalController for Editor {
    fn run_signal(&mut self, graphics: Arc<SharedGraphicsContext>) -> anyhow::Result<()> {
        let mut local_insert_script = false;
        let mut local_insert_camera = (false, String::new());
        let mut local_signal: Option<Signal> = None;
        let mut show = true;

        match &self.signal {
            Signal::None => {
                // returns absolutely nothing because no signal is set. 
                Ok::<(), anyhow::Error>(())
            }
            Signal::Copy(_) => {Ok(())}
            Signal::Paste(scene_entity) => {
                let spawn = PendingSpawn {
                    asset_path: scene_entity.model_path.clone(),
                    asset_name: scene_entity.label.clone(),
                    transform: scene_entity.transform,
                    properties: scene_entity.properties.clone(),
                    handle: None,
                };
                push_pending_spawn(spawn);
                self.signal = Signal::Copy(scene_entity.clone());
                Ok(())
            },
            Signal::Delete => {
                if let Some(sel_e) = &self.selected_entity {
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
                        Ok(())
                    } else {
                        match self.world.despawn(*sel_e) {
                            Ok(_) => {
                                info!("Decimated entity");
                                self.signal = Signal::None;
                                Ok(())
                            }
                            Err(e) => {
                                self.signal = Signal::None;
                                fatal!("Failed to delete entity: {}", e);
                                Err(anyhow::anyhow!(e))
                            }
                        }
                    }
                } else {
                    // no entity has been selected, so all good
                    Ok(())
                }
            }
            Signal::Undo => {
                if let Some(action) = self.undo_stack.pop() {
                    match action.undo(&mut self.world) {
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
                Ok(())
            }
            Signal::Play => {
                if matches!(self.editor_state, EditorState::Playing) {
                    fatal!("Unable to play: already in playing mode");
                    self.signal = Signal::None;
                    return Err(anyhow::anyhow!("Unable to play: already in playing mode"));
                }
                let has_player_camera_target = self
                    .world
                    .query::<(&Camera, &CameraComponent)>()
                    .iter()
                    .any(|(_, (_, comp))| comp.starting_camera);

                if has_player_camera_target {
                    if let Err(e) = self.create_backup() {
                        self.signal = Signal::None;
                        fatal!("Failed to create play mode backup: {}", e);
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
                            "Initialising entity script for entity {:?} with tags: {:?}",
                            entity_id,
                            script.tags
                        );

                        match self.script_manager.load_script() {
                            Ok(_) => {
                                if let Err(e) = self.script_manager.init_entity_script(
                                    entity_id,
                                    script.tags.clone(),
                                    &mut self.world,
                                    &self.input_state,
                                ) {
                                    log::warn!(
                                        "Failed to initialise script for entity {:?}: {}",
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
                                self.signal = Signal::StopPlaying;
                                fatal!("Failed to load script for {:?} with tags {:?} because {}", entity_id, script.tags, e);
                            }
                        }
                    }
                } else {
                    self.signal = Signal::None;
                    fatal!("Unable to build: No initial camera set");
                }

                self.signal = Signal::None;
                Ok(())
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
                Ok(())
            }
            Signal::AddComponent(entity, e_type) => {
                match e_type {
                    EntityType::Entity => {
                        if let Ok(mut q) = self.world
                            .query_one::<&AdoptedEntity>(*entity)
                        {
                            if let Some(e) = q.get() {
                                let label = e.model.label.clone();
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
                                            local_insert_script = true;
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

                                            local_insert_camera = (true, label.clone());
                                            local_signal = Some(Signal::None);
                                        }
                                    });

                            }
                        } else {
                            log_once::warn_once!(
                                "Failed to add component to entity: no entity component found"
                            );
                        }
                        if local_insert_script {
                            if let Err(e) = self.world
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

                        if local_insert_camera.0 {
                            let camera = Camera::predetermined(
                                graphics.clone(),
                                Some(&format!("{} Camera", local_insert_camera.1)),
                            );
                            let component = CameraComponent::new();
                            if let Err(e) = self.world
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
                        Ok(())
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

                                                log::warn!("Its not really added, it's just a dummy button. To be implemented...");

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
                                Ok(())
                            } else {
                                log_once::warn_once!(
                                    "Failed to add component to light: no light component found"
                                );
                                Ok(())
                            }
                        }
                    }
                    EntityType::Camera => {
                        {
                            if let Ok(mut q) = self.world
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
                                        .show(&graphics.get_egui_context(), |ui| {
                                            egui_extras::install_image_loaders(ui.ctx());
                                            ui.add(Image::from_bytes(
                                                "bytes://theres_nothing.jpg",
                                                include_bytes!("../../resources/textures/theres_nothing.jpg"),
                                            ));
                                            ui.label("Theres nothing...");
                                            // scripting could be planned???
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
                                Ok(())
                            } else {
                                log_once::warn_once!(
                                    "Failed to add component to light: no light component found"
                                );
                                Ok(())
                            }
                        }
                    }
                }
            }
            Signal::RemoveComponent(entity, c_type) =>
                {match &**c_type {
                    ComponentType::Script(_) => {
                        match self.world
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
                        Ok(())
                    }
                    ComponentType::Camera(_, _) => {
                        match self.world
                            .remove::<(Camera, CameraComponent)>(*entity)
                        {
                            Ok(component) => {
                                success!("Removed camera component from entity {:?}", entity);
                                UndoableAction::push_to_undo(
                                    &mut self.undo_stack,
                                    UndoableAction::RemoveComponent(
                                        *entity,
                                        Box::new(ComponentType::Camera(Box::new(component.0), component.1)),
                                    ),
                                );
                            }
                            Err(e) => {
                                warn!("Failed to remove script component from entity: {}", e);
                                self.signal = Signal::None;
                            }
                        };
                        self.signal = Signal::None;
                        Ok(())
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
                    .show(&graphics.get_egui_context(), |ui| {
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
                Ok(())
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
                Ok(())
            }
            Signal::Spawn(entity_type) => {
                match entity_type {
                    crate::editor::PendingSpawn2::Light => {
                        let light = Light::new(graphics.clone(), LightComponent::default(), Transform::new(), Some("Default Light"));
                        let handle = graphics.future_queue.push(light);
                        self.alt_pending_spawn_queue.push(handle);
                        success!("Pushed light to queue");
                    }
                    crate::editor::PendingSpawn2::Plane => {
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
                        push_pending_spawn(PendingSpawn {
                            asset_path: ResourceReference::from_reference(ResourceReferenceType::Plane),
                            asset_name: "DefaultPlane".to_string(),
                            transform,
                            properties: props,
                            handle: None,
                        });
                        success!("Pushed plane to queue");
                    }
                    PendingSpawn2::Cube => {
                        let pending = PendingSpawn {
                            asset_path: ResourceReference::from_bytes(include_bytes!("../../resources/models/cube.glb")),
                            asset_name: "Default Cube".to_string(),
                            transform: Default::default(),
                            properties: Default::default(),
                            handle: None,
                        };
                        push_pending_spawn(pending);
                        success!("Pushed cube to queue");
                    }
                    PendingSpawn2::Camera => {
                        let camera = Camera::predetermined(graphics.clone(), None);
                        let component = CameraComponent::new();
                        {
                            self.world
                                .spawn((camera, component));
                        }
                        success!("Pushed camera to queue");
                    }
                }
                self.signal = Signal::None;
                return Ok(());
            }
        }?;
        if !show {
            self.signal = Signal::None;
        }
        if let Some(signal) = local_signal {
            self.signal = signal;
        }
        Ok(())
    }
}