//! This module should describe the different components that are editable in the resource inspector.

use std::time::Instant;
use egui::{CollapsingHeader, Ui};
use hecs::Entity;
use dropbear_engine::entity::{AdoptedEntity, Transform};
use dropbear_engine::lighting::Light;
use crate::editor::{EntityType, Signal, StaticallyKept, UndoableAction};
use crate::scripting::{ScriptAction, TEMPLATE_SCRIPT};
use crate::states::ScriptComponent;

pub trait Component {
    fn inspect(&mut self, entity: &mut hecs::Entity, cfg: &mut StaticallyKept, ui: &mut Ui, undo_stack: &mut Vec<UndoableAction>, signal: &mut Signal, label: &mut String);
}

impl Component for Transform {
    fn inspect(&mut self, entity: &mut Entity, cfg: &mut StaticallyKept, ui: &mut Ui, undo_stack: &mut Vec<UndoableAction>, _signal: &mut Signal, _label: &mut String) {
        ui.group(|ui| {
            CollapsingHeader::new("Transform").default_open(true).show(
                ui,
                |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Position:");
                    });

                    ui.horizontal(|ui| {
                        ui.label("X:");
                        let response = ui.add(
                            egui::DragValue::new(&mut self.position.x)
                                .speed(0.1)
                                .fixed_decimals(3),
                        );

                        if response.drag_started() {
                            cfg.transform_old_entity = Some(entity.clone());
                            cfg.transform_original_transform = Some((*self).clone());
                            cfg.transform_in_progress = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take() {
                                if let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed X transform change to undo stack");
                                }
                            }
                            cfg.transform_in_progress = false;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Y:");
                        let response = ui.add(
                            egui::DragValue::new(&mut self.position.y)
                                .speed(0.1)
                                .fixed_decimals(3),
                        );

                        if response.drag_started() {
                            cfg.transform_old_entity = Some(entity.clone());
                            cfg.transform_original_transform = Some((*self).clone());
                            cfg.transform_in_progress = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take() {
                                if let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed Y transform change to undo stack");
                                }
                            }
                            cfg.transform_in_progress = false;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Z:");
                        let response = ui.add(
                            egui::DragValue::new(&mut self.position.z)
                                .speed(0.1)
                                .fixed_decimals(3),
                        );

                        if response.drag_started() {
                            cfg.transform_old_entity = Some(entity.clone());
                            cfg.transform_original_transform = Some((*self).clone());
                            cfg.transform_in_progress = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take() {
                                if let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed Z transform change to undo stack");
                                }
                            }
                            cfg.transform_in_progress = false;
                        }
                    });
                    if ui.button("Reset Position").clicked() {
                        self.position = glam::DVec3::ZERO;
                    }

                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.label("Rotation:");
                    });

                    let mut rotation_changed = false;
                    let (mut x_rot, mut y_rot, mut z_rot) =
                        self.rotation.to_euler(glam::EulerRot::XYZ);

                    ui.horizontal(|ui| {
                        ui.label("X:");
                        let response = ui.add(
                            egui::Slider::new(
                                &mut x_rot,
                                -std::f64::consts::PI
                                    ..=std::f64::consts::PI,
                            )
                                .step_by(0.01)
                                .custom_formatter(|n, _| {
                                    format!("{:.1}°", n.to_degrees())
                                })
                                .custom_parser(|s| {
                                    s.trim_end_matches('°')
                                        .parse::<f64>()
                                        .ok()
                                        .map(|v| v.to_radians())
                                }),
                        );

                        if response.drag_started() {
                            cfg.transform_old_entity = Some(entity.clone());
                            cfg.transform_original_transform = Some((*self).clone());
                            cfg.transform_in_progress = true;
                        }

                        if response.changed() {
                            rotation_changed = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take() {
                                if let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed X rotation change to undo stack");
                                }
                            }
                            cfg.transform_in_progress = false;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Y:");
                        let response = ui.add(
                            egui::Slider::new(
                                &mut y_rot,
                                -std::f64::consts::PI
                                    ..=std::f64::consts::PI,
                            )
                                .step_by(0.01)
                                .custom_formatter(|n, _| {
                                    format!("{:.1}°", n.to_degrees())
                                })
                                .custom_parser(|s| {
                                    s.trim_end_matches('°')
                                        .parse::<f64>()
                                        .ok()
                                        .map(|v| v.to_radians())
                                }),
                        );

                        if response.drag_started() {
                            cfg.transform_old_entity = Some(entity.clone());
                            cfg.transform_original_transform = Some((*self).clone());
                            cfg.transform_in_progress = true;
                        }

                        if response.changed() {
                            rotation_changed = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take() {
                                if let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed Y rotation change to undo stack");
                                }
                            }
                            cfg.transform_in_progress = false;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Z:");
                        let response = ui.add(
                            egui::Slider::new(
                                &mut z_rot,
                                -std::f64::consts::PI
                                    ..=std::f64::consts::PI,
                            )
                                .step_by(0.01)
                                .custom_formatter(|n, _| {
                                    format!("{:.1}°", n.to_degrees())
                                })
                                .custom_parser(|s| {
                                    s.trim_end_matches('°')
                                        .parse::<f64>()
                                        .ok()
                                        .map(|v| v.to_radians())
                                }),
                        );

                        if response.drag_started() {
                            cfg.transform_old_entity = Some(entity.clone());
                            cfg.transform_original_transform = Some((*self).clone());
                            cfg.transform_in_progress = true;
                        }

                        if response.changed() {
                            rotation_changed = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take() {
                                if let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed Z rotation change to undo stack");
                                }
                            }
                            cfg.transform_in_progress = false;
                        }
                    });

                    if rotation_changed {
                        self.rotation = glam::DQuat::from_euler(
                            glam::EulerRot::XYZ,
                            x_rot,
                            y_rot,
                            z_rot,
                        );
                    }
                    if ui.button("Reset Rotation").clicked() {
                        self.rotation = glam::DQuat::IDENTITY;
                    }
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.label("Scale:");
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                let lock_icon = if cfg.scale_locked {
                                    "🔒"
                                } else {
                                    "🔓"
                                };
                                if ui
                                    .button(lock_icon)
                                    .on_hover_text("Lock uniform scaling")
                                    .clicked()
                                {
                                    cfg.scale_locked = !cfg.scale_locked;
                                }
                            },
                        );
                    });

                    let mut scale_changed = false;
                    let mut new_scale = self.scale;

                    ui.horizontal(|ui| {
                        ui.label("X:");
                        let response = ui.add(
                            egui::DragValue::new(&mut new_scale.x)
                                .speed(0.01)
                                .fixed_decimals(3),
                        );

                        if response.drag_started() {
                            cfg.transform_old_entity = Some(entity.clone());
                            cfg.transform_original_transform = Some((*self).clone());
                            cfg.transform_in_progress = true;
                        }

                        if response.changed() {
                            scale_changed = true;
                            if cfg.scale_locked {
                                let scale_factor = new_scale.x / self.scale.x;
                                new_scale.y = self.scale.y * scale_factor;
                                new_scale.z = self.scale.z * scale_factor;
                            }
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take() {
                                if let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed X scale change to undo stack");
                                }
                            }
                            cfg.transform_in_progress = false;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Y:");
                        let y_slider = egui::DragValue::new(&mut new_scale.y)
                            .speed(0.01)
                            .fixed_decimals(3);

                        let response = ui.add_enabled(!cfg.scale_locked, y_slider);

                        if response.drag_started() && !cfg.scale_locked {
                            cfg.transform_old_entity = Some(entity.clone());
                            cfg.transform_original_transform = Some((*self).clone());
                            cfg.transform_in_progress = true;
                        }

                        if response.changed() {
                            scale_changed = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take() {
                                if let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed Y scale change to undo stack");
                                }
                            }
                            cfg.transform_in_progress = false;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Z:");
                        let z_slider = egui::DragValue::new(&mut new_scale.z)
                            .speed(0.01)
                            .fixed_decimals(3);

                        let response = ui.add_enabled(!cfg.scale_locked, z_slider);

                        if response.drag_started() && !cfg.scale_locked {
                            cfg.transform_old_entity = Some(entity.clone());
                            cfg.transform_original_transform = Some((*self).clone());
                            cfg.transform_in_progress = true;
                        }

                        if response.changed() {
                            scale_changed = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take() {
                                if let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed Z scale change to undo stack");
                                }
                            }
                            cfg.transform_in_progress = false;
                        }
                    });

                    if scale_changed {
                        self.scale = new_scale;
                    }
                    if ui.button("Reset Scale").clicked() {
                        self.scale = glam::DVec3::ONE;
                    }
                    ui.add_space(5.0);

                    // maybe use? probs not :/
                    // if pos_changed || rotation_changed || scale_changed {
                    //     ui.colored_label(egui::Color32::YELLOW, "Transform modified");
                    // }
                },
            );
        });
    }
}

