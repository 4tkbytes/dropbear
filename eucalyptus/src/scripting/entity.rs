use rhai::{Dynamic, Engine};

use crate::states::{ModelProperties, PropertyValue};

pub fn register_model_props_module(engine: &mut Engine) {
    engine.register_type_with_name::<ModelProperties>("Properties");

    engine.register_fn("get_property", |props: &mut ModelProperties, key: &str| -> Dynamic {
        match props.get_property(key) {
            Some(PropertyValue::String(s)) => Dynamic::from(s.clone()),
            Some(PropertyValue::Int(i)) => Dynamic::from(*i),
            Some(PropertyValue::Float(f)) => Dynamic::from(*f),
            Some(PropertyValue::Bool(b)) => Dynamic::from(*b),
            Some(PropertyValue::Vec3(v)) => Dynamic::from([v[0] as f64, v[1] as f64, v[2] as f64]),
            None => Dynamic::UNIT,
        }
    });

    engine.register_fn("set_property", |props: &mut ModelProperties, key: &str, value: String| {
        props.set_property(key.to_string(), PropertyValue::String(value));
    });

    engine.register_fn("set_property", |props: &mut ModelProperties, key: &str, value: i64| {
        props.set_property(key.to_string(), PropertyValue::Int(value));
    });

    engine.register_fn("set_property", |props: &mut ModelProperties, key: &str, value: f64| {
        props.set_property(key.to_string(), PropertyValue::Float(value));
    });

    engine.register_fn("set_property", |props: &mut ModelProperties, key: &str, value: bool| {
        props.set_property(key.to_string(), PropertyValue::Bool(value));
    });

    engine.register_fn("set_property", |props: &mut ModelProperties, key: &str, value: rhai::Array| {
        if value.len() == 3 {
            if let (Ok(x), Ok(y), Ok(z)) = (
                value[0].as_float(),
                value[1].as_float(), 
                value[2].as_float()
            ) {
                props.set_property(key.to_string(), PropertyValue::Vec3([x as f32, y as f32, z as f32]));
            }
        }
    });

    engine.register_fn("get_string", |props: &mut ModelProperties, key: &str| -> String {
        match props.get_property(key) {
            Some(PropertyValue::String(s)) => s.clone(),
            _ => String::new(),
        }
    });

    engine.register_fn("get_int", |props: &mut ModelProperties, key: &str| -> i64 {
        match props.get_property(key) {
            Some(PropertyValue::Int(i)) => *i,
            _ => 0,
        }
    });

    engine.register_fn("get_float", |props: &mut ModelProperties, key: &str| -> f64 {
        match props.get_property(key) {
            Some(PropertyValue::Float(f)) => *f,
            _ => 0.0,
        }
    });

    engine.register_fn("get_bool", |props: &mut ModelProperties, key: &str| -> bool {
        match props.get_property(key) {
            Some(PropertyValue::Bool(b)) => *b,
            _ => false,
        }
    });

    engine.register_fn("get_vec3", |props: &mut ModelProperties, key: &str| -> rhai::Array {
        match props.get_property(key) {
            Some(PropertyValue::Vec3(v)) => {
                vec![Dynamic::from(v[0] as f64), Dynamic::from(v[1] as f64), Dynamic::from(v[2] as f64)]
            },
            _ => vec![Dynamic::from(0.0), Dynamic::from(0.0), Dynamic::from(0.0)],
        }
    });

    engine.register_fn("has_property", |props: &mut ModelProperties, key: &str| -> bool {
        props.get_property(key).is_some()
    });

    log::info!("[Script] Initialised entity custom properties module");
}