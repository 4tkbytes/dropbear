use glam::DVec3;
use rustyscript::{serde_json, Runtime};
use serde::{Deserialize, Serialize};

use crate::{camera::CameraType, scripting::ScriptableModule};

impl ScriptableModule for SerializableCamera {
    fn register(runtime: &mut Runtime) -> anyhow::Result<()> {
        runtime.register_function("moveForward", Self::move_forward)?;
        runtime.register_function("moveBackward", Self::move_backward)?;
        runtime.register_function("moveLeft", Self::move_left)?;
        runtime.register_function("moveRight", Self::move_right)?;
        runtime.register_function("moveUp", Self::move_up)?;
        runtime.register_function("moveDown", Self::move_down)?;

        // Mouse look
        runtime.register_function("trackMouseDelta", Self::track_mouse_delta)?;

        // Camera switching and management
        runtime.register_function("switchToCameraByLabel", Self::switch_to_camera_by_label)?;
        runtime.register_function("getActiveCameraLabel", Self::get_active_camera_label)?;
        runtime.register_function("getAllCameraLabels", Self::get_all_camera_labels)?;
        runtime.register_function("getCamerasByType", Self::get_cameras_by_type)?;

        // Multi-camera manipulation
        runtime.register_function("manipulateCameraByLabel", Self::manipulate_camera_by_label)?;
        runtime.register_function("getCameraByLabel", Self::get_camera_by_label)?;
        runtime.register_function("setCameraByLabel", Self::set_camera_by_label)?;

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableCamera {
    pub label: String,
    pub eye: DVec3,
    pub target: DVec3,
    pub up: DVec3,
    pub aspect: f64,
    pub fov: f64,
    pub near: f64,
    pub far: f64,
    pub yaw: f64,
    pub pitch: f64,
    
    pub speed: f64,
    pub sensitivity: f64,

    pub camera_type: CameraType,
}

impl SerializableCamera {
    /// Moves the camera forward. 
    /// 
    /// # Parameters
    /// * args[0] - Self as a Camera. The value of the speed can be adjusted
    pub fn move_forward(args: &[serde_json::Value]) -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 1 {
            return Err(rustyscript::Error::Runtime("moveForward requires 1 arguments".to_string()));
        }

        let mut camera: SerializableCamera = serde_json::from_value(args[0].clone())
            .map_err(|e| rustyscript::Error::Runtime(format!("Invalid camera: {}", e)))?;

        let forward = (camera.target - camera.eye).normalize();
        camera.eye += forward * camera.speed;
        camera.target += forward * camera.speed;

        serde_json::to_value(camera)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Camera: {}", e)))
    }

    /// Moves the camera back. 
    /// 
    /// # Parameters
    /// * args[0] - Self as a Camera. The value of the speed can be adjusted
    pub fn move_backward(args: &[serde_json::Value]) -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 1 {
            return Err(rustyscript::Error::Runtime("moveBackward requires 1 arguments".to_string()));
        }

        let mut camera: SerializableCamera = serde_json::from_value(args[0].clone())
            .map_err(|e| rustyscript::Error::Runtime(format!("Invalid camera: {}", e)))?;

        let forward = (camera.target - camera.eye).normalize();
        camera.eye -= forward * camera.speed;
        camera.target -= forward * camera.speed;

        serde_json::to_value(camera)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Camera: {}", e)))
    }

    /// Moves the camera left. 
    /// 
    /// # Parameters
    /// * args[0] - Self as a Camera. The value of the speed can be adjusted
    pub fn move_left(args: &[serde_json::Value]) -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 1 {
            return Err(rustyscript::Error::Runtime("moveLeft requires 1 arguments".to_string()));
        }

        let mut camera: SerializableCamera = serde_json::from_value(args[0].clone())
            .map_err(|e| rustyscript::Error::Runtime(format!("Invalid camera: {}", e)))?;

        let forward = (camera.target - camera.eye).normalize();
        let right = camera.up.cross(forward).normalize();
        camera.eye -= right * camera.speed;
        camera.target -= right * camera.speed;

