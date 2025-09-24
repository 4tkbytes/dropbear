//! Camera and components related to cameras. 

use std::sync::Arc;

use glam::{DMat4, DQuat, DVec3, Mat4};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, BindGroupLayoutDescriptor,
    BindGroupLayoutEntry, BindingType, Buffer, BufferBindingType, ShaderStages,
};

use crate::graphics::SharedGraphicsContext;

/// Matrix that converts OpenGL (from [`glam`]) to [`wgpu`] values
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: [[f64; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 0.5, 0.0],
    [0.0, 0.0, 0.5, 1.0],
];

/// The basic values of a Camera.
#[derive(Default, Debug, Clone)]
pub struct Camera {
    /// The name of the camera
    pub label: String,

    /// Eye of camera / Position
    pub eye: DVec3,
    /// Target of camera / Looking at
    pub target: DVec3,
    /// Up
    pub up: DVec3,
    /// Aspect ratio
    pub aspect: f64,
    /// FOV of camera
    pub fov_y: f64,
    /// Near buffer?
    pub znear: f64,
    /// Far buffer?
    pub zfar: f64,
    /// Yaw (rotation)
    pub yaw: f64,
    /// Pitch (rotation)
    pub pitch: f64,

    /// Uniform/interface for Rust and the GPU
    pub uniform: CameraUniform,
    buffer: Option<Buffer>,

    layout: Option<BindGroupLayout>,
    bind_group: Option<BindGroup>,

    /// Speed of the camera
    pub speed: f64,
    /// Sensitivity of the mouse for the camera
    pub sensitivity: f64,

    /// View matrix
    pub view_mat: DMat4,
    /// Projection Matrix
    pub proj_mat: DMat4,
}

/// A simple builder/struct that allows you to build a [`Camera`]
pub struct CameraBuilder {
    pub eye: DVec3,
    pub target: DVec3,
    pub up: DVec3,
    pub aspect: f64,
    pub fov_y: f64,
    pub znear: f64,
    pub zfar: f64,
    pub speed: f64,
    pub sensitivity: f64,
}

impl Camera {
    /// Creates a new camera
    pub fn new(
        graphics: Arc<SharedGraphicsContext>,
        builder: CameraBuilder,
        label: Option<&str>,
    ) -> Self {
        let uniform = CameraUniform::new();
        let mut camera = Self {
            eye: builder.eye,
            target: builder.target,
            up: builder.up,
            aspect: builder.aspect,
            fov_y: builder.fov_y,
            znear: builder.znear,
            zfar: builder.zfar,
            uniform,
            buffer: None,
            layout: None,
            bind_group: None,
            speed: builder.speed,
            yaw: 0.0,
            pitch: 0.0,
            sensitivity: builder.sensitivity,
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
            CameraBuilder {
                eye: DVec3::new(0.0, 1.0, 2.0),
                target: DVec3::new(0.0, 0.0, 0.0),
                up: DVec3::Y,
                aspect: (graphics.screen_size.0 / graphics.screen_size.1).into(),
                fov_y: 45.0,
                znear: 0.1,
                zfar: 100.0,
                speed: 1.0,
                sensitivity: 0.002,
            },
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

    /// Prints out the values of the camera. 
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

        self.view_mat = view;
        self.proj_mat = proj;

        DMat4::from_cols_array_2d(&OPENGL_TO_WGPU_MATRIX) * proj * view
    }

    pub fn create_bind_group_layout(
        &mut self,
        graphics: Arc<SharedGraphicsContext>,
        camera_buffer: Buffer,
    ) {
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

        let camera_bind_group = graphics.device.create_bind_group(&BindGroupDescriptor {
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
            self.buffer.as_ref().unwrap(),
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
