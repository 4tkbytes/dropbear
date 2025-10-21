use glam::Vec3;
use jni::JNIEnv;
use jni::objects::{JFloatArray, JObject, JValue};
use jni::sys::{jfloatArray, jint};

pub fn new_float_array(env: &mut JNIEnv, x: f32, y: f32) -> jfloatArray {
    let java_array: JFloatArray = match env.new_float_array(2) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[ERROR] Failed to create float array: {}", e);
            return std::ptr::null_mut();
        }
    };
    let elements: [f32; 2] = [x, y];
    match env.set_float_array_region(&java_array, 0, &elements) {
        Ok(()) => {
            java_array.into_raw()
        },
        Err(e) => {
            eprintln!("[ERROR] Error setting float array region: {}", e);
            env.throw_new("java/lang/RuntimeException", "Failed to set float array region").unwrap();
            std::ptr::null_mut()
        }
    }
}

const JAVA_MOUSE_BUTTON_LEFT: jint = 0;
const JAVA_MOUSE_BUTTON_RIGHT: jint = 1;
const JAVA_MOUSE_BUTTON_MIDDLE: jint = 2;
const JAVA_MOUSE_BUTTON_BACK: jint = 3;
const JAVA_MOUSE_BUTTON_FORWARD: jint = 4;

pub fn java_button_to_rust(button_code: jint) -> Option<winit::event::MouseButton> {
    match button_code {
        JAVA_MOUSE_BUTTON_LEFT => Some(winit::event::MouseButton::Left),
        JAVA_MOUSE_BUTTON_RIGHT => Some(winit::event::MouseButton::Right),
        JAVA_MOUSE_BUTTON_MIDDLE => Some(winit::event::MouseButton::Middle),
        JAVA_MOUSE_BUTTON_BACK => Some(winit::event::MouseButton::Back),
        JAVA_MOUSE_BUTTON_FORWARD => Some(winit::event::MouseButton::Forward),
        other if other >= 0 => Some(winit::event::MouseButton::Other(other as u16)), // Assuming Other uses the int directly
        _ => None,
    }
}

pub fn create_vector3<'a>(env: &mut JNIEnv<'a>, x: f64, y: f64, z: f64) -> anyhow::Result<JObject<'a>> {
    let vector3_class = env.find_class("com/dropbear/math/Vector3")?;

    let x_obj = env.call_static_method(
        "java/lang/Double",
        "valueOf",
        "(D)Ljava/lang/Double;",
        &[JValue::Double(x)]
    )?.l()?;

    let y_obj = env.call_static_method(
        "java/lang/Double",
        "valueOf",
        "(D)Ljava/lang/Double;",
        &[JValue::Double(y)]
    )?.l()?;

    let z_obj = env.call_static_method(
        "java/lang/Double",
        "valueOf",
        "(D)Ljava/lang/Double;",
        &[JValue::Double(z)]
    )?.l()?;

    let vector3 = env.new_object(
        vector3_class,
        "(Ljava/lang/Number;Ljava/lang/Number;Ljava/lang/Number;)V",
        &[
            JValue::Object(&x_obj),
            JValue::Object(&y_obj),
            JValue::Object(&z_obj),
        ]
    )?;

    Ok(vector3)
}

pub fn extract_vector3(env: &mut JNIEnv, vector_obj: &JObject) -> Option<Vec3> {
    let x_obj = env.get_field(vector_obj, "x", "Ljava/lang/Number;").ok()?.l().ok()?;
    let y_obj = env.get_field(vector_obj, "y", "Ljava/lang/Number;").ok()?.l().ok()?;
    let z_obj = env.get_field(vector_obj, "z", "Ljava/lang/Number;").ok()?.l().ok()?;

    let x = env.call_method(&x_obj, "doubleValue", "()D", &[]).ok()?.d().ok()?;
    let y = env.call_method(&y_obj, "doubleValue", "()D", &[]).ok()?.d().ok()?;
    let z = env.call_method(&z_obj, "doubleValue", "()D", &[]).ok()?.d().ok()?;

    Some(Vec3::new(x as f32, y as f32, z as f32))
}