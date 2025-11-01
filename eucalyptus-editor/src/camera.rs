use crate::editor::component::InspectableComponent;
use crate::editor::{EntityType, Signal, StaticallyKept, UndoableAction};
use dropbear_engine::camera::Camera;
use egui::{CollapsingHeader, Ui};
use eucalyptus_core::camera::{CameraComponent, CameraType};
use hecs::Entity;
use std::time::Instant;

impl InspectableComponent for Camera {
    fn inspect(
        &mut self,
        entity: &mut Entity,
        cfg: &mut StaticallyKept,
        ui: &mut Ui,
        undo_stack: &mut Vec<UndoableAction>,
        _signal: &mut Signal,
        _label: &mut String,
    ) {
        ui.vertical(|ui| {
            CollapsingHeader::new("Camera")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name: ");
                        let resp = ui.text_edit_singleline(&mut self.label);

                        if resp.changed() {
                            if cfg.old_label_entity.is_none() {
                                cfg.old_label_entity = Some(*entity);
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
                        ui.label(format!(
                            "{:.2}, {:.2}, {:.2}",
                            self.eye.x, self.eye.y, self.eye.z
                        ));
                        if ui.button("Reset").clicked() {
                            self.eye = glam::DVec3::ZERO;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Target:");
                        ui.label(format!(
                            "{:.2}, {:.2}, {:.2}",
                            self.target.x, self.target.y, self.target.z
                        ));
                        if ui.button("Reset").clicked() {
                            self.target = glam::DVec3::ZERO;
                        }
                    });
                });
        });
    }
}

#[derive(Debug)]
#[allow(dead_code)]
// todo: provide a purpose for this
pub enum UndoableCameraAction {
    Speed(Entity, f64),
    Sensitivity(Entity, f64),
    Fov(Entity, f64),
    Type(Entity, CameraType),
}

impl InspectableComponent for CameraComponent {
    fn inspect(
        &mut self,
        _entity: &mut Entity,
        _cfg: &mut StaticallyKept,
        ui: &mut Ui,
        _undo_stack: &mut Vec<UndoableAction>,
        _signal: &mut Signal,
        _label: &mut String,
    ) {
        ui.vertical(|ui| {
            CollapsingHeader::new("Camera Component")
                .default_open(true)
                .show(ui, |ui| {
                    if !matches!(self.camera_type, CameraType::Player) {
                        egui::ComboBox::from_id_salt(
                            "i aint r kelly the way i take the piss ; \
                        but im mj, my shots don't eva miss",
                        )
                        .selected_text(format!("{:?}", self.camera_type))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.camera_type,
                                CameraType::Normal,
                                "Normal",
                            );
                            ui.selectable_value(&mut self.camera_type, CameraType::Debug, "Debug");
                        });
                    }

                    ui.horizontal(|ui| {
                        ui.label("Speed:");
                        ui.add(
                            egui::DragValue::new(&mut self.settings.speed)
                                .speed(0.1)
                                .range(0.1..=20.0),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Sensitivity:");
                        ui.add(
                            egui::DragValue::new(&mut self.settings.sensitivity)
                                .speed(0.0001)
                                .range(0.0001..=1.0),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("FOV:");
                        ui.add(
                            egui::Slider::new(&mut self.settings.fov_y, 10.0..=120.0).suffix("°"),
                        );
                    });
                });
        });
    }
}
