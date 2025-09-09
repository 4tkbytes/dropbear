use crate::{scripting::ScriptableModule, states::{ModelProperties, PropertyValue}};
use rustyscript::{serde_json, Runtime};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct SerializableModelProperties {
    pub properties: std::collections::HashMap<String, serde_json::Value>,
}

impl From<&ModelProperties> for SerializableModelProperties {
    fn from(props: &ModelProperties) -> Self {
        let mut properties = std::collections::HashMap::new();
        
        for (key, value) in props.custom_properties.iter() {
            let json_value = match value {
                PropertyValue::String(s) => serde_json::Value::String(s.clone()),
                PropertyValue::Int(i) => serde_json::Value::Number(serde_json::Number::from(*i)),
                PropertyValue::Float(f) => serde_json::Value::Number(
                    serde_json::Number::from_f64(*f as f64).unwrap_or(serde_json::Number::from(0))
                ),
                PropertyValue::Bool(b) => serde_json::Value::Bool(*b),
                PropertyValue::Vec3(v) => serde_json::Value::Array(vec![
                    serde_json::Value::Number(serde_json::Number::from_f64(v[0] as f64).unwrap()),
                    serde_json::Value::Number(serde_json::Number::from_f64(v[1] as f64).unwrap()),
                    serde_json::Value::Number(serde_json::Number::from_f64(v[2] as f64).unwrap()),
                ]),
            };
            properties.insert(key.clone(), json_value);
        }
        
        Self { properties }
    }
}

impl SerializableModelProperties {
    pub fn to_model_properties(&self) -> ModelProperties {
        let mut props = ModelProperties::default();
        
        for (key, value) in &self.properties {
            let property_value = match value {
                serde_json::Value::String(s) => PropertyValue::String(s.clone()),
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        PropertyValue::Int(i)
                    } else if let Some(f) = n.as_f64() {
                        PropertyValue::Float(f)
                    } else {
                        continue;
                    }
                },
                serde_json::Value::Bool(b) => PropertyValue::Bool(*b),
                serde_json::Value::Array(arr) => {
                    if arr.len() == 3 {
                        if let (Some(x), Some(y), Some(z)) = (
                            arr[0].as_f64(),
                            arr[1].as_f64(),
                            arr[2].as_f64(),
                        ) {
                            PropertyValue::Vec3([x as f32, y as f32, z as f32])
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                },
                _ => continue,
            };
            props.set_property(key.clone(), property_value);
        }
        
        props
    }
}

impl ScriptableModule for ModelProperties {
    fn register(runtime: &mut Runtime) -> anyhow::Result<()> {
        runtime.register_function("getProperty", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 2 {
                return Err(rustyscript::Error::Runtime("getProperty requires 2 arguments (properties, key)".to_string()));
            }
            
            let props: SerializableModelProperties = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid properties: {}", e)))?;
            
            let key = args[1].as_str()
                .ok_or_else(|| rustyscript::Error::Runtime("Key must be a string".to_string()))?;
            
            let value = props.properties.get(key)
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            
            Ok(value)
        })?;

