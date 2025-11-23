use crate::editor::{Editor, EditorState, PendingSpawnType, Signal};
use dropbear_engine::camera::Camera;
use dropbear_engine::entity::{EntityTransform, MeshRenderer, Transform};
use dropbear_engine::graphics::SharedGraphicsContext;
use dropbear_engine::lighting::{Light as EngineLight, LightComponent};
use dropbear_engine::utils::{ResourceReference, ResourceReferenceType};
use egui::Align2;
use eucalyptus_core::camera::{CameraComponent, CameraType};
use eucalyptus_core::scene::SceneEntity;
use eucalyptus_core::scripting::{BuildStatus, build_jvm};
use eucalyptus_core::spawn::{PendingSpawn, push_pending_spawn};
use eucalyptus_core::states::{
    EditorTab, Label, ModelProperties, PROJECT, Script, SerializedMeshRenderer,
};
use eucalyptus_core::traits::SerializableComponent;
use eucalyptus_core::{fatal, info, success, success_without_console, warn, warn_without_console};
use std::any::TypeId;
use std::path::PathBuf;
use std::sync::Arc;
use winit::keyboard::KeyCode;

pub trait SignalController {
    fn run_signal(&mut self, graphics: Arc<SharedGraphicsContext>) -> anyhow::Result<()>;
}

