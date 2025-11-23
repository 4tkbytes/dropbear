use crate::editor::component::InspectableComponent;
use crate::editor::{Signal, StaticallyKept, UndoableAction};
use dropbear_engine::camera::Camera;
use egui::{CollapsingHeader, Ui};
use eucalyptus_core::camera::{CameraComponent, CameraType};
use hecs::Entity;

impl InspectableComponent for Camera {
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
            CollapsingHeader::new("Camera")
                .default_open(true)
                .show(ui, |ui| {
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
