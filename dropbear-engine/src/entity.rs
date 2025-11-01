use glam::{DMat4, DQuat, DVec3, Mat4};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};

use crate::{
    graphics::{Instance, SharedGraphicsContext},
    model::{LoadedModel, Model, ModelId},
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

#[derive(Clone)]
pub struct MeshRenderer {
    handle: LoadedModel,
    pub instance: Instance,
    pub previous_matrix: DMat4,
    pub is_selected: bool,
}

impl MeshRenderer {
    pub async fn from_path(
        graphics: Arc<SharedGraphicsContext>,
        path: impl AsRef<Path>,
        label: Option<&str>,
    ) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let handle = Model::load(graphics, &path, label).await?;
        Ok(Self::from_handle(handle))
    }

    pub fn from_handle(handle: LoadedModel) -> Self {
        Self {
            handle,
            instance: Instance::new(DVec3::ZERO, DQuat::IDENTITY, DVec3::ONE),
            previous_matrix: DMat4::IDENTITY,
            is_selected: false,
        }
    }

    pub fn model(&self) -> Arc<Model> {
        self.handle.get()
    }

    pub fn model_id(&self) -> ModelId {
        self.handle.id()
    }

    pub fn handle(&self) -> &LoadedModel {
        &self.handle
    }

    pub fn handle_mut(&mut self) -> &mut LoadedModel {
        &mut self.handle
    }

    pub fn make_model_mut(&mut self) -> &mut Model {
        self.handle.make_mut()
    }

    pub fn update(&mut self, transform: &Transform) {
        let current_matrix = transform.matrix();
        if self.previous_matrix != current_matrix {
            self.instance = Instance::from_matrix(current_matrix);
            self.previous_matrix = current_matrix;
        }
    }

    pub fn set_handle(&mut self, handle: LoadedModel) {
        self.handle = handle;
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