impl Component for ScriptComponent {
    fn inspect(&mut self, _entity: &mut Entity, _cfg: &mut StaticallyKept, ui: &mut Ui, _undo_stack: &mut Vec<UndoableAction>, signal: &mut Signal, label: &mut String) {
        let script_loc = self.path.to_str().unwrap_or("").to_string();

        ui.group(|ui| {
            CollapsingHeader::new("Scripting")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Browse").clicked() {
                            if let Some(script_file) = rfd::FileDialog::new()
                                .add_filter("Rhai Script", &["rhai"])
                                .pick_file()
                            {
                                let script_name = script_file
                                    .file_stem()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();
                                *signal = Signal::ScriptAction(ScriptAction::AttachScript {
                                    script_path: script_file,
                                    script_name,
                                });
                            }
                        }

                        if ui.button("New").clicked() {
                            if let Some(script_path) = rfd::FileDialog::new()
                                .add_filter("Rhai Script", &["rhai"])
                                .set_file_name(format!("{}_script.rhai", label))
                                .save_file()
                            {
                                match std::fs::write(&script_path, TEMPLATE_SCRIPT) {
                                    Ok(_) => {
                                        let script_name = script_path
                                            .file_stem()
                                            .unwrap_or_default()
                                            .to_string_lossy()
                                            .to_string();
                                        *signal = Signal::ScriptAction(ScriptAction::CreateAndAttachScript {
                                            script_path,
                                            script_name,
                                        });
                                    },
                                    Err(e) => {
                                        crate::warn!("Failed to create script file: {}", e);
                                    },
                                }
                            }
                        }
                    });

