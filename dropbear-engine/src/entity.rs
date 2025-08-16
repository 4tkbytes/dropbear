use std::path::PathBuf;

use glam::{DMat4, DQuat, DVec3, Mat4};
use serde::{Deserialize, Serialize};
// use nalgebra::{Matrix4, UnitQuaternion, Vector3};
use wgpu::{util::DeviceExt, BindGroup, Buffer, RenderPass, RenderPipeline};

use crate::{
    camera::Camera, graphics::{Graphics, Instance}, lighting::LightManager, model::{DrawModel, Model}
};

#[derive(Default)]
pub struct AdoptedEntity {
    model: Option<Model>,
    uniform: ModelUniform,
    uniform_buffer: Option<Buffer>,
    uniform_bind_group: Option<BindGroup>,
    #[allow(unused)]
    instance: Instance,
    instance_buffer: Option<Buffer>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Copy)]
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

impl AdoptedEntity {
    pub fn new(graphics: &Graphics, path: &PathBuf, label: Option<&str>) -> anyhow::Result<Self> {
        let model = Model::load(graphics, path, label.clone())?;
        Ok(Self::adopt(graphics, model, label))
    }

    pub fn label(&self) -> &String {
        &self.model().label
    }

    pub fn set_label(&mut self, label: &str) {
        self.model_mut().label = label.to_string();
    }

    pub fn adopt(graphics: &Graphics, model: Model, label: Option<&str>) -> Self {
        let uniform = ModelUniform::new();
        let uniform_buffer = graphics.create_uniform(uniform, Some("Entity Model Uniform"));

        let model_layout = graphics.create_model_uniform_bind_group_layout();
        let uniform_bind_group =
            graphics
                .state
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("model_uniform_bind_group"),
                    layout: &model_layout,
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    }],
                });

        let instance = Instance::new(DVec3::ONE, DQuat::IDENTITY, DVec3::ONE);

        let instance_buffer =
            graphics
                .state
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: match label {
                        Some(_) => label,
                        None => Some("instance buffer"),
                    },
                    contents: bytemuck::cast_slice(&[instance.to_raw()]),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });
        log::debug!("Successfully adopted Model");
        Self {
            model: Some(model),
            uniform,
            uniform_buffer: Some(uniform_buffer),
            uniform_bind_group: Some(uniform_bind_group),
            instance,
            instance_buffer: Some(instance_buffer),
        }
    }

    pub fn update(&mut self, graphics: &Graphics, transform: &Transform) {
        self.uniform.model = transform.matrix().as_mat4().to_cols_array_2d();

        if let Some(buffer) = &self.uniform_buffer {
            graphics
                .state
                .queue
                .write_buffer(buffer, 0, bytemuck::cast_slice(&[self.uniform]));
        }

        self.instance = Instance::from_matrix(transform.matrix());

        if let Some(instance_buffer) = &self.instance_buffer {
            let instance_raw = self.instance.to_raw();
            graphics.state.queue.write_buffer(
                instance_buffer,
                0,
                bytemuck::cast_slice(&[instance_raw]),
            );
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>, pipeline: &RenderPipeline, camera: &'a Camera, light_manager: &'a LightManager) {
        if let Some(model) = &self.model {
            render_pass.set_vertex_buffer(1, self.instance_buffer.as_ref().unwrap().slice(..));

            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(2, self.uniform_bind_group.as_ref().unwrap(), &[]);
            for (_, light) in light_manager.iter() {
                render_pass.draw_model(model, camera.bind_group(), light.bind_group());
            }
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
