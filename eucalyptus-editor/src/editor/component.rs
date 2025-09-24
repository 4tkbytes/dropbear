//! This module should describe the different components that are editable in the resource inspector.

use std::sync::Arc;
use crate::editor::{EntityType, Signal, StaticallyKept, UndoableAction};
use dropbear_engine::attenuation::ATTENUATION_PRESETS;
use dropbear_engine::entity::{AdoptedEntity, Transform};
use dropbear_engine::lighting::{Light, LightComponent, LightType};
use egui::{Button, CollapsingHeader, ComboBox, DragValue, Grid, RichText, Slider, TextEdit, Ui, Widget};
use eucalyptus_core::scripting::{ScriptAction, TEMPLATE_SCRIPT};
use eucalyptus_core::states::{ModelProperties, ScriptComponent, Value};
use eucalyptus_core::warn;
use glam::Vec3;
use hecs::Entity;
use std::time::Instant;
use egui_extras::{Column, TableBuilder};

/// A trait that can added to any component that allows you to inspect the value in the editor.
pub trait InspectableComponent {
    fn inspect(
        &mut self,
        entity: &mut Entity,
        cfg: &mut StaticallyKept,
        ui: &mut Ui,
        undo_stack: &mut Vec<UndoableAction>,
        signal: &mut Signal,
        label: &mut String,
    );
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum ValueType {
    String,
    Float,
    Int,
    Bool,
    Vec3,
}

impl From<Value> for ValueType {
    fn from(value: Value) -> Self {
        match value {
            Value::String(_) => {
                ValueType::String
            }
            Value::Int(_) => {
                ValueType::Int
            }
            Value::Float(_) => {
                ValueType::Float
            }
            Value::Bool(_) => {
                ValueType::Bool
            }
            Value::Vec3(_) => {
                ValueType::Vec3
            }
        }
    }
}

impl InspectableComponent for ModelProperties {
    fn inspect(&mut self, _entity: &mut Entity, _cfg: &mut StaticallyKept, ui: &mut Ui, _undo_stack: &mut Vec<UndoableAction>, _signal: &mut Signal, _label: &mut String) {
        CollapsingHeader::new("Custom Properties")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("properties")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Key").heading());
                        ui.label(RichText::new("Type").heading());
                        ui.label(RichText::new("Value").heading());
                        ui.label(RichText::new("Action").heading());
                        ui.end_row();

                        let mut local_del = (false, String::new());
                        let mut local_key_change = (false, String::new(), String::new(), 0); // flag, old key, new key, index
                        for (i, (key, val)) in self.custom_properties.iter_mut().enumerate() {
                            // key
                            let mut before_key = key.clone();
                            ui.add_sized([100.0, 20.0], TextEdit::singleline(&mut before_key));

                            if before_key != *key {
                                local_key_change = (true, key.clone(), before_key.clone(), i);
                            }

                            let mut before = ValueType::from(val.clone());

                            // type
                            ComboBox::from_id_salt(format!("{:?}", val))
                                .selected_text(format!("{:?}", before))
                                .show_ui(ui, |ui| {
                                ui.selectable_value(&mut before, ValueType::String, "String");
                                ui.selectable_value(&mut before, ValueType::Float, "Float");
                                ui.selectable_value(&mut before, ValueType::Int, "Int");
                                ui.selectable_value(&mut before, ValueType::Bool, "Bool");
                                ui.selectable_value(&mut before, ValueType::Vec3, "Vec3");
                            });

                            if ValueType::from(val.clone()) != before {
                                match before {
                                    ValueType::String => {
                                        *val = Value::String(String::new());
                                    }
                                    ValueType::Float => {
                                        *val = Value::Float(0.0);
                                    }
                                    ValueType::Int => {
                                        *val = Value::Int(0);
                                    }
                                    ValueType::Bool => {
                                        *val = Value::Bool(false);
                                    }
                                    ValueType::Vec3 => {
                                        *val = Value::Vec3([0.0, 0.0, 0.0])
                                    }
                                }
                            }

                            // value
                            let speed = {
                                let shift = ui.input(|i| i.modifiers.shift);
                                let ctrl_or_cmd = ui.input(|i| {
                                    #[cfg(target_os = "macos")]
                                    {
                                        i.modifiers.mac_cmd
                                    }
                                    #[cfg(not(target_os = "macos"))]
                                    {
                                        i.modifiers.ctrl
                                    }
                                });

                                if shift {
                                    0.01
                                } else if ctrl_or_cmd {
                                    1.0
                                } else {
                                    0.1
                                }
                            };
                            match val {
                                Value::String(string) => {
                                    ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(string));
                                }
                                Value::Int(int) => {
                                    ui.add(DragValue::new(int).speed(1));
                                }
                                Value::Float(float) => {
                                    let drag = DragValue::new(float).speed(speed);
                                    ui.add(drag);
                                }
                                Value::Bool(bool) => {
                                    if ui.button(if *bool {"✅"} else {"❌"}).clicked() {
                                        *bool = !*bool;
                                    };
                                }
                                Value::Vec3(vec) => {
                                    ui.horizontal(|ui| {
                                        ui.add(DragValue::new(&mut vec[0]).speed(speed));
                                        ui.add(DragValue::new(&mut vec[1]).speed(speed));
                                        ui.add(DragValue::new(&mut vec[2]).speed(speed));
                                    });
                                }
                            }

                            // action
                            if ui.button("🗑️").clicked() {
                                log::debug!("Trashing {}", key);
                                local_del = (true, key.clone());
                            };
                            ui.end_row();
                        }

