use jni::JNIEnv;
use jni::objects::JFloatArray;
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