use std::sync::Arc;

use glam::{DMat4, DQuat, DVec3, Mat4};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, ShaderStages,
};

use crate::graphics::SharedGraphicsContext;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: [[f64; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 0.5, 0.0],
    [0.0, 0.0, 0.5, 1.0],
];

#[derive(Default, Debug, Clone)]
pub struct Camera {
    pub label: String,

    pub eye: DVec3,
    pub target: DVec3,
    pub up: DVec3,
    pub aspect: f64,
    pub fov_y: f64,
    pub znear: f64,
    pub zfar: f64,
    pub yaw: f64,
    pub pitch: f64,

    pub uniform: CameraUniform,
    buffer: Option<Buffer>,

    layout: Option<BindGroupLayout>,
    bind_group: Option<BindGroup>,

    pub speed: f64,
    pub sensitivity: f64,

    pub view_mat: DMat4,
    pub proj_mat: DMat4,
}

impl Camera {
    /// Creates a new camera
    pub fn new(
        graphics: Arc<SharedGraphicsContext>,
        eye: DVec3,
        target: DVec3,
        up: DVec3,
        aspect: f64,
        fov_y: f64,
        znear: f64,
        zfar: f64,
        speed: f64,
        sensitivity: f64,
        label: Option<&str>,
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
            label: if let Some(l) = label {
                l.to_string()
            } else {
                String::from("Camera")
            },
            ..Default::default()
        };
        camera.update_view_proj();
        let buffer = graphics.create_uniform(camera.uniform, Some("Camera Uniform"));
        camera.create_bind_group_layout(graphics.clone(), buffer.clone());
        camera.buffer = Some(buffer);
        log::debug!("Created new camera");
        camera
    }

    /// Creates a default camera
    pub fn predetermined(graphics: Arc<SharedGraphicsContext>, label: Option<&str>) -> Self {
        Self::new(
            graphics.clone(),
            DVec3::new(0.0, 1.0, 2.0),
            DVec3::new(0.0, 0.0, 0.0),
            DVec3::Y,
            (graphics.screen_size.0 / graphics.screen_size.1).into(),
            45.0,
            0.1,
            100.0,
            1.0,
            0.002,
            label,
        )
    }

    pub fn rotation(&self) -> DQuat {
        let yaw = DQuat::from_axis_angle(DVec3::Y, self.yaw);
        let pitch = DQuat::from_axis_angle(DVec3::X, self.pitch);
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

    pub fn forward(&self) -> DVec3 {
        (self.target - self.eye).normalize()
    }

    pub fn position(&self) -> DVec3 {
        self.eye
    }

    pub fn debug_camera_state(&self) {
        let camera = self;
        log::debug!("Camera state:");
        log::debug!("  Eye: {:?}", camera.eye);
        log::debug!("  Target: {:?}", camera.target);
        log::debug!("  Up: {:?}", camera.up);
        log::debug!("  FOV Y: {}", camera.fov_y);
        log::debug!("  Aspect: {}", camera.aspect);
        log::debug!("  Z Near: {}", camera.znear);
        log::debug!("  Proj Mat finite: {}", camera.proj_mat.is_finite());
        log::debug!("  View Mat finite: {}", camera.view_mat.is_finite());
    }

    fn build_vp(&mut self) -> DMat4 {
        let view = DMat4::look_at_lh(self.eye, self.target, self.up);
        let proj = DMat4::perspective_infinite_reverse_lh(
            self.fov_y.to_radians(),
            self.aspect,
            self.znear,
        );

        self.view_mat = view.clone();
        self.proj_mat = proj.clone();

        let result = DMat4::from_cols_array_2d(&OPENGL_TO_WGPU_MATRIX) * proj * view;
        result
    }

    pub fn create_bind_group_layout(&mut self, graphics: Arc<SharedGraphicsContext>, camera_buffer: Buffer) {
        let camera_bind_group_layout =
            graphics
                .device
                .create_bind_group_layout(&BindGroupLayoutDescriptor {
                    entries: &[BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::VERTEX | ShaderStages::FRAGMENT,
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

    pub fn update(&mut self, graphics: Arc<SharedGraphicsContext>) {
        self.update_view_proj();
        graphics.queue.write_buffer(
            &self.buffer.as_ref().unwrap(),
            0,
            bytemuck::cast_slice(&[self.uniform]),
        );
    }

    pub fn update_view_proj(&mut self) {
        let mvp = self.build_vp();
        self.uniform.view_proj = mvp.as_mat4().to_cols_array_2d();
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
        // LH: right = up.cross(forward)
        let right = self.up.cross(forward).normalize();
        self.eye += right * self.speed;
        self.target += right * self.speed;
    }

    pub fn move_left(&mut self) {
        let forward = (self.target - self.eye).normalize();
        let right = self.up.cross(forward).normalize();
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

    pub fn track_mouse_delta(&mut self, dx: f64, dy: f64) {
        let sensitivity = self.sensitivity;
        self.yaw -= dx * sensitivity;
        self.pitch -= dy * sensitivity;
        let max_pitch = std::f64::consts::FRAC_PI_2 - 0.01;
        self.pitch = self.pitch.clamp(-max_pitch, max_pitch);
        let dir = DVec3::new(
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
    pub view_position: [f32; 4],
    pub view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_position: [0.0; 4],
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update(&mut self, camera: &mut Camera) {
        self.view_position = camera.eye.as_vec3().extend(1.0).to_array();
        self.view_proj = (DMat4::from_cols_array_2d(&OPENGL_TO_WGPU_MATRIX) * camera.build_vp())
            .as_mat4()
            .to_cols_array_2d();
    }
}
