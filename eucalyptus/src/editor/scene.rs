use std::path::PathBuf;

use dropbear_engine::{
    entity::{AdoptedEntity, Transform},
    graphics::{Graphics, Shader},
    scene::{Scene, SceneCommand},
};
use log;
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
        let camera = self.load_project_config(graphics).unwrap();

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
                // let script = ScriptComponent {
                //     name: "DummyScript".to_string(),
                //     path: PathBuf::from("dummy/path/to/script.rs"),
                // };
                self.world.spawn((
                    cube,
                    Transform::default(),
                    ModelProperties::default(),
                    // script
                ));
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

        let texture_bind_group = &graphics.texture_bind_group().clone();

        let model_layout = graphics.create_model_uniform_bind_group_layout();
        let pipeline = graphics.create_render_pipline(
            &shader,
            vec![texture_bind_group, camera.layout(), &model_layout],
        );

        self.camera = camera;
        self.render_pipeline = Some(pipeline);
        self.window = Some(graphics.state.window.clone());
    }

    fn update(&mut self, _dt: f32, graphics: &mut Graphics) {
        if let Some((_, tab)) = self.dock_state.find_active_focused() {
            self.is_viewport_focused = matches!(tab, EditorTab::Viewport);
        } else {
            self.is_viewport_focused = false;
        }

        if let Ok(mut pending_spawns) = PENDING_SPAWNS.lock() {
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

        if self.is_viewport_focused
            && matches!(self.viewport_mode, crate::utils::ViewportMode::CameraMove)
        {
            for key in &self.pressed_keys {
                match key {
                    KeyCode::KeyW => self.camera.move_forwards(),
                    KeyCode::KeyA => self.camera.move_left(),
                    KeyCode::KeyD => self.camera.move_right(),
                    KeyCode::KeyS => self.camera.move_back(),
                    KeyCode::ShiftLeft => self.camera.move_down(),
                    KeyCode::Space => self.camera.move_up(),
                    _ => {}
                }
            }
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

                        if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                            toasts.add(egui_toast_fork::Toast {
                                kind: egui_toast_fork::ToastKind::Success,
                                text: format!("Paste!").into(),
                                options: egui_toast_fork::ToastOptions::default()
                                    .duration_in_seconds(1.0)
                                    .show_progress(false),
                                ..Default::default()
                            });
                        }
                        self.signal = Signal::Copy(scene_entity.clone());
                    }
                    Err(e) => {
                        if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                            toasts.add(egui_toast_fork::Toast {
                                kind: egui_toast_fork::ToastKind::Warning,
                                text: format!(
                                    "Failed to paste-spawn {}: {}",
                                    scene_entity.label, e
                                )
                                .into(),
                                options: egui_toast_fork::ToastOptions::default()
                                    .duration_in_seconds(3.0)
                                    .show_progress(true),
                                ..Default::default()
                            });
                        }
                        log::error!("Failed to paste-spawn {}: {}", scene_entity.label, e);
                    }
                }
            }
            Signal::Delete => {
                if let Some(sel_e) = &self.selected_entity {
                    match self.world.despawn(*sel_e) {
                        Ok(_) => {
                            if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                toasts.add(egui_toast_fork::Toast {
                                    kind: egui_toast_fork::ToastKind::Success,
                                    text: format!("Decimated entity").into(),
                                    options: egui_toast_fork::ToastOptions::default()
                                        .duration_in_seconds(3.0)
                                        .show_progress(true),
                                    ..Default::default()
                                });
                            }
                            self.signal = Signal::None;
                        }
                        Err(e) => {
                            if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                toasts.add(egui_toast_fork::Toast {
                                    kind: egui_toast_fork::ToastKind::Warning,
                                    text: format!("Failed to delete entity: {}", e).into(),
                                    options: egui_toast_fork::ToastOptions::default()
                                        .duration_in_seconds(3.0)
                                        .show_progress(true),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                }
            }
            Signal::Undo => {
                if let Some(action) = self.undo_stack.pop() {
                    match action.undo(&mut self.world) {
                        Ok(_) => {
                            if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                toasts.add(egui_toast_fork::Toast {
                                    kind: egui_toast_fork::ToastKind::Success,
                                    text: format!("Undid action").into(),
                                    options: egui_toast_fork::ToastOptions::default()
                                        .duration_in_seconds(1.0)
                                        .show_progress(false),
                                    ..Default::default()
                                });
                            }
                        }
                        Err(e) => {
                            if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                toasts.add(egui_toast_fork::Toast {
                                    kind: egui_toast_fork::ToastKind::Warning,
                                    text: format!("Failed to undo action: {}", e).into(),
                                    options: egui_toast_fork::ToastOptions::default()
                                        .duration_in_seconds(3.0)
                                        .show_progress(true),
                                    ..Default::default()
                                });
                            }
                        }
                    }
                } else {
                    if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                        toasts.add(egui_toast_fork::Toast {
                            kind: egui_toast_fork::ToastKind::Warning,
                            text: format!("Nothing to undo").into(),
                            options: egui_toast_fork::ToastOptions::default()
                                .duration_in_seconds(1.0)
                                .show_progress(false),
                            ..Default::default()
                        });
                    }
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
                                            if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                                toasts.add(egui_toast_fork::Toast {
                                                    kind: egui_toast_fork::ToastKind::Error,
                                                    text: format!("Failed to attach: {}", e).into(),
                                                    options: egui_toast_fork::ToastOptions::default()
                                                        .duration_in_seconds(3.0)
                                                        .show_progress(true),
                                                    ..Default::default()
                                                });
                                            }
                                            log::error!(
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
                                    log::warn!(
                                        "convert_entity_to_group failed (non-fatal): {}",
                                        e
                                    );
                                }

                                if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                    toasts.add(egui_toast_fork::Toast {
                                        kind: egui_toast_fork::ToastKind::Success,
                                        text: if replaced {
                                            format!("Reattached script '{}'", script_name)
                                        } else {
                                            format!("Attached script '{}'", script_name)
                                        }
                                        .into(),
                                        options: egui_toast_fork::ToastOptions::default()
                                            .duration_in_seconds(2.5)
                                            .show_progress(true),
                                        ..Default::default()
                                    });
                                }

                                log::info!(
                                    "{} script '{}' at {:?} to entity {:?}",
                                    if replaced { "Reattached" } else { "Attached" },
                                    script_name,
                                    moved_path,
                                    selected_entity
                                );
                            }
                            Err(e) if e.downcast_ref::<std::io::Error>().map_or(false, |io_err| io_err.kind() == std::io::ErrorKind::AlreadyExists) => {
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
                                            if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                                toasts.add(egui_toast_fork::Toast {
                                                    kind: egui_toast_fork::ToastKind::Error,
                                                    text: format!("Failed to attach: {}", e).into(),
                                                    options: egui_toast_fork::ToastOptions::default()
                                                        .duration_in_seconds(3.0)
                                                        .show_progress(true),
                                                    ..Default::default()
                                                });
                                            }
                                            log::error!(
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
                                    log::warn!(
                                        "convert_entity_to_group failed (non-fatal): {}",
                                        e
                                    );
                                }

                                if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                    toasts.add(egui_toast_fork::Toast {
                                        kind: egui_toast_fork::ToastKind::Success,
                                        text: if replaced {
                                            format!("Reattached script '{}'", script_name)
                                        } else {
                                            format!("Attached script '{}'", script_name)
                                        }
                                        .into(),
                                        options: egui_toast_fork::ToastOptions::default()
                                            .duration_in_seconds(2.5)
                                            .show_progress(true),
                                        ..Default::default()
                                    });
                                }

                                log::info!(
                                    "{} script '{}' at {:?} to entity {:?}",
                                    if replaced { "Reattached" } else { "Attached" },
                                    script_name,
                                    script_path.clone(),
                                    selected_entity
                                );
                            }
                            Err(e) => {
                                if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                    toasts.add(egui_toast_fork::Toast {
                                        kind: egui_toast_fork::ToastKind::Error,
                                        text: format!("Move failed: {}", e).into(),
                                        options: egui_toast_fork::ToastOptions::default()
                                            .duration_in_seconds(3.0)
                                            .show_progress(true),
                                        ..Default::default()
                                    });
                                }
                                log::error!(
                                    "Failed to move script {}: {}",
                                    script_path.display(),
                                    e
                                );
                            }
                        }
                    } else {
                        if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                            toasts.add(egui_toast_fork::Toast {
                                kind: egui_toast_fork::ToastKind::Warning,
                                text: "No selected entity to attach script".into(),
                                options: egui_toast_fork::ToastOptions::default()
                                    .duration_in_seconds(2.0)
                                    .show_progress(false),
                                ..Default::default()
                            });
                        }
                        log::warn!("AttachScript requested but no entity is selected");
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
                                    if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                        toasts.add(egui_toast_fork::Toast {
                                            kind: egui_toast_fork::ToastKind::Error,
                                            text: format!("Failed to attach new script: {}", e).into(),
                                            options: egui_toast_fork::ToastOptions::default()
                                                .duration_in_seconds(3.0)
                                                .show_progress(true),
                                            ..Default::default()
                                        });
                                    }
                                    log::error!(
                                        "Failed to attach newly created script to entity {:?}: {}",
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

                        if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                            toasts.add(egui_toast_fork::Toast {
                                kind: egui_toast_fork::ToastKind::Success,
                                text: if replaced {
                                    format!("Replaced script with new '{}'", script_name)
                                } else {
                                    format!("Created & attached script '{}'", script_name)
                                }
                                .into(),
                                options: egui_toast_fork::ToastOptions::default()
                                    .duration_in_seconds(2.5)
                                    .show_progress(true),
                                ..Default::default()
                            });
                        }

                        log::info!(
                            "{} new script '{}' at {:?} to entity {:?}",
                            if replaced { "Replaced" } else { "Attached" },
                            script_name,
                            script_path,
                            selected_entity
                        );
                    } else {
                        if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                            toasts.add(egui_toast_fork::Toast {
                                kind: egui_toast_fork::ToastKind::Warning,
                                text: "No selected entity to attach new script".into(),
                                options: egui_toast_fork::ToastOptions::default()
                                    .duration_in_seconds(2.0)
                                    .show_progress(false),
                                ..Default::default()
                            });
                        }
                        log::warn!("CreateAndAttachScript requested but no entity is selected");
                    }
                    self.signal = Signal::None;
                }
                ScriptAction::RemoveScript => {
                    // log::debug!("Not implemented: RemoveScript");

                    // delete scene


                    self.signal = Signal::None;
                }
                ScriptAction::ExecuteScript => {
                    log::debug!("Not implemented: ExecuteScript");
                    self.signal = Signal::None;
                }
                ScriptAction::EditScript => {
                    log::debug!("Not implemented: EditScript");
                    self.signal = Signal::None;
                }
            },
        }

        let new_size = graphics.state.viewport_texture.size;
        let new_aspect = new_size.width as f64 / new_size.height as f64;
        self.camera.aspect = new_aspect;

        self.camera.update(graphics);

        let query = self.world.query_mut::<(&mut AdoptedEntity, &Transform)>();
        for (_, (entity, transform)) in query {
            entity.update(&graphics, transform);
        }
    }

    fn render(&mut self, graphics: &mut Graphics) {
        let color = Color {
            r: 0.1,
            g: 0.2,
            b: 0.3,
            a: 1.0,
        };
        self.color = color.clone();
        self.size = graphics.state.viewport_texture.size.clone();
        self.texture_id = Some(graphics.state.texture_id.clone());
        self.show_ui(&graphics.get_egui_context());

        self.window = Some(graphics.state.window.clone());
        if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
            toasts.show(graphics.get_egui_context());
        }
        if let Some(pipeline) = &self.render_pipeline {
            {
                let mut query = self.world.query::<(&AdoptedEntity, &Transform)>();
                let mut render_pass = graphics.clear_colour(color);
                render_pass.set_pipeline(pipeline);

                for (_, (entity, _)) in query.iter() {
                    entity.render(&mut render_pass, &self.camera);
                }
            }
        }
    }

    fn exit(&mut self, _event_loop: &ActiveEventLoop) {}

    fn run_command(&mut self) -> SceneCommand {
        std::mem::replace(&mut self.scene_command, SceneCommand::None)
    }
}
