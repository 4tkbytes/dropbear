use futures::executor::block_on;
use glam::{DMat4, DQuat, DVec3, Mat4};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use wgpu::{Buffer, util::DeviceExt};

use crate::{
    graphics::{Instance, SharedGraphicsContext},
    model::{LazyModel, LazyType, Model},
};

/// A type that represents a position, rotation and scale of an entity
///
/// This type is the most primitive model, as it implements most traits.
#[repr(C)]
#[derive(Debug, Clone, Deserialize, Serialize, Copy, PartialEq)]
pub struct Transform {
    /// The position of the entity as [`DVec3`]
    pub position: DVec3,
    /// The rotation of the entity as [`DQuat`]
    pub rotation: DQuat,
    /// The scale of the entity as [`DVec3`]
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
    /// Creates a new default instance of Transform
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the matrix of the model
    pub(crate) fn matrix(&self) -> DMat4 {
        DMat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }

    /// Rotates the model on its X axis by a certain angle
    pub fn rotate_x(&mut self, angle_rad: f64) {
        self.rotation *= DQuat::from_euler(glam::EulerRot::XYZ, angle_rad, 0.0, 0.0);
    }

    /// Rotates the model on its Y axis by a certain value
    pub fn rotate_y(&mut self, angle_rad: f64) {
        self.rotation *= DQuat::from_euler(glam::EulerRot::XYZ, 0.0, angle_rad, 0.0);
    }

    /// Rotates the model on its Z axis by a certain value
    pub fn rotate_z(&mut self, angle_rad: f64) {
        self.rotation *= DQuat::from_euler(glam::EulerRot::XYZ, 0.0, 0.0, angle_rad);
    }

    /// Translates (moves) the model by a translation [`DVec3`].
    ///
    /// Doesn't replace the position value,
    /// it adds the value.
    pub fn translate(&mut self, translation: DVec3) {
        self.position += translation;
    }

    /// Scales the model by a scale value.
    ///
    /// Doesn't replace the scale value, just multiplies.
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
        let buffer = std::fs::read(path)?;
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

#[derive(Clone)]
pub struct AdoptedEntity {
    pub model: Arc<Model>,
    pub previous_matrix: DMat4,
    pub instance: Instance,
    pub instance_buffer: Option<Buffer>,
    pub dirty: bool,
    last_frame_rendered: Option<u64>,
}

impl AdoptedEntity {
    pub async fn new(
        graphics: Arc<SharedGraphicsContext>,
        path: impl AsRef<Path>,
        label: Option<&str>,
    ) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let model = Model::load(graphics.clone(), &path, label).await?;
        Ok(Self::adopt(graphics, model).await)
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
            model: Arc::new(model),
            instance,
            instance_buffer: Some(instance_buffer),
            previous_matrix: initial_matrix,
            dirty: true,
            last_frame_rendered: None,
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

    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    pub fn flush_to_gpu(&mut self, graphics: Arc<SharedGraphicsContext>) {
        if self.dirty {
            let instance_raw = self.instance.to_raw();
            if let Some(buffer) = &self.instance_buffer {
                graphics
                    .queue
                    .write_buffer(buffer, 0, bytemuck::cast_slice(&[instance_raw]));
            }
            self.dirty = false;
        }
    }

    pub fn mark_rendered(&mut self, frame_number: u64) {
        self.last_frame_rendered = Some(frame_number);
    }

    pub fn was_recently_rendered(&self, current_frame: u64, max_frames_ago: u64) -> bool {
        if let Some(last_frame) = self.last_frame_rendered {
            current_frame - last_frame <= max_frames_ago
        } else {
            false
        }
    }

    pub fn get_instance_buffer(&mut self, graphics: Arc<SharedGraphicsContext>) -> Option<&Buffer> {
        self.flush_to_gpu(graphics);
        self.instance_buffer.as_ref()
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
