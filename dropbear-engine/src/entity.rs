use nalgebra::{Matrix4, UnitQuaternion, Vector3};
use wgpu::{util::DeviceExt, Buffer, RenderPass};

use crate::{camera::Camera, graphics::{Graphics, Instance}, model::{DrawModel, Model}};

#[derive(Default)]
pub struct Entity {
    model: Option<Model>,
    uniform: ModelUniform,
    uniform_buffer: Option<Buffer>,
    #[allow(unused)]
    instance: Instance,
    instance_buffer: Option<Buffer>,
}

impl Entity {
    pub fn adopt(graphics: &Graphics, model: Model, label: Option<&str>) -> Self {
        let uniform = ModelUniform::new();
        let uniform_buffer = graphics.create_uniform(uniform, Some("Entity Model Uniform"));
        
        let instance = Instance::new(
            Vector3::identity(),
            UnitQuaternion::identity()
        );

        let instance_buffer = graphics.state.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: match label { Some(_) => label, None => Some("instance buffer")},
            contents: bytemuck::cast_slice(&[instance.to_raw()]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });
        log::debug!("Successfully adopted Model");
        Self {
            model: Some(model),
            uniform,
            uniform_buffer: Some(uniform_buffer),
            instance,
            instance_buffer: Some(instance_buffer)
        }
    }

    // --- Model matrix manipulation for single mesh ---
    pub fn rotate_x(&mut self, angle_rad: f32) {
        let model_matrix = Matrix4::from(self.uniform.model);
        let rotated = model_matrix * Matrix4::new_rotation(Vector3::x() * angle_rad);
        self.uniform.model = rotated.into();
    }

    pub fn rotate_y(&mut self, angle_rad: f32) {
        let model_matrix = Matrix4::from(self.uniform.model);
        let rotated = model_matrix * Matrix4::new_rotation(Vector3::y() * angle_rad);
        self.uniform.model = rotated.into();
    }

    pub fn rotate_z(&mut self, angle_rad: f32) {
        let model_matrix = Matrix4::from(self.uniform.model);
        let rotated = model_matrix * Matrix4::new_rotation(Vector3::z() * angle_rad);
        self.uniform.model = rotated.into();
    }

    pub fn translate(&mut self, translation: Vector3<f32>) {
        let model_matrix = Matrix4::from(self.uniform.model);
        let translated = model_matrix * Matrix4::new_translation(&translation);
        self.uniform.model = translated.into();
    }

    pub fn scale(&mut self, scale: Vector3<f32>) {
        let model_matrix = Matrix4::from(self.uniform.model);
        let scaled = model_matrix * Matrix4::new_nonuniform_scaling(&scale);
        self.uniform.model = scaled.into();
    }

    pub fn update(&mut self, graphics: &Graphics) {
        if let Some(buffer) = &self.uniform_buffer {
            graphics.state.queue.write_buffer(
                buffer,
                0,
                bytemuck::cast_slice(&[self.uniform]),
            );
        }

        self.instance = Instance::from_matrix(Matrix4::from(self.uniform.model));

        if let Some(instance_buffer) = &self.instance_buffer {
            let instance_raw = self.instance.to_raw();
            graphics.state.queue.write_buffer(
                instance_buffer,
                0, 
                bytemuck::cast_slice(&[instance_raw])
            );
        }
    }

    pub fn render<'a>(&'a self, render_pass: &mut RenderPass<'a>, camera: &'a Camera) {
        if let Some(model) = &self.model {
            render_pass.set_vertex_buffer(1, self.instance_buffer.as_ref().unwrap().slice(..));
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
