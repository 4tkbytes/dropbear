use std::collections::HashSet;

use dropbear_engine::camera::Camera;
use glam::DVec3;
use serde::{Deserialize, Serialize};
use winit::keyboard::KeyCode;

#[derive(Clone)]
pub struct CameraComponent {
    pub speed: f64,
    pub sensitivity: f64,
    pub camera_type: CameraType
}

impl CameraComponent {
    pub fn new() -> Self {
        Self {
            speed: 5.0,
            sensitivity: 0.002,
            camera_type: CameraType::Normal,
        }
    }

    // setting camera offset is just adding the CameraFollowTarget struct
    // to the ecs system
}

pub struct PlayerCamera;

impl PlayerCamera {
    pub fn new() -> CameraComponent {
        CameraComponent {
            camera_type: CameraType::Player,
            ..CameraComponent::new()
        }
    }

    pub fn handle_keyboard_input(
        camera: &mut Camera,
        pressed_keys: &HashSet<KeyCode>
    ) {
        for key in pressed_keys {
            match key {
                KeyCode::KeyW => camera.move_forwards(),
                KeyCode::KeyA => camera.move_left(),
                KeyCode::KeyD => camera.move_right(),
                KeyCode::KeyS => camera.move_back(),
                KeyCode::ShiftLeft => camera.move_down(),
                KeyCode::Space => camera.move_up(),
                _ => {}
            }
        }
    }

    pub fn handle_mouse_input(camera: &mut Camera, component: &CameraComponent, mouse_delta: Option<(f64, f64)>) {
        if let Some((dx, dy)) = mouse_delta {
            camera.track_mouse_delta(dx * component.sensitivity, dy * component.sensitivity);
        }
    }
}

pub struct DebugCamera;

impl DebugCamera {
    pub fn new() -> CameraComponent {
        CameraComponent {
            camera_type: CameraType::Debug,
            ..CameraComponent::new()
        }
    }

    pub fn handle_keyboard_input(
        camera: &mut Camera,
        pressed_keys: &HashSet<KeyCode>
    ) {
        for key in pressed_keys {
            match key {
                KeyCode::KeyW => camera.move_forwards(),
                KeyCode::KeyA => camera.move_left(),
                KeyCode::KeyD => camera.move_right(),
                KeyCode::KeyS => camera.move_back(),
                KeyCode::ShiftLeft => camera.move_down(),
                KeyCode::Space => camera.move_up(),
                _ => {}
            }
        }
    }

    pub fn handle_mouse_input(camera: &mut Camera, component: &CameraComponent, mouse_delta: Option<(f64, f64)>) {
        if let Some((dx, dy)) = mouse_delta {
            camera.track_mouse_delta(dx * component.sensitivity, dy * component.sensitivity);
        }
    }
}

#[derive(Default, Clone)]
pub struct CameraFollowTarget {
    pub follow_target: String,
    pub offset: DVec3,
}

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
}