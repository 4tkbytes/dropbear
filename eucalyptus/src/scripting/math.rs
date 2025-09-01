use dropbear_engine::entity::Transform;
use glam::{DQuat, DVec3};
use rustyscript::{serde_json, Runtime};

pub fn register_math_functions(runtime: &mut Runtime) -> anyhow::Result<()> {
    
    runtime.register_function("createTransform", |_args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
        let transform = Transform::new();
        serde_json::to_value(transform)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Transform: {}", e)))
    })?;

    runtime.register_function("transformTranslate", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 2 {
            return Err(rustyscript::Error::Runtime("transformTranslate requires 2 arguments".to_string()));
        }
        
        let mut transform: Transform = serde_json::from_value(args[0].clone())
            .map_err(|e| rustyscript::Error::Runtime(format!("Invalid transform: {}", e)))?;
        
        let translation = if let Some(array) = args[1].as_array() {
            if array.len() != 3 {
                return Err(rustyscript::Error::Runtime("Translation array must have 3 elements".to_string()));
            }
            DVec3::new(
                array[0].as_f64().unwrap_or(0.0),
                array[1].as_f64().unwrap_or(0.0),
                array[2].as_f64().unwrap_or(0.0),
            )
        } else {
            return Err(rustyscript::Error::Runtime("Translation must be an array".to_string()));
        };
        
        transform.position += translation;
        serde_json::to_value(transform)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Transform: {}", e)))
    })?;

    runtime.register_function("transformRotateX", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 2 {
            return Err(rustyscript::Error::Runtime("transformRotateX requires 2 arguments".to_string()));
        }
        
        let mut transform: Transform = serde_json::from_value(args[0].clone())
            .map_err(|e| rustyscript::Error::Runtime(format!("Invalid transform: {}", e)))?;
        
        let angle = args[1].as_f64().unwrap_or(0.0);
        let rotation = DQuat::from_rotation_x(angle);
        transform.rotation = rotation * transform.rotation;
        
        serde_json::to_value(transform)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Transform: {}", e)))
    })?;

    runtime.register_function("transformRotateY", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 2 {
            return Err(rustyscript::Error::Runtime("transformRotateY requires 2 arguments".to_string()));
        }
        
        let mut transform: Transform = serde_json::from_value(args[0].clone())
            .map_err(|e| rustyscript::Error::Runtime(format!("Invalid transform: {}", e)))?;
        
        let angle = args[1].as_f64().unwrap_or(0.0);
        let rotation = DQuat::from_rotation_y(angle);
        transform.rotation = rotation * transform.rotation;
        
        serde_json::to_value(transform)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Transform: {}", e)))
    })?;

    runtime.register_function("transformRotateZ", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 2 {
            return Err(rustyscript::Error::Runtime("transformRotateZ requires 2 arguments".to_string()));
        }
        
        let mut transform: Transform = serde_json::from_value(args[0].clone())
            .map_err(|e| rustyscript::Error::Runtime(format!("Invalid transform: {}", e)))?;
        
        let angle = args[1].as_f64().unwrap_or(0.0);
        let rotation = DQuat::from_rotation_z(angle);
        transform.rotation = rotation * transform.rotation;
        
        serde_json::to_value(transform)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Transform: {}", e)))
    })?;

    runtime.register_function("transformScale", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 2 {
            return Err(rustyscript::Error::Runtime("transformScale requires 2 arguments".to_string()));
        }
        
        let mut transform: Transform = serde_json::from_value(args[0].clone())
            .map_err(|e| rustyscript::Error::Runtime(format!("Invalid transform: {}", e)))?;
        
        let scale = if let Some(num) = args[1].as_f64() {
            DVec3::splat(num)
        } else if let Some(array) = args[1].as_array() {
            if array.len() != 3 {
                return Err(rustyscript::Error::Runtime("Scale array must have 3 elements".to_string()));
            }
            DVec3::new(
                array[0].as_f64().unwrap_or(1.0),
                array[1].as_f64().unwrap_or(1.0),
                array[2].as_f64().unwrap_or(1.0),
            )
        } else {
            return Err(rustyscript::Error::Runtime("Scale must be a number or array".to_string()));
        };
        
        transform.scale *= scale;
        serde_json::to_value(transform)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Transform: {}", e)))
    })?;

    // this shouldn't be here as there is no need for a matrix...
    runtime.register_function("transformMatrix", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 1 {
            return Err(rustyscript::Error::Runtime("transformMatrix requires 1 argument".to_string()));
        }
        
        let transform: Transform = serde_json::from_value(args[0].clone())
            .map_err(|e| rustyscript::Error::Runtime(format!("Invalid transform: {}", e)))?;
        
        let matrix = transform.matrix();
        serde_json::to_value(matrix)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize matrix: {}", e)))
    })?;

    runtime.register_function("createVec3", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
        let x = args.get(0).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let y = args.get(1).and_then(|v| v.as_f64()).unwrap_or(0.0);
        let z = args.get(2).and_then(|v| v.as_f64()).unwrap_or(0.0);
        
        let vec3 = DVec3::new(x, y, z);
        serde_json::to_value(vec3)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Vec3: {}", e)))
    })?;

    runtime.register_function("createQuatIdentity", |_args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
        let quat = DQuat::IDENTITY;
        serde_json::to_value(quat)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Quaternion: {}", e)))
    })?;

    runtime.register_function("createQuatFromEuler", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
        if args.len() != 3 {
            return Err(rustyscript::Error::Runtime("createQuatFromEuler requires 3 arguments".to_string()));
        }
        
        let x = args[0].as_f64().unwrap_or(0.0);
        let y = args[1].as_f64().unwrap_or(0.0);
        let z = args[2].as_f64().unwrap_or(0.0);
        
        let quat = DQuat::from_euler(glam::EulerRot::XYZ, x, y, z);
        serde_json::to_value(quat)
            .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize Quaternion: {}", e)))
    })?;

    log::info!("[Script] Initialised math module");
    Ok(())
}