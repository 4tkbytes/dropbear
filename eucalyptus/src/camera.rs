use std::{collections::HashSet, time::Instant};

use dropbear_engine::camera::Camera;
use egui::{CollapsingHeader, Ui};
use glam::DVec3;
use hecs::Entity;
use serde::{Deserialize, Serialize};
use winit::keyboard::KeyCode;

use crate::editor::{component::Component, EntityType, Signal, StaticallyKept, UndoableAction};

#[derive(Debug, Clone)]
pub struct CameraComponent {
    pub speed: f64,
    pub sensitivity: f64,
    pub fov_y: f64,
    pub camera_type: CameraType
}

impl CameraComponent {
    pub fn new() -> Self {
        Self {
            speed: 5.0,
            sensitivity: 0.002,
            fov_y: 60.0,
            camera_type: CameraType::Normal,
        }
    }

    pub fn update(&mut self, camera: &mut Camera) {
        camera.speed = self.speed;
        camera.sensitivity = self.sensitivity;
        camera.fov_y = self.fov_y;
    }

    // setting camera offset is just adding the CameraFollowTarget struct
    // to the ecs system
}

pub struct PlayerCamera;

impl PlayerCamera {
    pub fn new() -> CameraComponent {
        CameraComponent {
            camera_type: CameraType::Player,
            ..CameraComponent::new()
        }
    }

    pub fn handle_keyboard_input(
        camera: &mut Camera,
        pressed_keys: &HashSet<KeyCode>
    ) {
        for key in pressed_keys {
            match key {
                KeyCode::KeyW => camera.move_forwards(),
                KeyCode::KeyA => camera.move_left(),
                KeyCode::KeyD => camera.move_right(),
                KeyCode::KeyS => camera.move_back(),
                KeyCode::ShiftLeft => camera.move_down(),
                KeyCode::Space => camera.move_up(),
                _ => {}
            }
        }
    }

    pub fn handle_mouse_input(camera: &mut Camera, component: &CameraComponent, mouse_delta: Option<(f64, f64)>) {
        if let Some((dx, dy)) = mouse_delta {
            camera.track_mouse_delta(dx * component.sensitivity, dy * component.sensitivity);
        }
    }
}

pub struct DebugCamera;

impl DebugCamera {
    pub fn new() -> CameraComponent {
        CameraComponent {
            camera_type: CameraType::Debug,
            ..CameraComponent::new()
        }
    }

    pub fn handle_keyboard_input(
        camera: &mut Camera,
        pressed_keys: &HashSet<KeyCode>
    ) {
        for key in pressed_keys {
            match key {
                KeyCode::KeyW => camera.move_forwards(),
                KeyCode::KeyA => camera.move_left(),
                KeyCode::KeyD => camera.move_right(),
                KeyCode::KeyS => camera.move_back(),
                KeyCode::ShiftLeft => camera.move_down(),
                KeyCode::Space => camera.move_up(),
                _ => {}
            }
        }
    }

    pub fn handle_mouse_input(camera: &mut Camera, component: &CameraComponent, mouse_delta: Option<(f64, f64)>) {
        if let Some((dx, dy)) = mouse_delta {
            camera.track_mouse_delta(dx * component.sensitivity, dy * component.sensitivity);
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct CameraFollowTarget {
    pub follow_target: String,
    pub offset: DVec3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CameraType {
    Normal,
    Debug,
    Player,
}

impl Default for CameraType {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone)]
pub enum CameraAction {
    SetPlayerTarget { entity: hecs::Entity, offset: DVec3 },
    ClearPlayerTarget,
}

#[cfg(feature = "editor")]
impl Component for Camera {
    fn inspect(&mut self, entity: &mut Entity, cfg: &mut StaticallyKept, ui: &mut Ui, undo_stack: &mut Vec<UndoableAction>, _signal: &mut Signal, _label: &mut String) {
        let _ = _signal;
        ui.group(|ui| {
            CollapsingHeader::new("Camera").default_open(true).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Name: ");
                    let resp = ui.text_edit_singleline(&mut self.label);

                    if resp.changed() {
                        if cfg.old_label_entity.is_none() {
                            cfg.old_label_entity = Some(entity.clone());
                            cfg.label_original = Some(self.label.clone());
                        }
                        cfg.label_last_edit = Some(Instant::now());
                    }

                    if resp.lost_focus() {
                        if let Some(ent) = cfg.old_label_entity.take() {
                            if ent == *entity {
                                if let Some(orig) = cfg.label_original.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Label(ent, orig, EntityType::Entity),
                                    );
                                    log::debug!("Pushed camera label change to undo stack");
                                }
                            } else {
                                cfg.label_original = None;
                            }
                        }
                        cfg.label_last_edit = None;
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Position:");
                    ui.label(format!("{:.2}, {:.2}, {:.2}", self.eye.x, self.eye.y, self.eye.z));
                });

                ui.horizontal(|ui| {
                    ui.label("Target:");
                    ui.label(format!("{:.2}, {:.2}, {:.2}", self.target.x, self.target.y, self.target.z));
                });
            });
        });
    }
}