        // Set property (string)
        runtime.register_function("setPropertyString", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 3 {
                return Err(rustyscript::Error::Runtime("setPropertyString requires 3 arguments (properties, key, value)".to_string()));
            }
            
            let mut props: SerializableModelProperties = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid properties: {}", e)))?;
            
            let key = args[1].as_str()
                .ok_or_else(|| rustyscript::Error::Runtime("Key must be a string".to_string()))?;
            
            let value = args[2].as_str()
                .ok_or_else(|| rustyscript::Error::Runtime("Value must be a string".to_string()))?;
            
            props.properties.insert(key.to_string(), serde_json::Value::String(value.to_string()));
            
            serde_json::to_value(props)
                .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize properties: {}", e)))
        })?;

        // Set property (int)
        runtime.register_function("setPropertyInt", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 3 {
                return Err(rustyscript::Error::Runtime("setPropertyInt requires 3 arguments (properties, key, value)".to_string()));
            }
            
            let mut props: SerializableModelProperties = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid properties: {}", e)))?;
            
            let key = args[1].as_str()
                .ok_or_else(|| rustyscript::Error::Runtime("Key must be a string".to_string()))?;
            
            let value = args[2].as_i64()
                .ok_or_else(|| rustyscript::Error::Runtime("Value must be an integer".to_string()))?;
            
            props.properties.insert(key.to_string(), serde_json::Value::Number(serde_json::Number::from(value)));
            
            serde_json::to_value(props)
                .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize properties: {}", e)))
        })?;

        // Set property (float)
        runtime.register_function("setPropertyFloat", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 3 {
                return Err(rustyscript::Error::Runtime("setPropertyFloat requires 3 arguments (properties, key, value)".to_string()));
            }
            
            let mut props: SerializableModelProperties = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid properties: {}", e)))?;
            
            let key = args[1].as_str()
                .ok_or_else(|| rustyscript::Error::Runtime("Key must be a string".to_string()))?;
            
            let value = args[2].as_f64()
                .ok_or_else(|| rustyscript::Error::Runtime("Value must be a number".to_string()))?;
            
            props.properties.insert(key.to_string(), serde_json::Value::Number(
                serde_json::Number::from_f64(value).unwrap_or(serde_json::Number::from(0))
            ));
            
            serde_json::to_value(props)
                .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize properties: {}", e)))
        })?;

        // Set property (bool)
        runtime.register_function("setPropertyBool", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 3 {
                return Err(rustyscript::Error::Runtime("setPropertyBool requires 3 arguments (properties, key, value)".to_string()));
            }
            
            let mut props: SerializableModelProperties = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid properties: {}", e)))?;
            
            let key = args[1].as_str()
                .ok_or_else(|| rustyscript::Error::Runtime("Key must be a string".to_string()))?;
            
            let value = args[2].as_bool()
                .ok_or_else(|| rustyscript::Error::Runtime("Value must be a boolean".to_string()))?;
            
            props.properties.insert(key.to_string(), serde_json::Value::Bool(value));
            
            serde_json::to_value(props)
                .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize properties: {}", e)))
        })?;

        // Set property (Vec3)
        runtime.register_function("setPropertyVec3", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 3 {
                return Err(rustyscript::Error::Runtime("setPropertyVec3 requires 3 arguments (properties, key, value)".to_string()));
            }
            
            let mut props: SerializableModelProperties = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid properties: {}", e)))?;
            
            let key = args[1].as_str()
                .ok_or_else(|| rustyscript::Error::Runtime("Key must be a string".to_string()))?;
            
            let value = args[2].as_array()
                .ok_or_else(|| rustyscript::Error::Runtime("Value must be an array".to_string()))?;
            
            if value.len() != 3 {
                return Err(rustyscript::Error::Runtime("Vec3 array must have exactly 3 elements".to_string()));
            }
            
            let x = value[0].as_f64().ok_or_else(|| rustyscript::Error::Runtime("Vec3 elements must be numbers".to_string()))?;
            let y = value[1].as_f64().ok_or_else(|| rustyscript::Error::Runtime("Vec3 elements must be numbers".to_string()))?;
            let z = value[2].as_f64().ok_or_else(|| rustyscript::Error::Runtime("Vec3 elements must be numbers".to_string()))?;
            
            let vec3_array = serde_json::Value::Array(vec![
                serde_json::Value::Number(serde_json::Number::from_f64(x).unwrap()),
                serde_json::Value::Number(serde_json::Number::from_f64(y).unwrap()),
                serde_json::Value::Number(serde_json::Number::from_f64(z).unwrap()),
            ]);
            
            props.properties.insert(key.to_string(), vec3_array);
            
            serde_json::to_value(props)
                .map_err(|e| rustyscript::Error::Runtime(format!("Failed to serialize properties: {}", e)))
        })?;

        // Typed getters
        runtime.register_function("getString", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 2 {
                return Err(rustyscript::Error::Runtime("getString requires 2 arguments (properties, key)".to_string()));
            }
            
            let props: SerializableModelProperties = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid properties: {}", e)))?;
            
            let key = args[1].as_str()
                .ok_or_else(|| rustyscript::Error::Runtime("Key must be a string".to_string()))?;
            
            let value = props.properties.get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            
            Ok(serde_json::Value::String(value))
        })?;

        runtime.register_function("getInt", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 2 {
                return Err(rustyscript::Error::Runtime("getInt requires 2 arguments (properties, key)".to_string()));
            }
            
            let props: SerializableModelProperties = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid properties: {}", e)))?;
            
            let key = args[1].as_str()
                .ok_or_else(|| rustyscript::Error::Runtime("Key must be a string".to_string()))?;
            
            let value = props.properties.get(key)
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            
            Ok(serde_json::Value::Number(serde_json::Number::from(value)))
        })?;

        runtime.register_function("getFloat", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 2 {
                return Err(rustyscript::Error::Runtime("getFloat requires 2 arguments (properties, key)".to_string()));
            }
            
            let props: SerializableModelProperties = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid properties: {}", e)))?;
            
            let key = args[1].as_str()
                .ok_or_else(|| rustyscript::Error::Runtime("Key must be a string".to_string()))?;
            
            let value = props.properties.get(key)
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            
            Ok(serde_json::Value::Number(serde_json::Number::from_f64(value).unwrap()))
        })?;

        runtime.register_function("getBool", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 2 {
                return Err(rustyscript::Error::Runtime("getBool requires 2 arguments (properties, key)".to_string()));
            }
            
            let props: SerializableModelProperties = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid properties: {}", e)))?;
            
            let key = args[1].as_str()
                .ok_or_else(|| rustyscript::Error::Runtime("Key must be a string".to_string()))?;
            
            let value = props.properties.get(key)
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            
            Ok(serde_json::Value::Bool(value))
        })?;

        runtime.register_function("getVec3", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 2 {
                return Err(rustyscript::Error::Runtime("getVec3 requires 2 arguments (properties, key)".to_string()));
            }
            
            let props: SerializableModelProperties = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid properties: {}", e)))?;
            
            let key = args[1].as_str()
                .ok_or_else(|| rustyscript::Error::Runtime("Key must be a string".to_string()))?;
            
            let value = props.properties.get(key)
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_else(|| vec![
                    serde_json::Value::Number(serde_json::Number::from(0)),
                    serde_json::Value::Number(serde_json::Number::from(0)),
                    serde_json::Value::Number(serde_json::Number::from(0)),
                ]);
            
            Ok(serde_json::Value::Array(value))
        })?;

        runtime.register_function("hasProperty", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 2 {
                return Err(rustyscript::Error::Runtime("hasProperty requires 2 arguments (properties, key)".to_string()));
            }
            
            let props: SerializableModelProperties = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid properties: {}", e)))?;
            
            let key = args[1].as_str()
                .ok_or_else(|| rustyscript::Error::Runtime("Key must be a string".to_string()))?;
            
            let has_property = props.properties.contains_key(key);
            Ok(serde_json::Value::Bool(has_property))
        })?;

        log::info!("[Script] Initialised entity custom properties module");
        Ok(())
    }
}