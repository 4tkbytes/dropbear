use std::path::PathBuf;

use nalgebra::{Matrix4, UnitQuaternion, Vector3};
use wgpu::{BindGroup, Buffer, RenderPass, util::DeviceExt};

use crate::{
    camera::Camera,
    graphics::{Graphics, Instance},
    model::{DrawModel, Model},
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

#[derive(Debug, Clone)]
pub struct Transform {
    pub position: Vector3<f32>,
    pub rotation: UnitQuaternion<f32>,
    pub scale: Vector3<f32>,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: UnitQuaternion::identity(),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Transform {
    pub fn new() -> Self {
        Self {
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: UnitQuaternion::identity(),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }

    pub fn matrix(&self) -> Matrix4<f32> {
        Matrix4::new_translation(&self.position)
            * self.rotation.to_homogeneous()
            * Matrix4::new_nonuniform_scaling(&self.scale)
    }

    pub fn rotate_x(&mut self, angle_rad: f32) {
        self.rotation *= UnitQuaternion::from_euler_angles(angle_rad, 0.0, 0.0);
    }

    pub fn rotate_y(&mut self, angle_rad: f32) {
        self.rotation *= UnitQuaternion::from_euler_angles(0.0, angle_rad, 0.0);
    }

    pub fn rotate_z(&mut self, angle_rad: f32) {
        self.rotation *= UnitQuaternion::from_euler_angles(0.0, 0.0, angle_rad);
    }

    pub fn translate(&mut self, translation: Vector3<f32>) {
        self.position += translation;
    }

    pub fn scale(&mut self, scale: Vector3<f32>) {
        self.scale.component_mul_assign(&scale);
    }
}

impl AdoptedEntity {
    pub fn new(graphics: &Graphics, path: &PathBuf, label: Option<&str>) -> anyhow::Result<Self> {
        let model = Model::load(graphics, path, label.clone())?;
        Ok(Self::adopt(graphics, model, label))
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

        let instance = Instance::new(Vector3::identity(), UnitQuaternion::identity());

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
        self.uniform.model = transform.matrix().into();

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

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>, camera: &'a Camera) {
        if let Some(model) = &self.model {
            render_pass.set_vertex_buffer(1, self.instance_buffer.as_ref().unwrap().slice(..));
            render_pass.set_bind_group(2, self.uniform_bind_group.as_ref().unwrap(), &[]);
            render_pass.draw_model(model, camera.bind_group());
        }
    }

    pub fn model(&self) -> &Model {
        self.model.as_ref().unwrap()
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
            model: Matrix4::<f32>::identity().into(),
        }
    }
}