                        if local_del.0 {
                            self.custom_properties.remove(&local_del.1);
                        }

                        if local_key_change.0 {
                            let value = self.custom_properties.remove(&local_key_change.1);
                            if let Some(val) = value {
                                self.custom_properties.insert(local_key_change.2, val);
                            } else {
                                warn!("Cannot change name of key: unable to remove from hashmap")
                            }
                        }

                        if ui.button("Add").clicked() {
                            log::debug!("Inserting new default value");
                            self.custom_properties.insert(String::new(), Value::default());
                        };
                })
            });
    }
}

impl InspectableComponent for Transform {
    fn inspect(
        &mut self,
        entity: &mut Entity,
        cfg: &mut StaticallyKept,
        ui: &mut Ui,
        undo_stack: &mut Vec<UndoableAction>,
        _signal: &mut Signal,
        _label: &mut String,
    ) {
        ui.group(|ui| {
            CollapsingHeader::new("Transform")
                .default_open(true)
                .show(ui, |ui| {
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
                            cfg.transform_old_entity = Some(*entity);
                            cfg.transform_original_transform = Some(*self);
                            cfg.transform_in_progress = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take()
                                && let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed X transform change to undo stack");
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
                            cfg.transform_old_entity = Some(*entity);
                            cfg.transform_original_transform = Some(*self);
                            cfg.transform_in_progress = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take()
                                && let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed Y transform change to undo stack");
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
                            cfg.transform_old_entity = Some(*entity);
                            cfg.transform_original_transform = Some(*self);
                            cfg.transform_in_progress = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take()
                                && let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed Z transform change to undo stack");
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
                                -std::f64::consts::PI..=std::f64::consts::PI,
                            )
                            .step_by(0.01)
                            .custom_formatter(|n, _| format!("{:.1}°", n.to_degrees()))
                            .custom_parser(|s| {
                                s.trim_end_matches('°')
                                    .parse::<f64>()
                                    .ok()
                                    .map(|v| v.to_radians())
                            }),
                        );

                        if response.drag_started() {
                            cfg.transform_old_entity = Some(*entity);
                            cfg.transform_original_transform = Some(*self);
                            cfg.transform_in_progress = true;
                        }

                        if response.changed() {
                            rotation_changed = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take()
                                && let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed X rotation change to undo stack");
                                }
                            cfg.transform_in_progress = false;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Y:");
                        let response = ui.add(
                            egui::Slider::new(
                                &mut y_rot,
                                -std::f64::consts::PI..=std::f64::consts::PI,
                            )
                            .step_by(0.01)
                            .custom_formatter(|n, _| format!("{:.1}°", n.to_degrees()))
                            .custom_parser(|s| {
                                s.trim_end_matches('°')
                                    .parse::<f64>()
                                    .ok()
                                    .map(|v| v.to_radians())
                            }),
                        );

                        if response.drag_started() {
                            cfg.transform_old_entity = Some(*entity);
                            cfg.transform_original_transform = Some(*self);
                            cfg.transform_in_progress = true;
                        }

                        if response.changed() {
                            rotation_changed = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take()
                                && let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed Y rotation change to undo stack");
                                }
                            cfg.transform_in_progress = false;
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Z:");
                        let response = ui.add(
                            egui::Slider::new(
                                &mut z_rot,
                                -std::f64::consts::PI..=std::f64::consts::PI,
                            )
                            .step_by(0.01)
                            .custom_formatter(|n, _| format!("{:.1}°", n.to_degrees()))
                            .custom_parser(|s| {
                                s.trim_end_matches('°')
                                    .parse::<f64>()
                                    .ok()
                                    .map(|v| v.to_radians())
                            }),
                        );

                        if response.drag_started() {
                            cfg.transform_old_entity = Some(*entity);
                            cfg.transform_original_transform = Some(*self);
                            cfg.transform_in_progress = true;
                        }

                        if response.changed() {
                            rotation_changed = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take()
                                && let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed Z rotation change to undo stack");
                                }
                            cfg.transform_in_progress = false;
                        }
                    });

