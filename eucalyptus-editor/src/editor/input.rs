use super::*;
use dropbear_engine::{
    entity::{MeshRenderer, Transform},
    input::{Controller, Keyboard, Mouse},
};
use eucalyptus_core::success_without_console;
use gilrs::{Button, GamepadId};
use log;
use transform_gizmo_egui::GizmoMode;
use winit::{
    dpi::PhysicalPosition, event::MouseButton, event_loop::ActiveEventLoop, keyboard::KeyCode,
};

impl Keyboard for Editor {
    fn key_down(&mut self, key: KeyCode, _event_loop: &ActiveEventLoop) {
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

        let _alt_pressed = self.input_state.pressed_keys.contains(&KeyCode::AltLeft)
            || self.input_state.pressed_keys.contains(&KeyCode::AltRight);

        let shift_pressed = self.input_state.pressed_keys.contains(&KeyCode::ShiftLeft)
            || self.input_state.pressed_keys.contains(&KeyCode::ShiftRight);

        let is_double_press = self.double_key_pressed(key);

        let is_playing = matches!(self.editor_state, EditorState::Playing);

        // template
        // if let Some((_, tab)) = self.dock_state.find_active_focused()
        //      && matches!(tab, EditorTab::Viewport)
        // {
        //
        // }

        match key {
            KeyCode::KeyG => {
                if self.is_viewport_focused && !is_playing {
                    self.viewport_mode = ViewportMode::Gizmo;
                    info!("Switched to Viewport::Gizmo");

                    if let Some(window) = &self.window {
                        window.set_cursor_visible(true);
                    }
                } else {
                    self.input_state.pressed_keys.insert(key);
                }
            }
            KeyCode::KeyF => {
                if self.is_viewport_focused && !is_playing {
                    self.viewport_mode = ViewportMode::CameraMove;
                    info!("Switched to Viewport::CameraMove");
                    if let Some(window) = &self.window {
                        window.set_cursor_visible(false);

                        let size = window.inner_size();
                        let center = winit::dpi::PhysicalPosition::new(
                            size.width as f64 / 2.0,
                            size.height as f64 / 2.0,
                        );
                        let _ = window.set_cursor_position(center);
                    }
                } else {
                    self.input_state.pressed_keys.insert(key);
                }
            }
            KeyCode::Delete => {
                if !is_playing {
                    if let Some((_, tab)) = self.dock_state.find_active_focused()
                        && matches!(tab, EditorTab::ModelEntityList)
                    {
                        if self.selected_entity.is_some() {
                            self.signal = Signal::Delete;
                        } else {
                            warn!("Failed to delete: No entity selected");
                        }
                    }
                } else {
                    self.input_state.pressed_keys.insert(key);
                }
            }
            KeyCode::Escape => {
                if is_double_press {
                    if self.selected_entity.is_some() {
                        self.selected_entity = None;
                        log::debug!("Deselected entity");
                    }
                } else if self.is_viewport_focused && !is_playing {
                    self.viewport_mode = ViewportMode::None;
                    info!("Switched to Viewport::None");
                    if let Some(window) = &self.window {
                        window.set_cursor_visible(true);
                        let _ = window.set_cursor_grab(CursorGrabMode::None);
                    }
                } else {
                    self.input_state.pressed_keys.insert(key);
                }
            }
            KeyCode::KeyQ => {
                if ctrl_pressed && !is_playing {
                    match self.save_project_config() {
                        Ok(_) => {}
                        Err(e) => {
                            fatal!("Error saving project: {}", e);
                        }
                    }
                    log::info!("Successfully saved project, about to quit...");
                    success_without_console!("Successfully saved project");
                    self.scene_command = SceneCommand::Quit;
                } else if is_playing {
                    warn!(
                        "Unable to save-quit project, please pause your playing state, then try again"
                    );
                }
            }
            KeyCode::KeyC => {
                if ctrl_pressed && !is_playing {
                    if let Some((_, tab)) = self.dock_state.find_active_focused()
                        && matches!(tab, EditorTab::ModelEntityList)
                    {
                        if let Some(entity) = &self.selected_entity {
                            let query = self
                                .world
                                .query_one::<(&MeshRenderer, &Transform, &ModelProperties)>(
                                    *entity,
                                );
                            if let Ok(mut q) = query {
                                if let Some((renderer, t, props)) = q.get() {
                                    let s_entity = SceneEntity {
                                        model_path: renderer.handle().path.clone(),
                                        label: renderer.handle().label.clone(),
                                        transform: *t,
                                        properties: props.clone(),
                                        script: None,
                                        entity_id: None,
                                        camera: None,
                                    };
                                    self.signal = Signal::Copy(s_entity);

                                    info!("Copied!");

                                    log::debug!("Copied selected entity");
                                } else {
                                    warn!(
                                        "Unable to copy entity: Unable to fetch world entity properties"
                                    );
                                }
                            } else {
                                warn!("Unable to copy entity: Unable to obtain lock");
                            }
                        } else {
                            warn!("Unable to copy entity: None selected");
                        }
                    }
                } else if matches!(self.viewport_mode, ViewportMode::Gizmo) {
                    info!("GizmoMode set to scale");
                    self.gizmo_mode = GizmoMode::all_scale();
                } else {
                    self.input_state.pressed_keys.insert(key);
                }
            }
            KeyCode::KeyV => {
                if ctrl_pressed && !is_playing {
                    if let Signal::Copy(entity) = &self.signal {
                        self.signal = Signal::Paste(entity.clone());
                    }
                } else {
                    self.input_state.pressed_keys.insert(key);
                }
            }
            KeyCode::KeyS => {
                if ctrl_pressed {
                    if !is_playing {
                        match self.save_project_config() {
                            Ok(_) => {
                                success!("Successfully saved project");
                            }
                            Err(e) => {
                                fatal!("Error saving project: {}", e);
                            }
                        }
                    } else {
                        warn!(
                            "Unable to save project config, please quit your playing and try again"
                        );
                    }
                } else {
                    self.input_state.pressed_keys.insert(key);
                }
            }
            KeyCode::KeyZ => {
                if ctrl_pressed && !is_playing {
                    if shift_pressed {
                        // redo
                        info!("Redo not implemented yet, please report this")
                    } else {
                        // undo
                        log::debug!("Undo signal sent");
                        self.signal = Signal::Undo;
                    }
                } else if matches!(self.viewport_mode, ViewportMode::Gizmo) && !is_playing {
                    info!("GizmoMode set to translate");
                    self.gizmo_mode = GizmoMode::all_translate();
                } else {
                    self.input_state.pressed_keys.insert(key);
                }
            }
            KeyCode::F1 => {
                if !is_playing {
                    if self.is_using_debug_camera() {
                        self.switch_to_player_camera();
                    } else {
                        self.switch_to_debug_camera();
                    }
                } else {
                    self.input_state.pressed_keys.insert(key);
                }
            }
            KeyCode::KeyX => {
                if matches!(self.viewport_mode, ViewportMode::Gizmo) && !is_playing {
                    info!("GizmoMode set to rotate");
                    self.gizmo_mode = GizmoMode::all_rotate();
                } else {
                    self.input_state.pressed_keys.insert(key);
                }
            }
            KeyCode::KeyP => {
                if !is_playing && ctrl_pressed {
                    self.signal = Signal::Play
                } else {
                    self.input_state.pressed_keys.insert(key);
                }
            }
            _ => {
                self.input_state.pressed_keys.insert(key);
            }
        }
        self.input_state.pressed_keys.insert(key);
    }