impl SignalController for Editor {
    fn run_signal(&mut self, graphics: Arc<SharedGraphicsContext>) -> anyhow::Result<()> {
        let local_signal: Option<Signal> = None;
        let show = true;

        match &self.signal {
            Signal::None => {
                // returns absolutely nothing because no signal is set.
                Ok::<(), anyhow::Error>(())
            }
            Signal::Copy(_) => Ok(()),
            Signal::Paste(scene_entity) => {
                let spawn = PendingSpawn {
                    scene_entity: scene_entity.clone(),
                    handle: None,
                };
                push_pending_spawn(spawn);
                self.signal = Signal::Copy(scene_entity.clone());
                Ok(())
            }
            Signal::Delete => {
                if let Some(sel_e) = &self.selected_entity {
                    let is_viewport_cam =
                        if let Ok(mut q) = self.world.query_one::<&CameraComponent>(*sel_e) {
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

                if matches!(self.editor_state, EditorState::Editing) {
                    log::debug!("Starting build process");
                    let (tx, rx) = crossbeam_channel::unbounded();
                    self.progress_rx = Some(rx);

                    self.build_logs.clear();
                    self.build_progress = 0.0;
                    self.show_build_window = true;
                    self.last_build_error = None;

                    let project_root = {
                        let cfg = PROJECT.read();
                        cfg.project_path.clone()
                    };

                    let project_root = project_root.to_path_buf();
                    let status_tx = tx.clone();

                    let handle = graphics
                        .future_queue
                        .push(async move { build_jvm(project_root, status_tx).await });

                    log::debug!(
                        "Pushed future to future_queue, received handle: {:?}",
                        handle
                    );

                    self.handle_created = Some(handle);

                    self.editor_state = EditorState::Building;
                    log::debug!("Set editor state to EditorState::Building");
                }

                if matches!(self.editor_state, EditorState::Building) {
                    #[cfg(not(target_os = "macos"))]
                    let ctrl_pressed = self
                        .input_state
                        .pressed_keys
                        .contains(&KeyCode::ControlLeft)
                        || self
                            .input_state
                            .pressed_keys
                            .contains(&KeyCode::ControlRight);
                    #[cfg(target_os = "macos")]
                    let ctrl_pressed = self.input_state.pressed_keys.contains(&KeyCode::SuperLeft)
                        || self.input_state.pressed_keys.contains(&KeyCode::SuperRight);

                    let alt_pressed = self.input_state.pressed_keys.contains(&KeyCode::AltLeft)
                        || self.input_state.pressed_keys.contains(&KeyCode::AltRight);

                    // Ctrl+Alt+P skips build process and starts running, such as if using cached jar
                    if ctrl_pressed
                        && alt_pressed
                        && self.input_state.pressed_keys.contains(&KeyCode::KeyP)
                    {
                        if let Some(handle) = self.handle_created {
                            log::debug!("Cancelling build task due to manual intervention");
                            graphics.future_queue.cancel(&handle);
                        } else {
                            log::warn!("No handle was created during this time. Weird...")
                        }

                        let project_root = {
                            let cfg = PROJECT.read();
                            cfg.project_path.clone()
                        };
                        let libs_dir = project_root.join("build").join("libs");
                        if !libs_dir.exists() {
                            let err =
                                "Build succeeded but 'build/libs' directory is missing".to_string();
                            return Err(anyhow::anyhow!(err));
                        }

                        let jar_files: Vec<PathBuf> = std::fs::read_dir(&libs_dir)?
                            .filter_map(|entry| entry.ok().map(|e| e.path()))
                            .filter(|path| {
                                path.extension()
                                    .map_or(false, |ext| ext.eq_ignore_ascii_case("jar"))
                                    && !path
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .contains("-sources")
                                    && !path
                                        .file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .contains("-javadoc")
                            })
                            .collect();

                        if jar_files.is_empty() {
                            let err = "No JAR artifact found in 'build/libs'".to_string();
                            return Err(anyhow::anyhow!(err));
                        }

                        let fat_jar = jar_files.iter().find(|path| {
                            path.file_name()
                                .and_then(|n| n.to_str())
                                .map_or(false, |name| name.contains("-all"))
                        });

                        let jar_path = if let Some(fat) = fat_jar {
                            fat.clone()
                        } else {
                            jar_files
                                .into_iter()
                                .max_by_key(|path| {
                                    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
                                })
                                .unwrap()
                        };

                        info!("Using cached JAR: {}", jar_path.display());

                        self.show_build_window = false;

                        self.load_play_mode(jar_path)?;
                        return Ok(());
                    }

                    let mut local_handle_exchanged: Option<anyhow::Result<PathBuf>> = None;
                    if let Some(rx) = &self.progress_rx {
                        while let Ok(status) = rx.try_recv() {
                            match status {
                                BuildStatus::Started => {
                                    self.build_logs.push("Build started...".to_string());
                                    self.build_progress = 0.1;
                                    log::info!("Build started");
                                }
                                BuildStatus::Building(msg) => {
                                    log::info!("[BUILD] {}", msg);
                                    self.build_logs.push(msg.clone());
                                    self.build_progress = (self.build_progress + 0.01).min(0.9);
                                }
                                BuildStatus::Completed => {
                                    self.build_logs
                                        .push("Build completed successfully!".to_string());
                                    self.build_progress = 1.0;
                                    success_without_console!("Build completed");
                                    log::info!("Build completed successfully!");

                                    if let Some(handle) = self.handle_created {
                                        if let Some(result) = graphics
                                            .future_queue
                                            .exchange_owned_as::<anyhow::Result<PathBuf>>(&handle)
                                        {
                                            local_handle_exchanged = Some(result);
                                        }
                                    } else {
                                        self.signal = Signal::None;
                                        self.show_build_window = false;
                                        self.editor_state = EditorState::Editing;
                                    }
                                }
                                BuildStatus::Failed(_e) => {
                                    let error_msg = format!("Build failed, check logs");
                                    self.build_logs.push(error_msg.clone());

                                    self.build_progress = 0.0;
                                    fatal!("Failed to build gradle, check logs");

                                    self.signal = Signal::None;
                                    self.show_build_window = false;
                                    self.editor_state = EditorState::Editing;
                                    self.dock_state
                                        .push_to_focused_leaf(EditorTab::ErrorConsole);
                                }
                            }
                        }
                    }

                    if self.show_build_window {
                        let mut window_open = true;
                        egui::Window::new("Building Project")
                            .collapsible(false)
                            .resizable(false)
                            .fixed_size([500.0, 400.0])
                            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                            .open(&mut window_open)
                            .show(&graphics.get_egui_context(), |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.heading("Gradle Build Progress");
                                    ui.add_space(10.0);

                                    let progress_bar = egui::ProgressBar::new(self.build_progress)
                                        .show_percentage()
                                        .animate(true);
                                    ui.add(progress_bar);

                                    ui.add_space(15.0);
                                    ui.separator();
                                    ui.add_space(5.0);

                                    ui.heading("Build Log");
                                    ui.add_space(5.0);

                                    egui::ScrollArea::vertical()
                                        .stick_to_bottom(true)
                                        .max_height(200.0)
                                        .auto_shrink([false, false])
                                        .show(ui, |ui| {
                                            for log_line in &self.build_logs {
                                                ui.label(
                                                    egui::RichText::new(log_line)
                                                        .family(egui::FontFamily::Monospace)
                                                        .size(12.0),
                                                );
                                            }

                                            if !self.build_logs.is_empty() {
                                                ui.add_space(10.0);
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "Total log entries: {}",
                                                        self.build_logs.len()
                                                    ))
                                                    .italics()
                                                    .color(egui::Color32::GRAY),
                                                );
                                                ui.label(
                                                    "Tip: Press Ctrl+Alt+P to skip build and start running",
                                                );
                                            }
                                        });

                                    ui.add_space(10.0);
                                });
                            });

                        if !window_open {
                            if let Some(handle) = self.handle_created {
                                log::warn!("Cancelling build task due to window close");
                                graphics.future_queue.cancel(&handle);
                            }

                            self.show_build_window = false;
                            self.handle_created = None;
                            self.progress_rx = None;
                            self.editor_state = EditorState::Editing;
                            self.signal = Signal::None;
                        }
                    }

