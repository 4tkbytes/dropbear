use super::*;
use dropbear_engine::{
    input::{Controller, Keyboard, Mouse},
};
use gilrs::{Button, GamepadId};
use log;
use winit::{dpi::PhysicalPosition, event::MouseButton, event_loop::ActiveEventLoop, keyboard::KeyCode};

impl Keyboard for Editor {
    fn key_down(
        &mut self,
        key: KeyCode,
        _event_loop: &ActiveEventLoop,
    ) {
        match key {
            // KeyCode::Escape => event_loop.exit(),
            KeyCode::Escape => {
                // self.is_cursor_locked = !self.is_cursor_locked;
                // if !self.is_cursor_locked {
                //     if let Some((surface_idx, node_idx, _)) =
                //         self.dock_state.find_tab(&EditorTab::AssetViewer)
                //     {
                //         self.dock_state
                //             .set_focused_node_and_surface((surface_idx, node_idx));
                //     } else {
                //         self.dock_state.push_to_focused_leaf(EditorTab::AssetViewer);
                //     }
                // }
            }
            KeyCode::KeyS => {
                #[cfg(not(target_os = "macos"))]
                let ctrl_pressed = self.pressed_keys.contains(&KeyCode::ControlLeft)
                    || self.pressed_keys.contains(&KeyCode::ControlRight);
                #[cfg(target_os = "macos")]
                let ctrl_pressed = self.pressed_keys.contains(&KeyCode::SuperLeft)
                    || self.pressed_keys.contains(&KeyCode::SuperRight);

                if ctrl_pressed {
                    match self.save_project_config() {
                        Ok(_) => {
                            log::info!("Successfully saved project");
                            if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                toasts.add(egui_toast_fork::Toast {
                                    kind: egui_toast_fork::ToastKind::Success,
                                    text: format!("Successfully saved project").into(),
                                    options: egui_toast_fork::ToastOptions::default()
                                        .duration_in_seconds(5.0)
                                        .show_progress(true),
                                    ..Default::default()
                                });
                            }
                        }
                        Err(e) => {
                            log::error!("Error saving project: {}", e);
                            if let Ok(mut toasts) = GLOBAL_TOASTS.lock() {
                                toasts.add(egui_toast_fork::Toast {
                                    kind: egui_toast_fork::ToastKind::Error,
                                    text: format!("Error saving project: {}", e).into(),
                                    options: egui_toast_fork::ToastOptions::default()
                                        .duration_in_seconds(5.0)
                                        .show_progress(true),
                                    ..Default::default()
                                });
                            }
                        }
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

    fn key_up(
        &mut self,
        key: KeyCode,
        _event_loop: &ActiveEventLoop,
    ) {
        self.pressed_keys.remove(&key);
    }
}

impl Mouse for Editor {
    fn mouse_move(&mut self, _position: PhysicalPosition<f64>) {
        // // if self.is_cursor_locked {
        //     if let Some(window) = &self.window {
        //         let size = window.inner_size();
        //         let center =
        //             PhysicalPosition::new(size.width as f64 / 2.0, size.height as f64 / 2.0);

        //         let dx = position.x - center.x;
        //         let dy = position.y - center.y;
        //         self.camera.track_mouse_delta(dx as f32, dy as f32);

        //         window.set_cursor_position(center).ok();
        //         window.set_cursor_visible(false);
        //     }
        // // }
    }

    fn mouse_down(&mut self, _button: MouseButton) {}

    fn mouse_up(&mut self, _button: MouseButton) {}
}

impl Controller for Editor {
    fn button_down(
        &mut self,
        _button: Button,
        _id: GamepadId,
    ) {
    }

    fn button_up(
        &mut self,
        _button: Button,
        _id: GamepadId,
    ) {
    }

    fn left_stick_changed(&mut self, _x: f32, _y: f32, _id: GamepadId) {
        // used for moving the camera
    }

    fn right_stick_changed(&mut self, _x: f32, _y: f32, _id: GamepadId) {
        // used for moving the player
    }

    fn on_connect(&mut self, _id: GamepadId) {}

    fn on_disconnect(&mut self, _id: GamepadId) {}
}
