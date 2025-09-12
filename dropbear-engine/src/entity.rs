use glam::{DMat4, DQuat, DVec3, Mat4};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use wgpu::{Buffer, util::DeviceExt};

use crate::{
    graphics::{Graphics, Instance},
    model::Model,
};

#[derive(Debug, Clone, Deserialize, Serialize, Copy, PartialEq)]
pub struct Transform {
    pub position: DVec3,
    pub rotation: DQuat,
    pub scale: DVec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: DVec3::new(0.0, 0.0, 0.0),
            rotation: DQuat::IDENTITY,
            scale: DVec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Transform {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn matrix(&self) -> DMat4 {
        DMat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    pub fn rotate_x(&mut self, angle_rad: f64) {
        self.rotation *= DQuat::from_euler(glam::EulerRot::XYZ, angle_rad, 0.0, 0.0);
    }

    pub fn rotate_y(&mut self, angle_rad: f64) {
        self.rotation *= DQuat::from_euler(glam::EulerRot::XYZ, 0.0, angle_rad, 0.0);
    }

    pub fn rotate_z(&mut self, angle_rad: f64) {
        self.rotation *= DQuat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, angle_rad);
    }

    pub fn translate(&mut self, translation: DVec3) {
        self.position += translation;
    }

    pub fn scale(&mut self, scale: DVec3) {
        self.scale *= scale;
    }
}

#[derive(Default)]
pub struct AdoptedEntity {
    pub model: Option<Model>,
    pub previous_matrix: DMat4,
    pub instance: Instance,
    pub instance_buffer: Option<Buffer>,
}

impl AdoptedEntity {
    pub fn new(graphics: &Graphics, path: &PathBuf, label: Option<&str>) -> anyhow::Result<Self> {
        let model = Model::load(graphics, path, label.clone())?;
        Ok(Self::adopt(graphics, model, label))
    }

    pub fn label(&self) -> &String {
        &self.model().label
    }

    pub fn label_mut(&mut self) -> &mut String {
        &mut self.model_mut().label
    }

    pub fn set_label(&mut self, label: &str) {
        self.model_mut().label = label.to_string();
    }

    pub fn adopt(graphics: &Graphics, model: Model, label: Option<&str>) -> Self {
        let instance = Instance::new(DVec3::ZERO, DQuat::IDENTITY, DVec3::ONE);
        let initial_matrix = DMat4::IDENTITY; // Default; update in new() if transform provided
        let instance_raw = instance.to_raw();
        let instance_buffer =
            graphics
                .state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: match label {
                        Some(l) => Some(l),
                        None => Some("instance buffer"),
                    },
                    contents: bytemuck::cast_slice(&[instance_raw]),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });

        Self {
            model: Some(model),
            instance,
            instance_buffer: Some(instance_buffer),
            previous_matrix: initial_matrix,
        }
    }

    pub fn update(&mut self, graphics: &Graphics, transform: &Transform) {
        let current_matrix = transform.matrix();
        if self.previous_matrix != current_matrix {
            self.instance = Instance::from_matrix(current_matrix);
            let instance_raw = self.instance.to_raw();
            if let Some(buffer) = &self.instance_buffer {
                graphics
                    .state
                    .queue
                    .write_buffer(buffer, 0, bytemuck::cast_slice(&[instance_raw]));
            }
            self.previous_matrix = current_matrix;
        }
    }

    pub fn model(&self) -> &Model {
        self.model.as_ref().unwrap()
    }

    pub fn model_mut(&mut self) -> &mut Model {
        self.model.as_mut().unwrap()
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ModelUniform {
    model: [[f32; 4]; 4],
}

impl ModelUniform {
    pub fn new() -> Self {
        Self {
            model: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
}