                    if let Some(result) = local_handle_exchanged {
                        log::debug!("Build future completed, processing result");
                        self.handle_created = None;
                        self.progress_rx = None;

                        match result {
                            Ok(path) => {
                                log::debug!("Path is valid, JAR location as {}", path.display());
                                success!("Build completed successfully!");
                                self.show_build_window = false;

                                self.load_play_mode(path)?;
                            }
                            Err(e) => {
                                let error_msg = format!("Build process error: {}", e);
                                self.build_logs.push(error_msg.clone());
                                self.last_build_error = Some(self.build_logs.join("\n"));

                                fatal!("Failed to ready script manager interface because {}", e);
                                self.signal = Signal::None;
                                self.show_build_window = false;
                                self.show_build_error_window = true;
                                self.editor_state = EditorState::Editing;
                            }
                        }
                    }
                }

                if self.show_build_error_window {
                    if let Some(error_log) = &self.last_build_error {
                        let mut window_open = true;
                        let mut close_clicked = false;

                        egui::Window::new("Build Error")
                            .collapsible(true)
                            .resizable(false)
                            .fixed_size([700.0, 500.0])
                            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
                            .open(&mut window_open)
                            .show(&graphics.get_egui_context(), |ui| {
                                ui.vertical(|ui| {
                                    ui.heading("Build Failed");
                                    ui.add_space(5.0);
                                    ui.label("The Gradle build failed. See the error log below:");
                                    ui.add_space(10.0);
                                    ui.separator();
                                    ui.add_space(10.0);

                                    egui::ScrollArea::both()
                                        .auto_shrink([false, false])
                                        .max_height(300.0)
                                        .show(ui, |ui| {
                                            ui.add(
                                                egui::TextEdit::multiline(&mut error_log.as_str())
                                                    .font(egui::TextStyle::Monospace)
                                                    .desired_width(f32::INFINITY)
                                                    .desired_rows(20),
                                            );
                                        });

                                    ui.add_space(10.0);

                                    if ui.button("Close").clicked() {
                                        close_clicked = true;
                                    }
                                });
                            });

                        if !window_open || close_clicked {
                            self.show_build_error_window = false;
                        }
                    } else {
                        self.show_build_error_window = false;
                    }
                }
                Ok(())
            }
            Signal::StopPlaying => {
                if let Err(e) = self.restore() {
                    warn!("Failed to restore from play mode backup: {}", e);
                    log::warn!("Failed to restore scene state: {}", e);
                }

                self.editor_state = EditorState::Editing;

                self.switch_to_debug_camera();

                success!("Exited play mode");
                log::info!("Back to the editor you go...");

                self.signal = Signal::None;
                Ok(())
            }
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
                            self.signal = Signal::Spawn(PendingSpawnType::Light);
                        }

                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Plane")).clicked() {
                            log::debug!("Creating new plane");
                            self.signal = Signal::Spawn(PendingSpawnType::Plane);
                        }

                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Cube")).clicked() {
                            log::debug!("Creating new cube");
                            self.signal = Signal::Spawn(PendingSpawnType::Cube);
                        }

                        if ui.add_sized([ui.available_width(), 30.0], egui::Button::new("Camera")).clicked() {
                            log::debug!("Creating new cube");
                            self.signal = Signal::Spawn(PendingSpawnType::Camera);
                        }
                    });
                if !show {
                    self.signal = Signal::None;
                }
                Ok(())
            }
            Signal::LogEntities => {
                log::debug!("====================");
                log::info!("world total items: {}", self.world.len());
                log::info!("typeid of Label: {:?}", TypeId::of::<Label>());
                log::info!("typeid of MeshRenderer: {:?}", TypeId::of::<MeshRenderer>());
                log::info!("typeid of Transform: {:?}", TypeId::of::<Transform>());
                log::info!(
                    "typeid of ModelProperties: {:?}",
                    TypeId::of::<ModelProperties>()
                );
                log::info!("typeid of Camera: {:?}", TypeId::of::<Camera>());
                log::info!(
                    "typeid of CameraComponent: {:?}",
                    TypeId::of::<CameraComponent>()
                );
                log::info!(
                    "typeid of Script: {:?}",
                    TypeId::of::<Script>()
                );
                log::info!("typeid of EngineLight: {:?}", TypeId::of::<EngineLight>());
                log::info!(
                    "typeid of LightComponent: {:?}",
                    TypeId::of::<LightComponent>()
                );
                for i in self.world.iter() {
                    log::info!("entity id: {:?}", i.entity().id());
                    log::info!("entity bytes: {:?}", i.entity().to_bits().get());
                    log::info!(
                        "components [{}]: ",
                        i.component_types().collect::<Vec<_>>().len()
                    );
                    let mut comp_builder = String::new();
                    for j in i.component_types() {
                        comp_builder.push_str(format!("{:?} ", j).as_str());
                        if TypeId::of::<Label>() == j {
                            log::info!(" |- Label");
                        }

                        if TypeId::of::<MeshRenderer>() == j {
                            log::info!(" |- MeshRenderer");
                        }

                        if TypeId::of::<Transform>() == j {
                            log::info!(" |- Transform");
                        }

                        if TypeId::of::<ModelProperties>() == j {
                            log::info!(" |- ModelProperties");
                        }

                        if TypeId::of::<Camera>() == j {
                            log::info!(" |- Camera");
                        }

                        if TypeId::of::<CameraComponent>() == j {
                            log::info!(" |- CameraComponent");
                        }

                        if TypeId::of::<Script>() == j {
                            log::info!(" |- Script");
                        }

                        if TypeId::of::<EngineLight>() == j {
                            log::info!(" |- EngineLight");
                        }

                        if TypeId::of::<LightComponent>() == j {
                            log::info!(" |- LightComponent");
                        }
                        log::info!("----------")
                    }
                    log::info!("components (typeid) [{}]: ", comp_builder);
                }
                self.signal = Signal::None;
                Ok(())
            }
            Signal::Spawn(entity_type) => {
                match entity_type {
                    PendingSpawnType::Light => {
                        let light = EngineLight::new(
                            graphics.clone(),
                            LightComponent::default(),
                            Transform::new(),
                            Some("Default Light"),
                        );
                        let handle = graphics.future_queue.push(light);
                        self.light_spawn_queue.push(handle);
                        success!("Pushed light to queue");
                    }
                    PendingSpawnType::Plane => {
                        fatal!("Plane spawning is not yet supported, sorry! (system being remade)");
                        // let transform = Transform::new();
                        // let mut props = ModelProperties::new();
                        // props.add_property("width".to_string(), Value::Float(500.0));
                        // props.add_property("height".to_string(), Value::Float(200.0));
                        // props.add_property("tiles_x".to_string(), Value::Int(500));
                        // props.add_property("tiles_z".to_string(), Value::Int(200));
                        //
                        // push_pending_spawn(PendingSpawn {
                        //     asset_path: ResourceReference::from_reference(
                        //         ResourceReferenceType::Plane,
                        //     ),
                        //     asset_name: "DefaultPlane".to_string(),
                        //     transform,
                        //     properties: props,
                        //     handle: None,
                        // });
                        // success!("Pushed plane to queue");
                    }
                    PendingSpawnType::Cube => {
                        let mut components: Vec<Box<dyn SerializableComponent>> = Vec::new();
                        components.push(Box::new(EntityTransform::default()));
                        components.push(Box::new(SerializedMeshRenderer {
                            handle: ResourceReference::from_reference(ResourceReferenceType::Cube),
                            material_override: Vec::new(),
                        }));
                        components.push(Box::new(ModelProperties::new()));

                        let pending = PendingSpawn {
                            scene_entity: SceneEntity {
                                label: Label::from("Cube"),
                                components,
                                entity_id: None,
                            },
                            handle: None,
                        };
                        push_pending_spawn(pending);
                        success!("Pushed cube to queue");
                    }
                    PendingSpawnType::Camera => {
                        let camera = Camera::predetermined(graphics.clone(), None);
                        let component = CameraComponent::new();
                        {
                            self.world.spawn((camera, component));
                        }
                        success!("Pushed camera to queue");
                    }
                }
                self.signal = Signal::None;
                return Ok(());
            }
            Signal::AddComponent(entity, component_name) => {
                if component_name == "MeshRenderer" {
                    let graphics_clone = graphics.clone();
                    let future = async move {
                        let model= dropbear_engine::model::Model::load_from_memory(
                            graphics_clone.clone(),
                            include_bytes!("../../resources/models/cube.glb"),
                            Some("Cube"),
                        ).await?;
                        Ok::<MeshRenderer, anyhow::Error>(dropbear_engine::entity::MeshRenderer::from_handle(model))
                    };
                    let handle = graphics.future_queue.push(Box::pin(future));
                    self.pending_components.push((*entity, handle));
                    success!("Queued MeshRenderer addition for entity {:?}", entity);
                } else if component_name == "CameraComponent" {
                    let graphics_clone = graphics.clone();
                    let future = async move {
                        let camera = Camera::predetermined(graphics_clone, Some("New Camera"));
                        let component = CameraComponent::new();
                        Ok::<(Camera, CameraComponent), anyhow::Error>((camera, component))
                    };
                    let handle = graphics.future_queue.push(Box::pin(future));
                    self.pending_components.push((*entity, handle));
                    success!("Queued Camera addition for entity {:?}", entity);
                } else {
                    warn!("Unknown component type for AddComponent signal: {}", component_name);
                }
                self.signal = Signal::None;
                Ok(())
            }
            Signal::LoadModel(entity, uri) => {
                let graphics_clone = graphics.clone();
                let uri_clone = uri.clone();
                let future = async move {
                    let path = if uri_clone.starts_with("euca://") {
                        let path_str = uri_clone.trim_start_matches("euca://");
                        let project_path = PROJECT.read().project_path.clone();
                        project_path.join(path_str)
                    } else {
                        PathBuf::from(&uri_clone)
                    };
                    
                    let model = dropbear_engine::model::Model::load(
                        graphics_clone,
                        &path,
                        Some(&uri_clone)
                    ).await?;
                    
                    Ok::<MeshRenderer, anyhow::Error>(dropbear_engine::entity::MeshRenderer::from_handle(model))
                };
                
                let handle = graphics.future_queue.push(Box::pin(future));
                self.pending_components.push((*entity, handle));
                success!("Queued model load for entity {:?} from '{}'", entity, uri);
                self.signal = Signal::None;
                Ok(())
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