#[derive(Debug)]
#[cfg(feature = "editor")]
pub enum UndoableCameraAction {
    Speed(hecs::Entity, f64),
    Sensitivity(hecs::Entity, f64),
    FOV(hecs::Entity, f64),
    Type(hecs::Entity, CameraType),
}

#[cfg(feature = "editor")]
impl Component for CameraComponent {
    fn inspect(&mut self, _entity: &mut Entity, _cfg: &mut StaticallyKept, ui: &mut Ui, _undo_stack: &mut Vec<UndoableAction>, _signal: &mut Signal, _label: &mut String) {
        ui.group(|ui| {
            CollapsingHeader::new("Camera Component").default_open(true).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Type:");
                    egui::ComboBox::from_label("")
                        .selected_text(format!("{:?}", self.camera_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.camera_type, CameraType::Normal, "Normal");
                            if !matches!(self.camera_type, CameraType::Player) {
                                ui.selectable_value(&mut self.camera_type, CameraType::Debug, "Debug");
                            } else {
                                ui.add_enabled(false, egui::Button::new("Debug"));
                                ui.label("Debug not available for player cameras");
                            }
                            ui.selectable_value(&mut self.camera_type, CameraType::Player, "Player");
                        });
                });

                ui.horizontal(|ui| {
                    ui.label("Speed:");
                    ui.add(egui::DragValue::new(&mut self.speed).speed(0.1).range(0.1..=20.0));
                });

                ui.horizontal(|ui| {
                    ui.label("Sensitivity:");
                    ui.add(egui::DragValue::new(&mut self.sensitivity).speed(0.0001).range(0.0001..=1.0));
                });

                ui.horizontal(|ui| {
                    ui.label("FOV:");
                    ui.add(egui::Slider::new(&mut self.fov_y, 10.0..=120.0).suffix("°"));
                });

                if matches!(self.camera_type, CameraType::Player) {
                    ui.separator();
                    ui.label("Player Camera Controls:");
                    if ui.button("Set as Active Camera").clicked() {
                        // This would need to be implemented via signal
                        log::info!("Set player camera as active (not implemented)");
                    }
                }
            });
        });
    }
}

#[cfg(feature = "editor")]
impl Component for CameraFollowTarget {
    fn inspect(&mut self, _entity: &mut Entity, _cfg: &mut StaticallyKept, ui: &mut Ui, _undo_stack: &mut Vec<UndoableAction>, signal: &mut Signal, _label: &mut String) {
        ui.group(|ui| {
            CollapsingHeader::new("Camera Follow Target").default_open(true).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Target Entity:");
                    ui.text_edit_singleline(&mut self.follow_target);
                });

                ui.horizontal(|ui| {
                    ui.label("Offset:");
                });

                ui.horizontal(|ui| {
                    ui.label("X:");
                    ui.add(egui::DragValue::new(&mut self.offset.x).speed(0.1));
                });

                ui.horizontal(|ui| {
                    ui.label("Y:");
                    ui.add(egui::DragValue::new(&mut self.offset.y).speed(0.1));
                });

                ui.horizontal(|ui| {
                    ui.label("Z:");
                    ui.add(egui::DragValue::new(&mut self.offset.z).speed(0.1));
                });

                if ui.button("Clear Target").clicked() {
                    *signal = Signal::CameraAction(CameraAction::ClearPlayerTarget);
                }
            });
        });
    }
}