    fn key_up(&mut self, key: KeyCode, _event_loop: &ActiveEventLoop) {
        self.input_state.pressed_keys.remove(&key);
    }
}

impl Mouse for Editor {
    fn mouse_move(&mut self, position: PhysicalPosition<f64>, delta: Option<(f64, f64)>) {
        if self.is_viewport_focused && matches!(self.viewport_mode, ViewportMode::CameraMove) {
            if let Some(window) = &self.window {
                window.set_cursor_visible(false);
                if let Err(e) = window.set_cursor_grab(CursorGrabMode::Locked).or_else(|_| {
                    log_once::warn_once!("Using cursor grab fallback: CursorGrabMode::Locked");
                    window.set_cursor_grab(CursorGrabMode::Confined)
                }) {
                    log_once::error_once!("Unable to grab mouse: {}", e);
                }
            }

            if let Some(active_camera) = *self.active_camera.lock()
                && let Ok(mut q) = self
                    .world
                    .query_one::<(&mut Camera, &CameraComponent)>(active_camera)
                && let Some((camera, _)) = q.get()
            {
                if let Some((dx, dy)) = delta {
                    camera.track_mouse_delta(
                        dx * camera.settings.sensitivity,
                        dy * camera.settings.sensitivity,
                    );
                    self.input_state.mouse_delta = Some((dx, dy));
                } else {
                    log_once::warn_once!("Unable to track mouse delta, attempting fallback");
                    // fallback for mouse tracking
                    if let Some(old_mouse_pos) = self.input_state.last_mouse_pos {
                        let dx = position.x - old_mouse_pos.0;
                        let dy = position.y - old_mouse_pos.1;
                        camera.track_mouse_delta(
                            dx * camera.settings.sensitivity,
                            dy * camera.settings.sensitivity,
                        );
                        self.input_state.mouse_delta = Some((dx, dy));
                        log_once::debug_once!("Fallback mouse tracking used");
                    } else {
                        log_once::error_once!("Unable to track mouse delta, fallback failed");
                    }
                }
            }
            self.input_state.last_mouse_pos = Some(<(f64, f64)>::from(position));
        } else {
            if !matches!(self.editor_state, EditorState::Playing) {
                if let Some(window) = &self.window {
                    window.set_cursor_visible(true);
                    if let Err(e) = window.set_cursor_grab(CursorGrabMode::None) {
                        log_once::error_once!("Unable to release mouse grab: {}", e);
                    }
                }
            } else {
                // if it is in play mode, cursor grab would be defined in the user script
            }
            self.input_state.last_mouse_pos = Some(<(f64, f64)>::from(position));
        }

        self.input_state.mouse_delta = delta;
        self.input_state.mouse_pos = (position.x, position.y);
    }