                    if rotation_changed {
                        self.rotation =
                            glam::DQuat::from_euler(glam::EulerRot::XYZ, x_rot, y_rot, z_rot);
                    }
                    if ui.button("Reset Rotation").clicked() {
                        self.rotation = glam::DQuat::IDENTITY;
                    }
                    ui.add_space(5.0);

                    ui.horizontal(|ui| {
                        ui.label("Scale:");
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let lock_icon = if cfg.scale_locked { "🔒" } else { "🔓" };
                            if ui
                                .button(lock_icon)
                                .on_hover_text("Lock uniform scaling")
                                .clicked()
                            {
                                cfg.scale_locked = !cfg.scale_locked;
                            }
                        });
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
                            cfg.transform_old_entity = Some(*entity);
                            cfg.transform_original_transform = Some(*self);
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
                            if let Some(ent) = cfg.transform_old_entity.take()
                                && let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed X scale change to undo stack");
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
                            cfg.transform_old_entity = Some(*entity);
                            cfg.transform_original_transform = Some(*self);
                            cfg.transform_in_progress = true;
                        }

                        if response.changed() {
                            scale_changed = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take()
                                && let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed Y scale change to undo stack");
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
                            cfg.transform_old_entity = Some(*entity);
                            cfg.transform_original_transform = Some(*self);
                            cfg.transform_in_progress = true;
                        }

                        if response.changed() {
                            scale_changed = true;
                        }

                        if response.drag_stopped() && cfg.transform_in_progress {
                            if let Some(ent) = cfg.transform_old_entity.take()
                                && let Some(orig) = cfg.transform_original_transform.take() {
                                    UndoableAction::push_to_undo(
                                        undo_stack,
                                        UndoableAction::Transform(ent, orig),
                                    );
                                    log::debug!("Pushed Z scale change to undo stack");
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
                });
        });
    }
}

