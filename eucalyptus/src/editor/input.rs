use super::*;
use dropbear_engine::{
    entity::{AdoptedEntity, Transform},
    input::{Controller, Keyboard, Mouse},
};
use gilrs::{Button, GamepadId};
use log;
use winit::{
    dpi::PhysicalPosition, event::MouseButton, event_loop::ActiveEventLoop, keyboard::KeyCode,
};

impl Keyboard for Editor {
    fn key_down(&mut self, key: KeyCode, _event_loop: &ActiveEventLoop) {
        #[cfg(not(target_os = "macos"))]
        let ctrl_pressed = self.pressed_keys.contains(&KeyCode::ControlLeft)
            || self.pressed_keys.contains(&KeyCode::ControlRight);
        #[cfg(target_os = "macos")]
        let ctrl_pressed = self.pressed_keys.contains(&KeyCode::SuperLeft)
            || self.pressed_keys.contains(&KeyCode::SuperRight);

        let _alt_pressed = self.pressed_keys.contains(&KeyCode::AltLeft)
            || self.pressed_keys.contains(&KeyCode::AltRight);

        let shift_pressed = self.pressed_keys.contains(&KeyCode::ShiftLeft)
            || self.pressed_keys.contains(&KeyCode::ShiftRight);

        let is_double_press = self.is_double_key_press(key);

        match key {
            KeyCode::KeyG => {
                if self.is_viewport_focused {
                    self.viewport_mode = crate::utils::ViewportMode::Gizmo;
                    crate::info!("Switched to Viewport::Gizmo");
                    
                    if let Some(window) = &self.window {
                        window.set_cursor_visible(true);
                    }
                } else {
                    self.pressed_keys.insert(key);
                }
            }
            KeyCode::KeyF => {
                if self.is_viewport_focused {
                    self.viewport_mode = crate::utils::ViewportMode::CameraMove;
                    crate::info!("Switched to Viewport::CameraMove");
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
                    self.pressed_keys.insert(key);
                }
            }
            KeyCode::Delete => {
                if let Some(_) = &self.selected_entity {
                    self.signal = Signal::Delete;
                } else {
                    crate::warn!("Failed to delete: No entity selected");
                }
            }
            KeyCode::Escape => {
                if is_double_press {
                    if let Some(_) = &self.selected_entity {
                        self.selected_entity = None;
                        log::debug!("deselected entity");
                    }
                } else if self.is_viewport_focused {
                    self.viewport_mode = crate::utils::ViewportMode::None;
                    crate::warn!("Switched to Viewport::None");
                    if let Some(window) = &self.window {
                        window.set_cursor_visible(true);
                    }
                } else {
                    self.pressed_keys.insert(key);
                }
            }
            KeyCode::KeyQ => {
                if ctrl_pressed {
                    match self.save_project_config() {
                        Ok(_) => {}
                        Err(e) => {
                            crate::fatal!("Error saving project: {}", e);
                        }
                    }
                    log::info!("Successfully saved project, about to quit...");
                    crate::success!("Successfully saved project");
                }
            }
            KeyCode::KeyC => {
                if ctrl_pressed {
                    if let Some(entity) = &self.selected_entity {
                        let query = self
                            .world
                            .query_one::<(&AdoptedEntity, &Transform, &ModelProperties)>(*entity);
                        if let Ok(mut q) = query {
                            if let Some((e, t, props)) = q.get() {
                                let s_entity = crate::states::SceneEntity {
                                    model_path: e.model().path.clone(),
                                    label: e.model().label.clone(),
                                    transform: *t,
                                    properties: props.clone(),
                                    script: None,
                                    entity_id: None,
                                };
                                self.signal = Signal::Copy(s_entity);

                                crate::info!("Copied!");

                                log::debug!("Copied selected entity");
                            } else {
                                crate::warn!("Unable to copy entity: Unable to fetch world entity properties");
                            }
                        } else {
                            crate::warn!("Unable to copy entity: Unable to obtain lock");
                        }
                    } else {
                        crate::warn!("Unable to copy entity: None selected");
                        self.pressed_keys.insert(key);
                    }
                }
            }
            KeyCode::KeyV => {
                if ctrl_pressed {
                    match &self.signal {
                        Signal::Copy(entity) => {
                            self.signal = Signal::Paste(entity.clone());
                        }
                        _ => {
                            crate::warn!("Unable to paste: You haven't selected anything!");
                        }
                    }
                } else {
                    self.pressed_keys.insert(key);
                }
            }
            KeyCode::KeyS => {
                if ctrl_pressed {
                    match self.save_project_config() {
                        Ok(_) => {
                            crate::success!("Successfully saved project");
                        }
                        Err(e) => {
                            crate::fatal!("Error saving project: {}", e);
                        }
                    }
                } else {
                    self.pressed_keys.insert(key);
                }
            }
            KeyCode::KeyZ => {
                if ctrl_pressed {
                    if shift_pressed {
                        // redo
                    } else {
                        // undo
                        log::debug!("Undo signal sent");
                        self.signal = Signal::Undo;
                    }
                } else {
                    self.pressed_keys.insert(key);
                }
            }
            _ => {
                self.pressed_keys.insert(key);
            }
        }
    }

    fn key_up(&mut self, key: KeyCode, _event_loop: &ActiveEventLoop) {
        self.pressed_keys.remove(&key);
    }
}

impl Mouse for Editor {
    fn mouse_move(&mut self, position: PhysicalPosition<f64>) {
        if self.is_viewport_focused
            && matches!(self.viewport_mode, crate::utils::ViewportMode::CameraMove)
        {
            if let Some(window) = &self.window {
                let size = window.inner_size();
                let center =
                    PhysicalPosition::new(size.width as f64 / 2.0, size.height as f64 / 2.0);

                let dx = position.x - center.x;
                let dy = position.y - center.y;
                self.camera.track_mouse_delta(dx, dy);

                let _ = window.set_cursor_position(center);
                window.set_cursor_visible(false);
            }
        }
    }

    fn mouse_down(&mut self, _button: MouseButton) {}

    fn mouse_up(&mut self, _button: MouseButton) {}
}

impl Controller for Editor {
    fn button_down(&mut self, button: Button, _id: GamepadId) {
        match button {
            Button::South => {
                self.camera.move_up();
            }
            Button::East => {
                self.camera.move_down();
            }
            Button::LeftTrigger2 => {
                self.camera.move_up();
            }
            Button::RightTrigger2 => {
                self.camera.move_down();
            }
            _ => {
                log::debug!("Controller button pressed: {:?}", button);
            }
        }
    }

    fn button_up(&mut self, _button: Button, _id: GamepadId) {}

    fn left_stick_changed(&mut self, _x: f32, _y: f32, _id: GamepadId) {}

    fn right_stick_changed(&mut self, _x: f32, _y: f32, _id: GamepadId) {}

    fn on_connect(&mut self, _id: GamepadId) {}

    fn on_disconnect(&mut self, _id: GamepadId) {}
}
