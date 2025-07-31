use nalgebra::{Matrix4, Perspective3, Point3, UnitQuaternion, Vector3};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, ShaderStages,
};

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
    pub yaw: f32,
    pub pitch: f32,

    pub uniform: CameraUniform,
    buffer: Option<Buffer>,

    layout: Option<BindGroupLayout>,
    bind_group: Option<BindGroup>,

    pub speed: f32,
    pub sensitivity: f32,

    pub view_mat: Matrix4<f32>,
    pub proj_mat: Matrix4<f32>,
}

impl Camera {
    pub fn new(
        graphics: &Graphics,
        eye: Point3<f32>,
        target: Point3<f32>,
        up: Vector3<f32>,
        aspect: f32,
        fov_y: f32,
        znear: f32,
        zfar: f32,
        speed: f32,
        sensitivity: f32,
    ) -> Self {
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
            speed,
            yaw: 0.0,
            pitch: 0.0,
            sensitivity,
            ..Default::default()
        };
        camera.update_view_proj();
        let buffer = graphics.create_uniform(camera.uniform, Some("Camera Uniform"));
        camera.create_bind_group_layout(graphics, buffer.clone());
        camera.buffer = Some(buffer);
        log::debug!("Created new camera");
        camera
    }

    pub fn predetermined(graphics: &Graphics) -> Self {
        Self::new(
            graphics,
            Point3::new(0.0, 1.0, 2.0),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::y(),
            (graphics.state.config.width / graphics.state.config.height) as f32,
            45.0,
            0.1,
            100.0,
            0.125,
            0.002,
        )
    }

    pub fn rotation(&self) -> UnitQuaternion<f32> {
        let yaw = UnitQuaternion::from_axis_angle(&Vector3::y_axis(), self.yaw);
        let pitch = UnitQuaternion::from_axis_angle(&Vector3::x_axis(), self.pitch);
        yaw * pitch
    }

    pub fn uniform_buffer(&self) -> &Buffer {
        self.buffer.as_ref().unwrap()
    }

    pub fn layout(&self) -> &BindGroupLayout {
        self.layout.as_ref().unwrap()
    }

    pub fn bind_group(&self) -> &BindGroup {
        self.bind_group.as_ref().unwrap()
    }

    pub fn forward(&self) -> Vector3<f32> {
        (self.target - self.eye).normalize()
    }

    pub fn position(&self) -> Point3<f32> {
        self.eye
    }

    fn build_vp(&mut self) -> Matrix4<f32> {
        let view = Matrix4::<f32>::look_at_rh(&self.eye, &self.target, &self.up);
        let proj = Perspective3::new(self.aspect, self.fov_y.to_radians(), self.znear, self.zfar);
        self.view_mat = view.clone();
        self.proj_mat = proj.clone().to_homogeneous();
        let result = OPENGL_TO_WGPU_MATRIX * proj.to_homogeneous() * view;
        result
    }

    pub fn create_bind_group_layout(&mut self, graphics: &Graphics, camera_buffer: Buffer) {
        let camera_bind_group_layout =
            graphics
                .state
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                    label: Some("camera_bind_group_layout"),
                });

        let camera_bind_group = graphics
            .state
            .device
            .create_bind_group(&BindGroupDescriptor {
                layout: &camera_bind_group_layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
                label: Some("camera_bind_group"),
            });
        self.layout = Some(camera_bind_group_layout);
        self.bind_group = Some(camera_bind_group);
    }

    pub fn update(&mut self, graphics: &Graphics) {
        self.update_view_proj();
        graphics.state.queue.write_buffer(
            &self.buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&[self.uniform]),
        );
    }

    pub fn update_view_proj(&mut self) {
        let mvp = self.build_vp();
        self.uniform.view_proj = mvp.into();
    }

    pub fn move_forwards(&mut self) {
        let forward = (self.target - self.eye).normalize();
        self.eye += forward * self.speed;
        self.target += forward * self.speed;
    }

    pub fn move_back(&mut self) {
        let forward = (self.target - self.eye).normalize();
        self.eye -= forward * self.speed;
        self.target -= forward * self.speed;
    }

    pub fn move_right(&mut self) {
        let forward = (self.target - self.eye).normalize();
        let right = forward.cross(&self.up).normalize();
        self.eye += right * self.speed;
        self.target += right * self.speed;
    }

    pub fn move_left(&mut self) {
        let forward = (self.target - self.eye).normalize();
        let right = forward.cross(&self.up).normalize();
        self.eye -= right * self.speed;
        self.target -= right * self.speed;
    }

    pub fn move_up(&mut self) {
        let up = self.up.normalize();
        self.eye += up * self.speed;
        self.target += up * self.speed;
    }

    pub fn move_down(&mut self) {
        let up = self.up.normalize();
        self.eye -= up * self.speed;
        self.target -= up * self.speed;
    }

    pub fn track_mouse_delta(&mut self, dx: f32, dy: f32) {
        let sensitivity = self.sensitivity;
        self.yaw += dx * sensitivity;
        self.pitch -= dy * sensitivity;
        let max_pitch = std::f32::consts::FRAC_PI_2 - 0.01;
        self.pitch = self.pitch.clamp(-max_pitch, max_pitch);
        let dir = Vector3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        );
        self.target = self.eye + dir;
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