impl InspectableComponent for ScriptComponent {
    fn inspect(
        &mut self,
        _entity: &mut Entity,
        _cfg: &mut StaticallyKept,
        ui: &mut Ui,
        _undo_stack: &mut Vec<UndoableAction>,
        signal: &mut Signal,
        label: &mut String,
    ) {
        let script_loc = self.path.to_str().unwrap_or("").to_string();

        ui.group(|ui| {
            CollapsingHeader::new("Scripting")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Browse").clicked()
                            && let Some(script_file) = rfd::FileDialog::new()
                                .add_filter("Typescript", &["ts"])
                                .pick_file()
                            {
                                let script_name = script_file
                                    .file_stem()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .to_string();
                                let lib_path = &script_file.clone().parent().unwrap().join("dropbear.ts");
                                if std::fs::read(lib_path).is_err() {
                                    log::warn!("dropbear.ts library does not exist in project source directory, copying...");
                                    if let Err(e) = std::fs::write(lib_path, include_str!("../../../resources/dropbear.ts")) {
                                        log::error!("Non-fatal error: Creating library file failed: {}", e);
                                    } else {
                                        log::info!("Wrote dropbear.ts library file!");
                                    }
                                };
                                *signal = Signal::ScriptAction(ScriptAction::AttachScript {
                                    script_path: script_file,
                                    script_name,
                                });
                            }

                        if ui.button("New").clicked()
                            && let Some(script_path) = rfd::FileDialog::new()
                                .add_filter("TypeScript", &["ts"])
                                .set_file_name(format!("{}_script.ts", label))
                                .save_file()
                            {
                                // check if dropbear module exists
                                // todo: change this to an %APPDATA% file instead of to memory. 
                                let lib_path = &script_path.clone().parent().unwrap().join("dropbear.ts");
                                if std::fs::read(lib_path).is_err() {
                                    log::warn!("dropbear.ts library does not exist in project source directory, copying...");
                                    if let Err(e) = std::fs::write(lib_path, include_str!("../../../resources/dropbear.ts")) {
                                        log::error!("Non-fatal error: Creating library file failed: {}", e);
                                    } else {
                                        log::info!("Wrote dropbear.ts library file!");
                                    }
                                };
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
                                        warn!("Failed to create script file: {}", e);
                                    },
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

impl InspectableComponent for AdoptedEntity {
    fn inspect(
        &mut self,
        entity: &mut Entity,
        cfg: &mut StaticallyKept,
        ui: &mut Ui,
        undo_stack: &mut Vec<UndoableAction>,
        _signal: &mut Signal,
        _label: &mut String,
    ) {
        // label
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Name: ");

                let resp = ui.text_edit_singleline(&mut Arc::make_mut(&mut self.model).label);

                if resp.changed() {
                    if cfg.old_label_entity.is_none() {
                        cfg.old_label_entity = Some(*entity);
                        cfg.label_original = Some(self.model.label.clone());
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

impl InspectableComponent for Light {
    fn inspect(
        &mut self,
        entity: &mut Entity,
        cfg: &mut StaticallyKept,
        ui: &mut Ui,
        undo_stack: &mut Vec<UndoableAction>,
        _signal: &mut Signal,
        _label: &mut String,
    ) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.label("Name: ");

                let resp = ui.text_edit_singleline(&mut self.label);

                if resp.changed() {
                    if cfg.old_label_entity.is_none() {
                        cfg.old_label_entity = Some(*entity);
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

impl InspectableComponent for LightComponent {
    fn inspect(
        &mut self,
        _entity: &mut Entity,
        _cfg: &mut StaticallyKept,
        ui: &mut Ui,
        _undo_stack: &mut Vec<UndoableAction>,
        _signal: &mut Signal,
        _label: &mut String,
    ) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ComboBox::new("light_type", "Light Type")
                    // .width(ui.available_width())
                    .selected_text(self.light_type.to_string())
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut self.light_type,
                            LightType::Directional,
                            "Directional",
                        );
                        ui.selectable_value(&mut self.light_type, LightType::Point, "Point");
                        ui.selectable_value(&mut self.light_type, LightType::Spot, "Spot");
                    });
            });

            // let is_dir = matches!(self.light_type, LightType::Directional);
            let is_point = matches!(self.light_type, LightType::Point);
            let is_spot = matches!(self.light_type, LightType::Spot);

            // colour
            ui.separator();
            let mut colour = self.colour.clone().as_vec3().to_array();
            ui.horizontal(|ui| {
                ui.label("Colour");
                egui::color_picker::color_edit_button_rgb(ui, &mut colour)
            });
            self.colour = Vec3::from_array(colour).as_dvec3();

            // intensity
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Intensity");
                ui.add(egui::Slider::new(&mut self.intensity, 0.0..=1.0));
            });

            // enabled and visible
            ui.separator();
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.enabled, "Enabled");
                ui.checkbox(&mut self.visible, "Visible");
            });

            if is_spot || is_point {
                // attenuation
                ui.separator();
                ui.horizontal(|ui| {
                    ComboBox::new("Range", "Range")
                        // .width(ui.available_width())
                        .selected_text(format!("Range {}", self.attenuation.range))
                        .show_ui(ui, |ui| {
                            for (preset, label) in ATTENUATION_PRESETS {
                                ui.selectable_value(&mut self.attenuation, *preset, *label);
                            }
                        });
                });
            }

            if is_spot {
                // cutoff angles
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Slider::new(&mut self.cutoff_angle, 1.0..=89.0)
                            .text("Inner")
                            .suffix("°")
                            .step_by(0.1),
                    );
                });

                ui.horizontal(|ui| {
                    ui.add(
                        egui::Slider::new(&mut self.outer_cutoff_angle, 1.0..=90.0)
                            .text("Outer")
                            .suffix("°")
                            .step_by(0.1),
                    );
                });

                if self.outer_cutoff_angle <= self.cutoff_angle {
                    self.outer_cutoff_angle = self.cutoff_angle + 1.0;
                }

                let cone_softness = self.outer_cutoff_angle - self.cutoff_angle;
                ui.label(format!("Soft edge: {:.1}°", cone_softness));
            }
        });
    }
}
