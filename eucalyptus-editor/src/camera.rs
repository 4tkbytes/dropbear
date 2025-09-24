use crate::editor::component::InspectableComponent;
use crate::editor::{EntityType, Signal, StaticallyKept, UndoableAction};
use dropbear_engine::camera::Camera;
use egui::{CollapsingHeader, Ui};
use eucalyptus_core::camera::{CameraAction, CameraComponent, CameraType};
use hecs::Entity;
use std::time::Instant;
use eucalyptus_core::success;

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
                    });

                    ui.horizontal(|ui| {
                        ui.label("Target:");
                        ui.label(format!(
                            "{:.2}, {:.2}, {:.2}",
                            self.target.x, self.target.y, self.target.z
                        ));
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
                    // removing this because it is set based on the entity hierarchy
                    // ui.horizontal(|ui| {
                    //     ui.label("Type:");
                    //     egui::ComboBox::from_label("")
                    //         .selected_text(format!("{:?}", self.camera_type))
                    //         .show_ui(ui, |ui| {
                    //             ui.selectable_value(
                    //                 &mut self.camera_type,
                    //                 CameraType::Normal,
                    //                 "Normal",
                    //             );
                    //             if !matches!(self.camera_type, CameraType::Player) {
                    //                 ui.selectable_value(
                    //                     &mut self.camera_type,
                    //                     CameraType::Debug,
                    //                     "Debug",
                    //                 );
                    //             } else {
                    //                 ui.add_enabled(false, egui::Button::new("Debug"));
                    //                 ui.label("Debug not available for player cameras");
                    //             }
                    //             ui.selectable_value(
                    //                 &mut self.camera_type,
                    //                 CameraType::Player,
                    //                 "Player",
                    //             );
                    //         });
                    // });

                    if !matches!(self.camera_type, CameraType::Player) {
                        egui::ComboBox::from_id_salt("i aint r kelly the way i take the piss ; \
                        but im mj coz my shots don't miss")
                                .selected_text(format!("{:?}", self.camera_type))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.camera_type,
                                        CameraType::Normal,
                                        "Normal",
                                    );
                                    ui.selectable_value(
                                        &mut self.camera_type,
                                        CameraType::Debug,
                                        "Debug",
                                    );
                                });
                    }

                    ui.horizontal(|ui| {
                        ui.label("Speed:");
                        ui.add(
                            egui::DragValue::new(&mut self.speed)
                                .speed(0.1)
                                .range(0.1..=20.0),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("Sensitivity:");
                        ui.add(
                            egui::DragValue::new(&mut self.sensitivity)
                                .speed(0.0001)
                                .range(0.0001..=1.0),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label("FOV:");
                        ui.add(egui::Slider::new(&mut self.fov_y, 10.0..=120.0).suffix("°"));
                    });

                    // if matches!(self.camera_type, CameraType::Player) {
                    //     ui.separator();
                    //     ui.label("Player Camera Controls:");
                    //     if ui.button("Set as Active Camera").clicked() {
                    //         *signal = Signal::CameraAction(CameraAction::SetInitialCamera);
                    //         log::info!("Set player camera as active (not implemented)");
                    //     }
                    // }
                });
        });
    }
}

// impl InspectableComponent for CameraFollowTarget {
//     fn inspect(
//         &mut self,
//         entity: &mut Entity,
//         _cfg: &mut StaticallyKept,
//         ui: &mut Ui,
//         _undo_stack: &mut Vec<UndoableAction>,
//         signal: &mut Signal,
//         _label: &mut String,
//     ) {
//         ui.vertical(|ui| {
//             CollapsingHeader::new("Camera Follow Target")
//                 .default_open(true)
//                 .show(ui, |ui| {
//                     ui.horizontal(|ui| {
//                         ui.label("Offset:");
//                     });
// 
//                     ui.horizontal(|ui| {
//                         ui.label("X:");
//                         ui.add(egui::DragValue::new(&mut self.offset.x).speed(0.1));
//                     });
// 
//                     ui.horizontal(|ui| {
//                         ui.label("Y:");
//                         ui.add(egui::DragValue::new(&mut self.offset.y).speed(0.1));
//                     });
// 
//                     ui.horizontal(|ui| {
//                         ui.label("Z:");
//                         ui.add(egui::DragValue::new(&mut self.offset.z).speed(0.1));
//                     });
//                     ui.horizontal(|ui| {
//                         if ui.button("Set current position as offset").clicked() {
//                             success!("Set current position as offset");
//                             *signal = Signal::CameraAction(CameraAction::SetCurrentPositionAsOffset(*entity))
//                         }
//                         if ui.button("Clear Target").clicked() {
//                             success!("Removed camera offset");
//                             *signal = Signal::CameraAction(CameraAction::ClearPlayerTarget);
//                         }
//                     });
//                 });
//         });
//     }
// }
