use futures::executor::block_on;
use glam::{DMat4, DQuat, DVec3, Mat4};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use wgpu::{Buffer, util::DeviceExt};

use crate::{
    graphics::{Instance, SharedGraphicsContext},
    model::{LazyModel, LazyType, Model},
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

/// Creates a new adopted entity in a lazy method. It fetches the data first (which can be done on a separate
/// thread). After, the [`LazyAdoptedEntity::poke()`] function can be called to convert the Lazy to a Real adopted entity.
#[derive(Default)]
pub struct LazyAdoptedEntity {
    lazy_model: LazyModel,
    #[allow(dead_code)]
    label: String,
}

impl LazyAdoptedEntity {
    /// Create a LazyAdoptedEntity from a file path (can be run on background thread)
    pub async fn from_file(path: &PathBuf, label: Option<&str>) -> anyhow::Result<Self> {
        let buffer = tokio::fs::read(path).await?;
        Self::from_memory(buffer, label).await
    }

    /// Create a LazyAdoptedEntity from memory buffer (can be run on background thread)
    pub async fn from_memory(
        buffer: impl AsRef<[u8]>,
        label: Option<&str>,
    ) -> anyhow::Result<Self> {
        let lazy_model = Model::lazy_load(buffer, label).await?;
        let label_str = label.unwrap_or("LazyAdoptedEntity").to_string();

        Ok(Self {
            lazy_model,
            label: label_str,
        })
    }

    /// Create a LazyAdoptedEntity from an existing LazyModel
    pub fn from_lazy_model(lazy_model: LazyModel, label: Option<&str>) -> Self {
        let label_str = label.unwrap_or("LazyAdoptedEntity").to_string();
        Self {
            lazy_model,
            label: label_str,
        }
    }
}

impl LazyType for LazyAdoptedEntity {
    type T = AdoptedEntity;

    fn poke(self, graphics: Arc<SharedGraphicsContext>) -> anyhow::Result<Self::T> {
        let model = self.lazy_model.poke(graphics.clone())?;
        Ok(block_on(AdoptedEntity::adopt(graphics, model)))
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
    pub async fn new(
        graphics: Arc<SharedGraphicsContext>,
        path: &PathBuf,
        label: Option<&str>,
    ) -> anyhow::Result<Self> {
        let model = Model::load(graphics.clone(), path, label.clone()).await?;
        Ok(Self::adopt(graphics, model).await)
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

    pub async fn adopt(graphics: Arc<SharedGraphicsContext>, model: Model) -> Self {
        let label = model.label.clone();
        let instance = Instance::new(DVec3::ZERO, DQuat::IDENTITY, DVec3::ONE);
        let initial_matrix = DMat4::IDENTITY; // Default; update in new() if transform provided
        let instance_raw = instance.to_raw();
        let instance_buffer =
            graphics
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some(&label),
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

    pub fn update(&mut self, graphics: Arc<SharedGraphicsContext>, transform: &Transform) {
        let current_matrix = transform.matrix();
        if self.previous_matrix != current_matrix {
            self.instance = Instance::from_matrix(current_matrix);
            let instance_raw = self.instance.to_raw();
            if let Some(buffer) = &self.instance_buffer {
                graphics
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
