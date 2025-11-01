//! This module should describe the different components that are editable in the resource inspector.

use crate::editor::{EntityType, Signal, StaticallyKept, UndoableAction};
use dropbear_engine::attenuation::ATTENUATION_PRESETS;
use dropbear_engine::entity::{MeshRenderer, Transform};
use dropbear_engine::lighting::{Light, LightComponent, LightType};
use egui::{CollapsingHeader, ComboBox, DragValue, Grid, RichText, TextEdit, Ui};
use eucalyptus_core::states::{ModelProperties, Property, ScriptComponent, Value};
use eucalyptus_core::warn;
use glam::{DVec3, Vec3};
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
            Value::String(_) => ValueType::String,
            Value::Int(_) => ValueType::Int,
            Value::Float(_) => ValueType::Float,
            Value::Bool(_) => ValueType::Bool,
            Value::Vec3(_) => ValueType::Vec3,
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

fn wrap_angle_degrees(angle: f64) -> f64 {
    (angle + 180.0).rem_euclid(360.0) - 180.0
}

fn reconcile_angle(angle: f64, reference: f64) -> f64 {
    let delta = wrap_angle_degrees(angle - reference);
    wrap_angle_degrees(reference + delta)
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
                Grid::new("properties").striped(true).show(ui, |ui| {
                    ui.label(RichText::new("Key"));
                    ui.label(RichText::new("Type"));
                    ui.label(RichText::new("Value"));
                    ui.label(RichText::new("Action"));
                    ui.end_row();

                    let mut to_delete: Option<u64> = None;
                    let mut to_rename: Option<(u64, String)> = None;

                    for (_i, property) in self.custom_properties.iter_mut().enumerate() {
                        let mut edited_key = property.key.clone();
                        ui.add_sized([100.0, 20.0], TextEdit::singleline(&mut edited_key));

                        if edited_key != property.key {
                            to_rename = Some((property.id, edited_key));
                        }

                        let current_type = ValueType::from(&mut property.value);
                        let mut selected_type = current_type;

                        ComboBox::from_id_salt(format!("type_{}", property.id))
                            .selected_text(format!("{:?}", selected_type))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut selected_type,
                                    ValueType::String,
                                    "String",
                                );
                                ui.selectable_value(&mut selected_type, ValueType::Float, "Float");
                                ui.selectable_value(&mut selected_type, ValueType::Int, "Int");
                                ui.selectable_value(&mut selected_type, ValueType::Bool, "Bool");
                                ui.selectable_value(&mut selected_type, ValueType::Vec3, "Vec3");
                            });

                        if selected_type != current_type {
                            property.value = match selected_type {
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
                            } else if cfg!(target_os = "macos") && input.mac_cmd
                                || !cfg!(target_os = "macos") && input.ctrl
                            {
                                1.0
                            } else {
                                0.1
                            }
                        };

                        match &mut property.value {
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
                            log::debug!("Trashing {}", property.key);
                            to_delete = Some(property.id);
                        }

                        ui.end_row();
                    }

                    if let Some(id) = to_delete {
                        self.custom_properties.retain(|p| p.id != id);
                    }

                    if let Some((id, new_key)) = to_rename {
                        if let Some(property) =
                            self.custom_properties.iter_mut().find(|p| p.id == id)
                        {
                            property.key = new_key;
                        } else {
                            warn!("Failed to rename property: id not found");
                        }
                    }

                    if ui.button("Add").clicked() {
                        log::debug!("Inserting new default value");
                        let mut new_key = String::from("new_property");
                        let mut counter = 1;
                        while self.custom_properties.iter().any(|p| p.key == new_key) {
                            new_key = format!("new_property_{}", counter);
                            counter += 1;
                        }
                        self.custom_properties.push(Property {
                            id: self.next_id,
                            key: new_key,
                            value: Value::default(),
                        });
                        self.next_id += 1;
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
                                && let Some(orig) = cfg.transform_original_transform.take()
                            {
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
                                && let Some(orig) = cfg.transform_original_transform.take()
                            {
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
                                && let Some(orig) = cfg.transform_original_transform.take()
                            {
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

                    let cached_rotation = cfg.transform_rotation_cache.get(entity).copied();

                    let mut rotation_deg: DVec3 = if cfg.transform_in_progress {
                        cached_rotation.unwrap_or_else(|| {
                            let (x, y, z) = self.rotation.to_euler(glam::EulerRot::YXZ);
                            DVec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees())
                        })
                    } else {
                        let (x, y, z) = self.rotation.to_euler(glam::EulerRot::YXZ);
                        let mut degrees =
                            DVec3::new(x.to_degrees(), y.to_degrees(), z.to_degrees());

                        if let Some(prev) = cached_rotation {
                            degrees.x = reconcile_angle(degrees.x, prev.x);
                            degrees.y = reconcile_angle(degrees.y, prev.y);
                            degrees.z = reconcile_angle(degrees.z, prev.z);
                        }

                        degrees.x = wrap_angle_degrees(degrees.x);
                        degrees.y = wrap_angle_degrees(degrees.y);
                        degrees.z = wrap_angle_degrees(degrees.z);

                        cfg.transform_rotation_cache.insert(*entity, degrees);
                        degrees
                    };

                    let mut rotation_changed = false;

                    ui.horizontal(|ui| {
                        ui.label("X:");
                        let response = ui.add(
                            egui::DragValue::new(&mut rotation_deg.x)
                                .speed(0.5)
                                .suffix("°")
                                .range(-180.0..=180.0)
                                .fixed_decimals(2),
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
                                && let Some(orig) = cfg.transform_original_transform.take()
                            {
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
                            egui::DragValue::new(&mut rotation_deg.y)
                                .speed(0.5)
                                .suffix("°")
                                .range(-180.0..=180.0)
                                .fixed_decimals(2),
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
                                && let Some(orig) = cfg.transform_original_transform.take()
                            {
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
                            egui::DragValue::new(&mut rotation_deg.z)
                                .speed(0.5)
                                .suffix("°")
                                .range(-180.0..=180.0)
                                .fixed_decimals(2),
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
                                && let Some(orig) = cfg.transform_original_transform.take()
                            {
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
                        rotation_deg.x = wrap_angle_degrees(rotation_deg.x);
                        rotation_deg.y = wrap_angle_degrees(rotation_deg.y);
                        rotation_deg.z = wrap_angle_degrees(rotation_deg.z);

                        cfg.transform_rotation_cache.insert(*entity, rotation_deg);
                        self.rotation = glam::DQuat::from_euler(
                            glam::EulerRot::YXZ,
                            rotation_deg.x.to_radians(),
                            rotation_deg.y.to_radians(),
                            rotation_deg.z.to_radians(),
                        );
                    }
                    if ui.button("Reset Rotation").clicked() {
                        self.rotation = glam::DQuat::IDENTITY;
                        cfg.transform_rotation_cache.insert(*entity, DVec3::ZERO);
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
                                && let Some(orig) = cfg.transform_original_transform.take()
                            {
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
                                && let Some(orig) = cfg.transform_original_transform.take()
                            {
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
                                && let Some(orig) = cfg.transform_original_transform.take()
                            {
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
        _signal: &mut Signal,
        _label: &mut String,
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
                                    ui.add_sized(
                                        [current_width * 70.0 / 100.0, 20.0],
                                        TextEdit::singleline(tag),
                                    );
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

impl InspectableComponent for MeshRenderer {
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

                let resp = ui.text_edit_singleline(&mut self.make_model_mut().label);

                if resp.changed() {
                    if cfg.old_label_entity.is_none() {
                        cfg.old_label_entity = Some(*entity);
                        cfg.label_original = Some(self.handle().label.clone());
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
            });

            ui.label(format!("Entity ID: {}", entity.id()));
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