        serde_json::to_value(camera)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Camera: {}", e)))
    }

    /// Moves the camera right. 
    /// 
    /// # Parameters
    /// * args[0] - Self as a Camera. The value of the speed can be adjusted
    pub fn move_right(args: &[serde_json::Value]) -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 1 {
            return Err(rustyscript::Error::Runtime("moveRight requires 1 arguments".to_string()));
        }

        let mut camera: SerializableCamera = serde_json::from_value(args[0].clone())
            .map_err(|e| rustyscript::Error::Runtime(format!("Invalid camera: {}", e)))?;

        let forward = (camera.target - camera.eye).normalize();
        let right = camera.up.cross(forward).normalize();
        camera.eye += right * camera.speed;
        camera.target += right * camera.speed;

        serde_json::to_value(camera)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Camera: {}", e)))
    }

    /// Moves the camera up. 
    /// 
    /// # Parameters
    /// * args[0] - Self as a Camera. The value of the speed can be adjusted
    pub fn move_up(args: &[serde_json::Value]) -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 1 {
            return Err(rustyscript::Error::Runtime("moveUp requires 1 arguments".to_string()));
        }

        let mut camera: SerializableCamera = serde_json::from_value(args[0].clone())
            .map_err(|e| rustyscript::Error::Runtime(format!("Invalid camera: {}", e)))?;

        let up = camera.up.normalize();
        camera.eye += up * camera.speed;
        camera.target += up * camera.speed;

        serde_json::to_value(camera)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Camera: {}", e)))
    }

    /// Moves the camera down. 
    /// 
    /// # Parameters
    /// * args[0] - Self as a Camera. The value of the speed can be adjusted
    pub fn move_down(args: &[serde_json::Value]) -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 1 {
            return Err(rustyscript::Error::Runtime("moveDown requires 1 arguments".to_string()));
        }

        let mut camera: SerializableCamera = serde_json::from_value(args[0].clone())
            .map_err(|e| rustyscript::Error::Runtime(format!("Invalid camera: {}", e)))?;

        let up = camera.up.normalize();
        camera.eye -= up * camera.speed;
        camera.target -= up * camera.speed;

        serde_json::to_value(camera)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Camera: {}", e)))
    }

    /// Sets the tracking of the camera to the mouse delta
    /// 
    /// # Parameters
    /// * args[0] - Self as a Camera
    /// * args[1] - The dx value of the mouse position as a number
    /// * args[2] - The dy value of the mouse position as a number
    pub fn track_mouse_delta(args: &[serde_json::Value]) -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 3 {
            return Err(rustyscript::Error::Runtime("trackMouseDelta requires 3 arguments (camera, dx, dy)".to_string()));
        }

        let mut camera: SerializableCamera = serde_json::from_value(args[0].clone())
            .map_err(|e| rustyscript::Error::Runtime(format!("Invalid camera: {}", e)))?;

        let dx = args[1].as_f64().ok_or_else(|| {
            rustyscript::Error::Runtime("dx must be a number".to_string())
        })?;

        let dy = args[2].as_f64().ok_or_else(|| {
            rustyscript::Error::Runtime("dy must be a number".to_string())
        })?;

        camera.yaw += dx * camera.sensitivity;
        camera.pitch += dy * camera.sensitivity;

        camera.pitch = camera.pitch.clamp(-89.0_f64.to_radians(), 89.0_f64.to_radians());

        let direction = DVec3::new(
            camera.yaw.cos() * camera.pitch.cos(),
            camera.pitch.sin(),
            camera.yaw.sin() * camera.pitch.cos(),
        );
        camera.target = camera.eye + direction;

        serde_json::to_value(camera)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Camera: {}", e)))
    }

    pub fn switch_to_camera_by_label(args: &[serde_json::Value]) -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 1 {
            return Err(rustyscript::Error::Runtime("switchToCameraByLabel requires 1 argument (label)".to_string()));
        }

        let label = args[0].as_str().ok_or_else(|| {
            rustyscript::Error::Runtime("Label must be a string".to_string())
        })?;

        let action = CameraAction {
            action: "switch_camera".to_string(),
            label: Some(label.to_string()),
            camera_type: None,
            camera_data: None,
            property: None,
            value: None,
        };

        serde_json::to_value(action)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize camera action: {}", e)))
    }

    /// Get the label of the currently active camera
    pub fn get_active_camera_label(_args: &[serde_json::Value]) -> Result<serde_json::Value, rustyscript::Error> {
        let action = CameraAction {
            action: "get_active_camera".to_string(),
            label: None,
            camera_type: None,
            camera_data: None,
            property: None,
            value: None,
        };

        serde_json::to_value(action)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize camera action: {}", e)))
    }

    /// Get all camera labels in the world
    pub fn get_all_camera_labels(_args: &[serde_json::Value]) -> Result<serde_json::Value, rustyscript::Error> {
        let action = CameraAction {
            action: "get_all_cameras".to_string(),
            label: None,
            camera_type: None,
            camera_data: None,
            property: None,
            value: None,
        };

        serde_json::to_value(action)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize camera action: {}", e)))
    }

    /// Get cameras by type
    pub fn get_cameras_by_type(args: &[serde_json::Value]) -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 1 {
            return Err(rustyscript::Error::Runtime("getCamerasByType requires 1 argument (camera_type)".to_string()));
        }

        let camera_type = args[0].as_str().ok_or_else(|| {
            rustyscript::Error::Runtime("Camera type must be a string".to_string())
        })?;

        // Validate camera type
        match camera_type {
            "Normal" | "Debug" | "Player" => {
                let action = CameraAction {
                    action: "get_cameras_by_type".to_string(),
                    label: None,
                    camera_type: Some(camera_type.to_string()),
                    camera_data: None,
                    property: None,
                    value: None,
                };

                serde_json::to_value(action)
                    .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize camera action: {}", e)))
            }
            _ => Err(rustyscript::Error::Runtime("Invalid camera type. Use 'Normal', 'Debug', or 'Player'".to_string()))
        }
    }

    /// Manipulate a specific camera by label without switching to it
    pub fn manipulate_camera_by_label(args: &[serde_json::Value]) -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 3 {
            return Err(rustyscript::Error::Runtime("manipulateCameraByLabel requires 3 arguments (label, property, value)".to_string()));
        }

        let label = args[0].as_str().ok_or_else(|| {
            rustyscript::Error::Runtime("Label must be a string".to_string())
        })?;

        let property = args[1].as_str().ok_or_else(|| {
            rustyscript::Error::Runtime("Property must be a string".to_string())
        })?;

        // Validate property
        match property {
            "position" | "target" | "speed" | "sensitivity" | "fov" | "yaw" | "pitch" => {
                let action = CameraAction {
                    action: "manipulate_camera".to_string(),
                    label: Some(label.to_string()),
                    camera_type: None,
                    camera_data: None,
                    property: Some(property.to_string()),
                    value: Some(args[2].clone()),
                };

                serde_json::to_value(action)
                    .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize camera action: {}", e)))
            }
            _ => Err(rustyscript::Error::Runtime("Invalid property. Use 'position', 'target', 'speed', 'sensitivity', 'fov', 'yaw', or 'pitch'".to_string()))
        }
    }

    /// Get a camera's data by label
    pub fn get_camera_by_label(args: &[serde_json::Value]) -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 1 {
            return Err(rustyscript::Error::Runtime("getCameraByLabel requires 1 argument (label)".to_string()));
        }

        let label = args[0].as_str().ok_or_else(|| {
            rustyscript::Error::Runtime("Label must be a string".to_string())
        })?;

        let action = CameraAction {
            action: "get_camera_by_label".to_string(),
            label: Some(label.to_string()),
            camera_type: None,
            camera_data: None,
            property: None,
            value: None,
        };

        serde_json::to_value(action)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize camera action: {}", e)))
    }

    /// Set a camera's complete data by label
    pub fn set_camera_by_label(args: &[serde_json::Value]) -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 2 {
            return Err(rustyscript::Error::Runtime("setCameraByLabel requires 2 arguments (label, camera_data)".to_string()));
        }

        let label = args[0].as_str().ok_or_else(|| {
            rustyscript::Error::Runtime("Label must be a string".to_string())
        })?;

        let camera_data: SerializableCamera = serde_json::from_value(args[1].clone())
            .map_err(|e| rustyscript::Error::Runtime(format!("Invalid camera data: {}", e)))?;

        let action = CameraAction {
            action: "set_camera_by_label".to_string(),
            label: Some(label.to_string()),
            camera_type: None,
            camera_data: Some(camera_data),
            property: None,
            value: None,
        };

        serde_json::to_value(action)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize camera action: {}", e)))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraAction {
    pub action: String,
    pub label: Option<String>,
    pub camera_type: Option<String>,
    pub camera_data: Option<SerializableCamera>,
    pub property: Option<String>,
    pub value: Option<serde_json::Value>,
}