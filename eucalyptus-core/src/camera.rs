use dropbear_engine::camera::Camera;
use glam::DVec3;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct CameraComponent {
    pub speed: f64,
    pub sensitivity: f64,
    pub fov_y: f64,
    pub camera_type: CameraType,
    pub starting_camera: bool,
}

impl Default for CameraComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl CameraComponent {
    pub fn new() -> Self {
        Self {
            speed: 5.0,
            sensitivity: 0.1,
            fov_y: 60.0,
            camera_type: CameraType::Normal,
            starting_camera: false,
        }
    }

    pub fn update(&mut self, camera: &mut Camera) {
        camera.speed = self.speed;
        camera.sensitivity = self.sensitivity;
        camera.fov_y = self.fov_y;
    }

    // setting camera offset is just adding the CameraFollowTarget struct
    // to the ecs system
}

pub struct PlayerCamera;

impl PlayerCamera {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> CameraComponent {
        CameraComponent {
            camera_type: CameraType::Player,
            ..CameraComponent::new()
        }
    }
}

pub struct DebugCamera;

impl DebugCamera {
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> CameraComponent {
        CameraComponent {
            camera_type: CameraType::Debug,
            ..CameraComponent::new()
        }
    }
}

// #[derive(Debug, Default, Clone)]
// pub struct CameraFollowTarget {
//     pub follow_target: String,
//     pub offset: DVec3,
// }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CameraType {
    Normal,
    Debug,
    Player,
}

impl Default for CameraType {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone)]
pub enum CameraAction {
    SetPlayerTarget { entity: hecs::Entity, offset: DVec3 },
    ClearPlayerTarget,
    SetCurrentPositionAsOffset(hecs::Entity),
}