                    ui.separator();

                    ui.horizontal_wrapped(|ui| {
                        ui.label("Script Location:");
                        ui.label(script_loc);
                    });

                    if ui.button("Remove").clicked() {
                        *signal = Signal::ScriptAction(ScriptAction::RemoveScript);
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("Edit Script").clicked() {
                            *signal = Signal::ScriptAction(ScriptAction::EditScript);
                        }
                    });
                });
        });
    }
}

impl Component for AdoptedEntity {
    fn inspect(&mut self, entity: &mut Entity, cfg: &mut StaticallyKept, ui: &mut Ui, undo_stack: &mut Vec<UndoableAction>, _signal: &mut Signal, _label: &mut String) {
        // label
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Name: ");

                let resp = ui.text_edit_singleline(self.label_mut());

                if resp.changed() {
                    if cfg.old_label_entity.is_none() {
                        cfg.old_label_entity = Some(entity.clone());
                        cfg.label_original = Some(self.label().clone());
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
                                log::debug!("Pushed label change to undo stack (immediate)");
                            }
                        } else {
                            cfg.label_original = None;
                        }
                    }
                    cfg.label_last_edit = None;
                }
            })
        });
    }
}

impl Component for Light {
    fn inspect(&mut self, entity: &mut Entity, cfg: &mut StaticallyKept, ui: &mut Ui, undo_stack: &mut Vec<UndoableAction>, _signal: &mut Signal, _label: &mut String) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Name: ");

                let resp = ui.text_edit_singleline(&mut self.label);

                if resp.changed() {
                    if cfg.old_label_entity.is_none() {
                        cfg.old_label_entity = Some(entity.clone());
                        cfg.label_original = Some(self.label.clone().to_string());
                    }
                    cfg.label_last_edit = Some(Instant::now());
                }

                if resp.lost_focus() {
                    if let Some(ent) = cfg.old_label_entity.take() {
                        if ent == *entity {
                            if let Some(orig) = cfg.label_original.take() {
                                UndoableAction::push_to_undo(
                                    undo_stack,
                                    UndoableAction::Label(ent, orig, EntityType::Light),
                                );
                                log::debug!("Pushed label change to undo stack (immediate)");
                            }
                        } else {
                            cfg.label_original = None;
                        }
                    }
                    cfg.label_last_edit = None;
                }
            })
        });
    }
}