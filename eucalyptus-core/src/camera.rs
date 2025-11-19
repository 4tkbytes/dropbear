use crate::traits::SerializableComponent;
use dropbear_engine::camera::{Camera, CameraBuilder, CameraSettings};
use glam::DVec3;
use serde::{Deserialize, Serialize};
use dropbear_macro::SerializableComponent;
use crate::states::CameraConfig;

#[derive(Debug, Clone, Serialize, Deserialize, SerializableComponent)]
pub struct CameraComponent {
    pub settings: CameraSettings,
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
            settings: CameraSettings::new(5.0, 0.1, 60.0),
            camera_type: CameraType::Normal,
            starting_camera: false,
        }
    }

    pub fn update(&mut self, camera: &mut Camera) {
        camera.settings = self.settings;
    }
}

impl From<CameraConfig> for CameraBuilder {
    fn from(value: CameraConfig) -> Self {
        Self {
            eye: value.eye.into(),
            target: value.target.into(),
            up: value.up.into(),
            aspect: value.aspect,
            znear: value.near as f64,
            zfar: value.far as f64,
            settings: CameraSettings {
                speed: value.speed as f64,
                sensitivity: value.sensitivity as f64,
                fov_y: value.fov as f64,
            },
        }
    }
}

impl From<CameraConfig> for CameraComponent {
    fn from(value: CameraConfig) -> Self {
        let settings = CameraSettings::new(value.speed as f64, value.sensitivity as f64, value.fov as f64);
        Self {
            settings,
            camera_type: value.camera_type,
            starting_camera: value.starting_camera,
        }
    }
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
