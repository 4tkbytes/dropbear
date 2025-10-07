//! This module should describe the different components that are editable in the resource inspector.

use std::sync::Arc;
use crate::editor::{EntityType, Signal, StaticallyKept, UndoableAction};
use dropbear_engine::attenuation::ATTENUATION_PRESETS;
use dropbear_engine::entity::{AdoptedEntity, Transform};
use dropbear_engine::lighting::{Light, LightComponent, LightType};
use egui::{CollapsingHeader, ComboBox, DragValue, Grid, RichText, TextEdit, Ui};
use eucalyptus_core::states::{ModelProperties, ScriptComponent, Value};
use eucalyptus_core::warn;
use glam::Vec3;
use hecs::Entity;
use std::time::Instant;

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

impl From<&mut Value> for ValueType {
    fn from(value: &mut Value) -> Self {
        match value {
            Value::String(_) => ValueType::String,
            Value::Int(_) => ValueType::Int,
            Value::Float(_) => ValueType::Float,
            Value::Bool(_) => ValueType::Bool,
            Value::Vec3(_) => ValueType::Vec3,
        }
    }
}


impl InspectableComponent for ModelProperties {
    fn inspect(
        &mut self,
        _entity: &mut Entity,
        _cfg: &mut StaticallyKept,
        ui: &mut Ui,
        _undo_stack: &mut Vec<UndoableAction>,
        _signal: &mut Signal,
        _label: &mut String,
    ) {
        CollapsingHeader::new("Custom Properties")
            .default_open(true)
            .show(ui, |ui| {
                Grid::new("properties")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label(RichText::new("Key"));
                        ui.label(RichText::new("Type"));
                        ui.label(RichText::new("Value"));
                        ui.label(RichText::new("Action"));
                        ui.end_row();

                        let mut to_delete: Option<String> = None;
                        let mut to_rename: Option<(String, String)> = None;

                        let keys: Vec<String> = self.custom_properties.keys().cloned().collect();
                        for (i, key) in keys.into_iter().enumerate() {
                            let val = self.custom_properties.get_mut(&key).unwrap();

                            let mut edited_key = key.clone();
                            ui.add_sized([100.0, 20.0], TextEdit::singleline(&mut edited_key));

                            if edited_key != key {
                                to_rename = Some((key.clone(), edited_key));
                            }

                            let current_type = ValueType::from(&mut *val);
                            let mut selected_type = current_type;

                            ComboBox::from_id_salt(format!("type_{}_{}", i, key))
                                .selected_text(format!("{:?}", selected_type))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut selected_type, ValueType::String, "String");
                                    ui.selectable_value(&mut selected_type, ValueType::Float, "Float");
                                    ui.selectable_value(&mut selected_type, ValueType::Int, "Int");
                                    ui.selectable_value(&mut selected_type, ValueType::Bool, "Bool");
                                    ui.selectable_value(&mut selected_type, ValueType::Vec3, "Vec3");
                                });

                            if selected_type != current_type {
                                *val = match selected_type {
                                    ValueType::String => Value::String(String::new()),
                                    ValueType::Float => Value::Float(0.0),
                                    ValueType::Int => Value::Int(0),
                                    ValueType::Bool => Value::Bool(false),
                                    ValueType::Vec3 => Value::Vec3([0.0, 0.0, 0.0]),
                                };
                            }

                            let speed = {
                                let input = ui.input(|i| i.modifiers);
                                if input.shift {
                                    0.01
                                } else if cfg!(target_os = "macos") && input.mac_cmd || !cfg!(target_os = "macos") && input.ctrl {
                                    1.0
                                } else {
                                    0.1
                                }
                            };

                            match val {
                                Value::String(s) => {
                                    ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(s));
                                }
                                Value::Int(n) => {
                                    ui.add(DragValue::new(n).speed(1.0));
                                }
                                Value::Float(f) => {
                                    ui.add(DragValue::new(f).speed(speed));
                                }
                                Value::Bool(b) => {
                                    if ui.button(if *b { "✅" } else { "❌" }).clicked() {
                                        *b = !*b;
                                    }
                                }
                                Value::Vec3(v) => {
                                    ui.horizontal(|ui| {
                                        ui.add(DragValue::new(&mut v[0]).speed(speed));
                                        ui.add(DragValue::new(&mut v[1]).speed(speed));
                                        ui.add(DragValue::new(&mut v[2]).speed(speed));
                                    });
                                }
                            }

                            if ui.button("🗑️").clicked() {
                                log::debug!("Trashing {}", key);
                                to_delete = Some(key);
                            }

                            ui.end_row();
                        }

                        if let Some(key) = to_delete {
                            self.custom_properties.remove(&key);
                        }

                        if let Some((old_key, new_key)) = to_rename {
                            if new_key.is_empty() {
                                warn!("Skipping rename to empty key");
                            } else if old_key != new_key {
                                if let Some(val) = self.custom_properties.remove(&old_key) {
                                    self.custom_properties.insert(new_key, val);
                                } else {
                                    warn!("Failed to rename property: old key not found");
                                }
                            }
                        }

                        if ui.button("Add").clicked() {
                            log::debug!("Inserting new default value");
                            let mut new_key = String::from("new_property");
                            let mut counter = 1;
                            while self.custom_properties.contains_key(&new_key) {
                                new_key = format!("new_property_{}", counter);
                                counter += 1;
                            }
                            self.custom_properties.insert(new_key, Value::default());
                        }
                    });
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
        ui.vertical(|ui| {
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
        ui.vertical(|ui| {
            CollapsingHeader::new("Scripting")
                .default_open(true)
                .show(ui, |ui| {
                    CollapsingHeader::new("Tags")
                        .default_open(true)
                        .show(ui, |ui| {
                            let mut local_del: Option<usize> = None;
                            for (i, tag) in self.tags.iter_mut().enumerate() {
                                let current_width = ui.available_width();
                                ui.horizontal(|ui| {
                                    ui.add_sized([current_width*70.0/100.0, 20.0], TextEdit::singleline(tag));
                                    if ui.button("🗑️").clicked() {
                                        local_del = Some(i);
                                    }
                                });
                            }
                            if let Some(i) = local_del {
                                self.tags.remove(i);
                            }
                            if ui.button("➕ Add").clicked() {
                                self.tags.push(String::new())
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
        ui.vertical(|ui| {
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
        ui.vertical(|ui| {
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
        ui.vertical(|ui| {
            CollapsingHeader::new("Component").show(ui, |ui| {
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
        });
    }
}
