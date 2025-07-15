use nalgebra::{Matrix4, Perspective3, Point3, Vector3};
use wgpu::{BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, ShaderStages};

use crate::graphics::Graphics;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Matrix4<f32> = Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.0,
    0.0, 0.0, 0.5, 1.0,
);

#[derive(Default)]
pub struct Camera {
    pub eye: Point3<f32>,
    pub target: Point3<f32>,
    pub up: Vector3<f32>,
    pub aspect: f32,
    pub fov_y: f32,
    pub znear: f32,
    pub zfar: f32,

    pub uniform: CameraUniform,
    pub buffer: Option<Buffer>,

    pub layout: Option<BindGroupLayout>,
    pub bind_group: Option<BindGroup>,
}

impl Camera {
    pub fn new(graphics: &Graphics, eye: Point3<f32>, target: Point3<f32>, up: Vector3<f32>, aspect: f32, fov_y: f32, znear: f32, zfar: f32) -> Self {
        let uniform = CameraUniform::new();
        let mut camera = Self {
            eye,
            target,
            up,
            aspect,
            fov_y,
            znear,
            zfar,
            uniform,
            buffer: None,
            layout: None,
            bind_group: None,
        };
        camera.update_view_proj();
        let buffer = graphics.create_uniform(camera.uniform, Some("Camera Uniform"));
        camera.create_bind_group_layout(graphics, buffer.clone());
        camera.buffer = Some(buffer);
        camera
    }

    pub fn build_vp(&self) -> Matrix4<f32> {
        let view = Matrix4::<f32>::look_at_rh(&self.eye, &self.target, &self.up);
        let proj = Perspective3::new(self.aspect, self.fov_y.to_radians(), self.znear, self.zfar);
        return OPENGL_TO_WGPU_MATRIX * proj.to_homogeneous() * view;
    }

    pub fn create_bind_group_layout(&mut self, graphics: &Graphics, camera_buffer: Buffer) {
        let camera_bind_group_layout = graphics.state.device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            entries: &[
                BindGroupLayoutEntry {
                    binding: 0,
                    visibility: ShaderStages::VERTEX,
                    ty: BindingType::Buffer {
                        ty: BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"), 
        });

        let camera_bind_group =  graphics.state.device.create_bind_group(&BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });
        self.layout = Some(camera_bind_group_layout);
        self.bind_group = Some(camera_bind_group);
    }

    pub fn update_view_proj(&mut self) {
        let mvp = self.build_vp();
        self.uniform.view_proj = mvp.into();
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Matrix4::<f32>::identity().into(),
        }
    }
}