    fn mouse_down(&mut self, button: MouseButton) {
        self.input_state.mouse_button.insert(button);
    }

    fn mouse_up(&mut self, button: MouseButton) {
        self.input_state.mouse_button.remove(&button);
    }
}

impl Controller for Editor {
    fn button_down(&mut self, button: Button, id: GamepadId) {
        self.input_state
            .pressed_buttons
            .entry(id)
            .or_default()
            .insert(button);
    }

    fn button_up(&mut self, button: Button, id: GamepadId) {
        if let Some(buttons) = self.input_state.pressed_buttons.get_mut(&id) {
            buttons.remove(&button);
        }
    }

    fn left_stick_changed(&mut self, x: f32, y: f32, id: GamepadId) {
        self.input_state.left_stick_position.insert(id, (x, y));
    }

    fn right_stick_changed(&mut self, x: f32, y: f32, id: GamepadId) {
        self.input_state.right_stick_position.insert(id, (x, y));
    }

    fn on_connect(&mut self, id: GamepadId) {
        self.input_state.connected_gamepads.insert(id);
    }

    fn on_disconnect(&mut self, id: GamepadId) {
        self.input_state.connected_gamepads.remove(&id);
        self.input_state.pressed_buttons.remove(&id);
        self.input_state.left_stick_position.remove(&id);
        self.input_state.right_stick_position.remove(&id);
    }
}
