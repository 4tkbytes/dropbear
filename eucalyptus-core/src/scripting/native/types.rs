use std::ffi::c_char;

#[repr(C)]
pub struct NativeTransform {
    pub position_x: f64,
    pub position_y: f64,
    pub position_z: f64,
    pub rotation_x: f64,
    pub rotation_y: f64,
    pub rotation_z: f64,
    pub rotation_w: f64,
    pub scale_x: f64,
    pub scale_y: f64,
    pub scale_z: f64,
}

#[repr(C)]
pub struct Vector3D {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[repr(C)]
pub struct NativeCamera {
    pub label: *const c_char,
    pub entity_id: i64,
    pub eye: Vector3D,
    pub target: Vector3D,
    pub up: Vector3D,
    pub aspect: f64,
    pub fov_y: f64,
    pub znear: f64,
    pub zfar: f64,
    pub yaw: f64,
    pub pitch: f64,
    pub speed: f64,
    pub sensitivity: f64,
}