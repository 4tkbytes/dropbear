use super::*;
use dropbear_engine::{
    entity::{AdoptedEntity, Transform},
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
                    if self.selected_entity.is_some() {
                        self.signal = Signal::Delete;
                    } else {
                        warn!("Failed to delete: No entity selected");
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
                    if let Some(entity) = &self.selected_entity {
                        let query = self
                            .world
                            .query_one::<(&AdoptedEntity, &Transform, &ModelProperties)>(*entity);
                        if let Ok(mut q) = query {
                            if let Some((e, t, props)) = q.get() {
                                let s_entity = SceneEntity {
                                    model_path: e.model.path.clone(),
                                    label: e.model.label.clone(),
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
                } else if matches!(self.viewport_mode, ViewportMode::Gizmo) {
                    info!("GizmoMode set to scale");
                    self.gizmo_mode = GizmoMode::all_scale();
                } else {
                    self.input_state.pressed_keys.insert(key);
                }
            }
            KeyCode::KeyV => {
                if ctrl_pressed && !is_playing {
                    match &self.signal {
                        Signal::Copy(entity) => {
                            self.signal = Signal::Paste(entity.clone());
                        }
                        _ => {
                            warn!("Unable to paste: You haven't selected anything!");
                        }
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
                }
            }
            _ => {
                self.input_state.pressed_keys.insert(key);
            }
        }
    }

    fn key_up(&mut self, key: KeyCode, _event_loop: &ActiveEventLoop) {
        self.input_state.pressed_keys.remove(&key);
    }
}

impl Mouse for Editor {
    #[cfg(target_os = "linux")]
    // this impl doesn't have the mouse being recentered back to the center (to be fixed), this works as a usable alternative for now
    fn mouse_move(&mut self, position: PhysicalPosition<f64>) {
        if (self.is_viewport_focused && matches!(self.viewport_mode, ViewportMode::CameraMove))
            || (matches!(self.editor_state, EditorState::Playing)
                && !self.input_state.is_cursor_locked)
        {
            if let Some(last_pos) = self.input_state.last_mouse_pos {
                let dx = position.x - last_pos.0;
                let dy = position.y - last_pos.1;

                if let Some(active_camera) = *self.active_camera.lock()
                    && let Ok(mut q) = self.world.query_one::<(
                        &mut Camera,
                        &CameraComponent,
                        // Option<&CameraFollowTarget>,
                    )>(active_camera)
                    && let Some((camera, _)) = q.get()
                {
                    camera.track_mouse_delta(dx, dy);
                }
            }
            self.input_state.last_mouse_pos = Some((position.x, position.y));
        }

        self.input_state.mouse_pos = (position.x, position.y);
    }

    #[cfg(not(target_os = "linux"))]
    // for some reason, this doesn't work on linux but works on windows (not tested on anywhere else)
    fn mouse_move(&mut self, position: PhysicalPosition<f64>) {
        if ((self.is_viewport_focused && matches!(self.viewport_mode, ViewportMode::CameraMove))
            || (matches!(self.editor_state, EditorState::Playing)
                && !self.input_state.is_cursor_locked))
            && let Some(window) = &self.window
        {
            let size = window.inner_size();
            let center = PhysicalPosition::new(size.width as f64 / 2.0, size.height as f64 / 2.0);

            let distance_from_center =
                ((position.x - center.x).powi(2) + (position.y - center.y).powi(2)).sqrt();

            if distance_from_center > 5.0 {
                let dx = position.x - center.x;
                let dy = position.y - center.y;

                if let Some(active_camera) = *self.active_camera.lock()
                    && let Ok(mut q) = self.world.query_one::<(
                        &mut Camera,
                        &CameraComponent,
                        // Option<&CameraFollowTarget>,
                    )>(active_camera)
                    && let Some((camera, _)) = q.get()
                {
                    camera.track_mouse_delta(dx, dy);
                }

                let _ = window.set_cursor_position(center);
            }

            window.set_cursor_visible(false);
        }

        self.input_state.mouse_pos = (position.x, position.y);
    }

    fn mouse_down(&mut self, button: MouseButton) {
        {
            self.input_state.mouse_button.insert(button);
        }
    }

    fn mouse_up(&mut self, button: MouseButton) {
        self.input_state.mouse_button.remove(&button);
    }
}

impl Controller for Editor {
    fn button_down(&mut self, _button: Button, _id: GamepadId) {}

    fn button_up(&mut self, _button: Button, _id: GamepadId) {}

    fn left_stick_changed(&mut self, _x: f32, _y: f32, _id: GamepadId) {}

    fn right_stick_changed(&mut self, _x: f32, _y: f32, _id: GamepadId) {}

    fn on_connect(&mut self, _id: GamepadId) {}

    fn on_disconnect(&mut self, _id: GamepadId) {}
}
