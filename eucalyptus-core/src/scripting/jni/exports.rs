use crate::camera::{CameraComponent, CameraType};
use crate::ptr::{AssetRegistryPtr, GraphicsPtr, InputStatePtr, WorldPtr};
use crate::scripting::jni::error::{
    clear_last_error_message, get_last_error_message, get_last_error_message_ptr,
    set_last_error_message,
};
use crate::scripting::jni::utils::{
    create_vector3, extract_vector3, java_button_to_rust, new_float_array,
};
use crate::states::{Label, ModelProperties, Value};
use crate::utils::keycode_from_ordinal;
use crate::window::{GraphicsCommand, WindowCommand};
use crate::{convert_jlong_to_entity, convert_jstring, convert_ptr};
use dropbear_engine::asset::PointerKind::Const;
use dropbear_engine::asset::{ASSET_REGISTRY, AssetHandle, AssetRegistry};
use dropbear_engine::camera::Camera;
use dropbear_engine::entity::{MeshRenderer, Transform};
use dropbear_engine::model::Model;
use dropbear_engine::utils::ResourceReference;
use glam::{DQuat, DVec3};
use hecs::World;
use jni::JNIEnv;
use jni::objects::{JClass, JObject, JPrimitiveArray, JString, JValue};
use jni::sys::{
    JNI_FALSE, jboolean, jclass, jdouble, jfloatArray, jint, jlong, jobject, jobjectArray, jstring,
};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// `JNIEXPORT jlong JNICALL Java_com_dropbear_ffi_JNINative_getEntity
///   (JNIEnv *, jclass, jlong, jstring);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getEntity(
    mut env: JNIEnv,
    _obj: JClass,
    world_handle: jlong,
    label: JString,
) -> jlong {
    let label_jni_result = env.get_string(&label);
    let label_str = match label_jni_result {
        Ok(java_string) => match java_string.to_str() {
            Ok(rust_str) => rust_str.to_string(),
            Err(e) => {
                println!(
                    "[Java_com_dropbear_ffi_JNINative_getEntity] [ERROR] Failed to convert Java string to Rust string: {}",
                    e
                );
                return -1;
            }
        },
        Err(e) => {
            println!(
                "[Java_com_dropbear_ffi_JNINative_getEntity] [ERROR] Failed to get string from JNI: {}",
                e
            );
            return -1;
        }
    };

    let world = world_handle as *mut World;

    if world.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_getEntity] [ERROR] World pointer is null");
        return -1;
    }

    let world = unsafe { &mut *world };

    for (id, entity_label) in world.query::<&Label>().iter() {
        if entity_label.as_str() == label_str {
            return id.to_bits().get() as jlong;
        }
    }
    0
}

/// `JNIEXPORT jobject JNICALL Java_com_dropbear_ffi_JNINative_getWorldTransform
///   (JNIEnv *, jclass, jlong, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getWorldTransform(
    mut env: JNIEnv,
    _class: jclass,
    world_handle: jlong,
    entity_id: jlong,
) -> JObject {
    let world = world_handle as *mut World;

    if world.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_getWorldTransform] [ERROR] World pointer is null"
        );
        return JObject::null();
    }

    let world = unsafe { &mut *world };

    let entity = convert_jlong_to_entity!(entity_id);

    if let Ok(mut q) = world.query_one::<(&WorldTransform, &LocalTransform)>(entity)
        && let Some((wt, lt)) = q.get()
    {
        let new_transform = *transform;

        let transform_class = match env.find_class("com/dropbear/math/Transform") {
            Ok(c) => c,
            Err(_) => return JObject::null(),
        };

        return match env.new_object(
            &transform_class,
            "(DDDDDDDDDD)V",
            &[
                new_transform.position.x.into(),
                new_transform.position.y.into(),
                new_transform.position.z.into(),
                new_transform.rotation.x.into(),
                new_transform.rotation.y.into(),
                new_transform.rotation.z.into(),
                new_transform.rotation.w.into(),
                new_transform.scale.x.into(),
                new_transform.scale.y.into(),
                new_transform.scale.z.into(),
            ],
        ) {
            Ok(java_transform) => java_transform,
            Err(_) => {
                println!(
                    "[Java_com_dropbear_ffi_JNINative_getWorldTransform] [ERROR] Failed to create Transform object"
                );
                JObject::null()
            }
        };
    }

    println!(
        "[Java_com_dropbear_ffi_JNINative_getWorldTransform] [ERROR] Failed to query for world transform value for entity: {}",
        entity_id
    );
    JObject::null()
}

/// `JNIEXPORT jobject JNICALL Java_com_dropbear_ffi_JNINative_getLocalTransform
///   (JNIEnv *, jclass, jlong, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getLocalTransform(
    mut env: JNIEnv,
    _class: jclass,
    world_handle: jlong,
    entity_id: jlong,
) -> JObject {
    let world = world_handle as *mut World;

    if world.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_getLocalTransform] [ERROR] World pointer is null"
        );
        return JObject::null();
    }

    let world = unsafe { &mut *world };

    let entity = convert_jlong_to_entity!(entity_id);

    if let Ok(mut q) = world.query_one::<(&WorldTransform, &LocalTransform)>(entity)
        && let Some((wt, lt)) = q.get()
    {
        let new_transform = *transform;

        let transform_class = match env.find_class("com/dropbear/math/Transform") {
            Ok(c) => c,
            Err(_) => return JObject::null(),
        };

        return match env.new_object(
            &transform_class,
            "(DDDDDDDDDD)V",
            &[
                new_transform.position.x.into(),
                new_transform.position.y.into(),
                new_transform.position.z.into(),
                new_transform.rotation.x.into(),
                new_transform.rotation.y.into(),
                new_transform.rotation.z.into(),
                new_transform.rotation.w.into(),
                new_transform.scale.x.into(),
                new_transform.scale.y.into(),
                new_transform.scale.z.into(),
            ],
        ) {
            Ok(java_transform) => java_transform,
            Err(_) => {
                println!(
                    "[Java_com_dropbear_ffi_JNINative_getLocalTransform] [ERROR] Failed to create Transform object"
                );
                JObject::null()
            }
        };
    }

    println!(
        "[Java_com_dropbear_ffi_JNINative_getLocalTransform] [ERROR] Failed to query for local transform value for entity: {}",
        entity_id
    );
    JObject::null()
}

/// `JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_commitWorldTransform
///   (JNIEnv *, jclass, jlong, jlong, jobject);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_commitWorldTransform(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    transform_obj: JObject,
) {
    let world = world_handle as *mut World;

    if world.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_commitWorldTransform] [ERROR] World pointer is null"
        );
        return;
    }

    let world = unsafe { &mut *world };
    let entity = convert_jlong_to_entity!(entity_id);

    let get_number_field = |env: &mut JNIEnv, obj: &JObject, field_name: &str| -> f64 {
        match env.get_field(obj, field_name, "Ljava/lang/Number;") {
            Ok(v) => match v.l() {
                Ok(num_obj) => {
                    match env.call_method(&num_obj, "doubleValue", "()D", &[]) {
                        Ok(result) => result.d().unwrap_or_else(|_| {
                            println!("[Java_com_dropbear_ffi_JNINative_commitWorldTransform] [ERROR] Failed to extract double from {}", field_name);
                            0.0
                        }),
                        Err(_) => {
                            println!("[Java_com_dropbear_ffi_JNINative_commitWorldTransform] [ERROR] Failed to call doubleValue on {}", field_name);
                            0.0
                        }
                    }
                }
                Err(_) => {
                    println!("[Java_com_dropbear_ffi_JNINative_commitWorldTransform] [ERROR] Failed to extract Number object for {}", field_name);
                    0.0
                }
            },
            Err(_) => {
                println!("[Java_com_dropbear_ffi_JNINative_commitWorldTransform] [ERROR] Failed to get field {}", field_name);
                0.0
            }
        }
    };

    let position_obj: JObject =
        match env.get_field(&transform_obj, "position", "Lcom/dropbear/math/Vector3;") {
            Ok(v) => v.l().unwrap_or_else(|_| JObject::null()),
            Err(_) => JObject::null(),
        };

    let rotation_obj: JObject =
        match env.get_field(&transform_obj, "rotation", "Lcom/dropbear/math/Quaternion;") {
            Ok(v) => v.l().unwrap_or_else(|_| JObject::null()),
            Err(_) => JObject::null(),
        };

    let scale_obj: JObject =
        match env.get_field(&transform_obj, "scale", "Lcom/dropbear/math/Vector3;") {
            Ok(v) => v.l().unwrap_or_else(|_| JObject::null()),
            Err(_) => JObject::null(),
        };

    if position_obj.is_null() || rotation_obj.is_null() || scale_obj.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_commitWorldTransform] [ERROR] Failed to extract position/rotation/scale objects"
        );
        return;
    }

    let px = get_number_field(&mut env, &position_obj, "x");
    let py = get_number_field(&mut env, &position_obj, "y");
    let pz = get_number_field(&mut env, &position_obj, "z");

    let rx = get_number_field(&mut env, &rotation_obj, "x");
    let ry = get_number_field(&mut env, &rotation_obj, "y");
    let rz = get_number_field(&mut env, &rotation_obj, "z");
    let rw = get_number_field(&mut env, &rotation_obj, "w");

    let sx = get_number_field(&mut env, &scale_obj, "x");
    let sy = get_number_field(&mut env, &scale_obj, "y");
    let sz = get_number_field(&mut env, &scale_obj, "z");

    let new_transform = WorldTransform::new(Transform {
        position: DVec3::new(px, py, pz),
        rotation: DQuat::from_xyzw(rx, ry, rz, rw),
        scale: DVec3::new(sx, sy, sz),
    });

    if let Ok(mut q) = world.query_one::<&mut WorldTransform>(entity) {
        if let Some(transform) = q.get() {
            *transform = new_transform;
        } else {
            println!(
                "[Java_com_dropbear_ffi_JNINative_commitWorldTransform] [ERROR] Failed to query for world transform component"
            );
        }
    } else {
        println!(
            "[Java_com_dropbear_ffi_JNINative_commitWorldTransform] [ERROR] Failed to query entity for world transform component"
        );
    }
}

/// `JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_commitLocalTransform
///   (JNIEnv *, jclass, jlong, jlong, jobject);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_commitLocalTransform(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    transform_obj: JObject,
) {
    let world = world_handle as *mut World;

    if world.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_commitLocalTransform] [ERROR] World pointer is null"
        );
        return;
    }

    let world = unsafe { &mut *world };
    let entity = convert_jlong_to_entity!(entity_id);

    let get_number_field = |env: &mut JNIEnv, obj: &JObject, field_name: &str| -> f64 {
        match env.get_field(obj, field_name, "Ljava/lang/Number;") {
            Ok(v) => match v.l() {
                Ok(num_obj) => {
                    match env.call_method(&num_obj, "doubleValue", "()D", &[]) {
                        Ok(result) => result.d().unwrap_or_else(|_| {
                            println!("[Java_com_dropbear_ffi_JNINative_commitLocalTransform] [ERROR] Failed to extract double from {}", field_name);
                            0.0
                        }),
                        Err(_) => {
                            println!("[Java_com_dropbear_ffi_JNINative_commitLocalTransform] [ERROR] Failed to call doubleValue on {}", field_name);
                            0.0
                        }
                    }
                }
                Err(_) => {
                    println!("[Java_com_dropbear_ffi_JNINative_commitLocalTransform] [ERROR] Failed to extract Number object for {}", field_name);
                    0.0
                }
            },
            Err(_) => {
                println!("[Java_com_dropbear_ffi_JNINative_commitLocalTransform] [ERROR] Failed to get field {}", field_name);
                0.0
            }
        }
    };

    let position_obj: JObject =
        match env.get_field(&transform_obj, "position", "Lcom/dropbear/math/Vector3;") {
            Ok(v) => v.l().unwrap_or_else(|_| JObject::null()),
            Err(_) => JObject::null(),
        };

    let rotation_obj: JObject =
        match env.get_field(&transform_obj, "rotation", "Lcom/dropbear/math/Quaternion;") {
            Ok(v) => v.l().unwrap_or_else(|_| JObject::null()),
            Err(_) => JObject::null(),
        };

    let scale_obj: JObject =
        match env.get_field(&transform_obj, "scale", "Lcom/dropbear/math/Vector3;") {
            Ok(v) => v.l().unwrap_or_else(|_| JObject::null()),
            Err(_) => JObject::null(),
        };

    if position_obj.is_null() || rotation_obj.is_null() || scale_obj.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_commitLocalTransform] [ERROR] Failed to extract position/rotation/scale objects"
        );
        return;
    }

    let px = get_number_field(&mut env, &position_obj, "x");
    let py = get_number_field(&mut env, &position_obj, "y");
    let pz = get_number_field(&mut env, &position_obj, "z");

    let rx = get_number_field(&mut env, &rotation_obj, "x");
    let ry = get_number_field(&mut env, &rotation_obj, "y");
    let rz = get_number_field(&mut env, &rotation_obj, "z");
    let rw = get_number_field(&mut env, &rotation_obj, "w");

    let sx = get_number_field(&mut env, &scale_obj, "x");
    let sy = get_number_field(&mut env, &scale_obj, "y");
    let sz = get_number_field(&mut env, &scale_obj, "z");

    let new_transform = LocalTransform::new(Transform {
        position: DVec3::new(px, py, pz),
        rotation: DQuat::from_xyzw(rx, ry, rz, rw),
        scale: DVec3::new(sx, sy, sz),
    });

    if let Ok(mut q) = world.query_one::<&mut LocalTransform>(entity) {
        if let Some(transform) = q.get() {
            *transform = new_transform;
        } else {
            println!(
                "[Java_com_dropbear_ffi_JNINative_commitLocalTransform] [ERROR] Failed to query for local transform component"
            );
        }
    } else {
        println!(
            "[Java_com_dropbear_ffi_JNINative_commitLocalTransform] [ERROR] Failed to query entity for local transform component"
        );
    }
}

/// `JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_printInputState
///   (JNIEnv *, jclass, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_printInputState(
    _env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
) {
    let input = input_handle as InputStatePtr;

    if input.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_printInputState] [ERROR] Input state pointer is null"
        );
        return;
    }

    let input = unsafe { &*input };
    println!("{:#?}", input);
}

/// `JNIEXPORT jboolean JNICALL Java_com_dropbear_ffi_JNINative_isKeyPressed
///   (JNIEnv *, jclass, jlong, jint);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_isKeyPressed(
    _env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
    key: jint,
) -> jboolean {
    let input = input_handle as InputStatePtr;
    if input.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_isKeyPressed] [ERROR] Input state pointer is null"
        );
        return false.into();
    }
    let input = unsafe { &*input };

    // println!("[Java_com_dropbear_ffi_JNINative_isKeyPressed] [DEBUG] Original code: {:?}", key);

    match keycode_from_ordinal(key) {
        Some(k) => {
            // println!("[Java_com_dropbear_ffi_JNINative_isKeyPressed] [DEBUG] Keycode: {:?}", k);
            if input.pressed_keys.contains(&k) {
                true.into()
            } else {
                false.into()
            }
        }
        None => {
            println!(
                "[Java_com_dropbear_ffi_JNINative_isKeyPressed] [WARN] Ordinal keycode is invalid"
            );
            false.into()
        }
    }
}

/// `JNIEXPORT jfloatArray JNICALL Java_com_dropbear_ffi_JNINative_getMousePosition
///   (JNIEnv *, jclass, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getMousePosition(
    mut env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
) -> jfloatArray {
    let input = input_handle as InputStatePtr;
    if input.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_getMousePosition] [ERROR] Input state pointer is null"
        );
        return new_float_array(&mut env, -1.0, -1.0);
    }

    let input = unsafe { &*input };

    new_float_array(&mut env, input.mouse_pos.0 as f32, input.mouse_pos.1 as f32)
}

/// `JNIEXPORT jboolean JNICALL Java_com_dropbear_ffi_JNINative_isMouseButtonPressed
///   (JNIEnv *, jclass, jlong, jint);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_isMouseButtonPressed(
    _env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
    button: jint,
) -> jboolean {
    let input_ptr = input_handle as InputStatePtr;

    if input_ptr.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_isMouseButtonPressed] [ERROR] Input state pointer is null"
        );
        return false as jboolean;
    }

    let input = unsafe { &*input_ptr };

    if let Some(rust_button) = java_button_to_rust(button) {
        let is_pressed = input.mouse_button.contains(&rust_button);
        is_pressed as jboolean
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_isMouseButtonPressed] [ERROR] Invalid button code: {}",
            button
        );
        false as jboolean
    }
}

/// `JNIEXPORT jfloatArray JNICALL Java_com_dropbear_ffi_JNINative_getMouseDelta
///   (JNIEnv *, jclass, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getMouseDelta(
    mut env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
) -> jfloatArray {
    let input = input_handle as InputStatePtr;
    if input.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_getMouseDelta] [ERROR] Input state pointer is null"
        );
        return new_float_array(&mut env, 0.0, 0.0);
    }

    let input = unsafe { &mut *input };

    if let Some(pos) = input.mouse_delta.take() {
        new_float_array(&mut env, pos.0 as f32, pos.1 as f32)
    } else {
        // println!("[Java_com_dropbear_ffi_JNINative_getMouseDelta] [WARN] input_state.mouse_delta returns \"(None)\". Returning (0.0, 0.0)");
        new_float_array(&mut env, 0.0, 0.0)
    }
}

/// `JNIEXPORT jboolean JNICALL Java_com_dropbear_ffi_JNINative_isCursorLocked
///   (JNIEnv *, jclass, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_isCursorLocked(
    _env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
) -> jboolean {
    let input = input_handle as InputStatePtr;
    if input.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_isCursorLocked] [ERROR] Input state pointer is null"
        );
        return false as jboolean;
    }

    let input = unsafe { &*input };

    input.is_cursor_locked as jboolean
}

/// `JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setCursorLocked
///   (JNIEnv *, jclass, jlong, jlong, jboolean);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_setCursorLocked(
    _env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
    graphics_handle: jlong,
    locked: jboolean,
) {
    let input = input_handle as InputStatePtr;

    if input.is_null() {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setCursorLocked] [ERROR] Input state pointer is null"
        );
        return;
    }

    let graphics = graphics_handle as GraphicsPtr;

    if graphics.is_null() {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setCursorLocked] [ERROR] Graphics pointer is null"
        );
        return;
    }

    let input = unsafe { &mut *input };
    let graphics = unsafe { &*graphics };

    let is_locked = locked != 0;

    if let Err(e) = graphics.send(GraphicsCommand::WindowCommand(WindowCommand::WindowGrab(
        is_locked,
    ))) {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setCursorLocked] [ERROR] Unable to send window command: {}",
            e
        );
        return;
    }

    input.is_cursor_locked = is_locked;
}

/// `JNIEXPORT jfloatArray JNICALL Java_com_dropbear_ffi_JNINative_getLastMousePos
///   (JNIEnv *, jclass, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getLastMousePos(
    mut env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
) -> jfloatArray {
    let input = input_handle as InputStatePtr;
    if input.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_getLastMousePos] [ERROR] Input state pointer is null"
        );
        return new_float_array(&mut env, 0.0, 0.0);
    }

    let input = unsafe { &*input };
    if let Some(pos) = input.last_mouse_pos {
        new_float_array(&mut env, pos.0 as f32, pos.1 as f32)
    } else {
        new_float_array(&mut env, 0.0, 0.0)
    }
}

/// `JNIEXPORT jstring JNICALL Java_com_dropbear_ffi_JNINative_getStringProperty
///   (JNIEnv *, jclass, jlong, jlong, jstring);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getStringProperty(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    property_name: JString,
) -> jstring {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_getStringProperty] [ERROR] World pointer is null"
        );
        return std::ptr::null_mut();
    }

    let world = unsafe { &mut *world };
    let entity = convert_jlong_to_entity!(entity_id);
    if let Ok(mut q) = world.query_one::<(&MeshRenderer, &ModelProperties)>(entity)
        && let Some((_, props)) = q.get()
    {
        let string = env.get_string(&property_name);
        let value: String = if let Ok(str) = string {
            let value = str.to_string_lossy();
            value.to_string()
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getStringProperty] [ERROR] Failed to get property name"
            );
            return std::ptr::null_mut();
        };
        let output = props.get_property(&value);
        if let Some(output) = output {
            match output {
                Value::String(val) => match env.new_string(val) {
                    Ok(string) => string.as_raw(),
                    Err(e) => {
                        eprintln!(
                            "[Java_com_dropbear_ffi_JNINative_getStringProperty] [ERROR] Failed to create string: {}",
                            e
                        );
                        std::ptr::null_mut()
                    }
                },
                _ => {
                    println!(
                        "[Java_com_dropbear_ffi_JNINative_getStringProperty] [WARN] Property is not a string"
                    );
                    std::ptr::null_mut()
                }
            }
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getStringProperty] [WARN] Property not found"
            );
            std::ptr::null_mut()
        }
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_getStringProperty] [ERROR] Failed to query entity for model properties"
        );
        std::ptr::null_mut()
    }
}

/// Fetches a [`jint`]/[`i32`] value from a key value.
///
/// If the value does not exist, it will return `650911`, a randomly generated number
/// that is extremely specific that no one would be sane enough to use this as a property.
///
/// `JNIEXPORT jint JNICALL Java_com_dropbear_ffi_JNINative_getIntProperty
///   (JNIEnv *, jclass, jlong, jlong, jstring);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getIntProperty(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    property_name: JString,
) -> jint {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!("[Java_com_dropbear_ffi_JNINative_getIntProperty] [ERROR] World pointer is null");
        return 650911;
    }

    let world = unsafe { &mut *world };
    let entity = convert_jlong_to_entity!(entity_id);
    if let Ok(mut q) = world.query_one::<(&MeshRenderer, &ModelProperties)>(entity)
        && let Some((_, props)) = q.get()
    {
        let string = env.get_string(&property_name);
        let value: String = if let Ok(str) = string {
            let value = str.to_string_lossy();
            value.to_string()
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getIntProperty] [ERROR] Failed to get property name"
            );
            return 650911;
        };
        let output = props.get_property(&value);
        if let Some(output) = output {
            match output {
                Value::Int(val) => *val as jint,
                _ => {
                    eprintln!(
                        "[Java_com_dropbear_ffi_JNINative_getIntProperty] [WARN] Property is not an int"
                    );
                    650911
                }
            }
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getIntProperty] [WARN] Property not found");
            650911
        }
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_getIntProperty] [ERROR] Failed to query entity for model properties"
        );
        650911
    }
}

/// Gets a [`jlong`]/[`i64`] property.
///
/// If the value doesn't exist, it will return this value: `6509112938`. This is a random number
/// from a generator I got, and it is such a specific number that no one would ever have this number
/// in one of their properties.
/// `JNIEXPORT jlong JNICALL Java_com_dropbear_ffi_JNINative_getLongProperty
///   (JNIEnv *, jclass, jlong, jlong, jstring);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getLongProperty(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    property_name: JString,
) -> jlong {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_getLongProperty] [ERROR] World pointer is null"
        );
        return 6509112938;
    }

    let world = unsafe { &mut *world };
    let entity = convert_jlong_to_entity!(entity_id);
    if let Ok(mut q) = world.query_one::<(&MeshRenderer, &ModelProperties)>(entity)
        && let Some((_, props)) = q.get()
    {
        let string = env.get_string(&property_name);
        let value: String = if let Ok(str) = string {
            let value = str.to_string_lossy();
            value.to_string()
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getLongProperty] [ERROR] Failed to get property name"
            );
            return 0;
        };
        let output = props.get_property(&value);
        if let Some(output) = output {
            match output {
                Value::Int(val) => *val as jlong,
                _ => {
                    eprintln!(
                        "[Java_com_dropbear_ffi_JNINative_getLongProperty] [WARN] Property is not a long"
                    );
                    6509112938
                }
            }
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getLongProperty] [WARN] Property not found"
            );
            6509112938
        }
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_getLongProperty] [ERROR] Failed to query entity for model properties"
        );
        6509112938
    }
}

/// `JNIEXPORT jdouble JNICALL Java_com_dropbear_ffi_JNINative_getFloatProperty
///   (JNIEnv *, jclass, jlong, jlong, jstring);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getFloatProperty(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    property_name: JString,
) -> jdouble {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_getFloatProperty] [ERROR] World pointer is null"
        );
        return f64::NAN;
    }

    let world = unsafe { &mut *world };
    let entity = convert_jlong_to_entity!(entity_id);
    if let Ok(mut q) = world.query_one::<(&MeshRenderer, &ModelProperties)>(entity)
        && let Some((_, props)) = q.get()
    {
        let string = env.get_string(&property_name);
        let value: String = if let Ok(str) = string {
            let value = str.to_string_lossy();
            value.to_string()
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getFloatProperty] [ERROR] Failed to get property name"
            );
            return f64::NAN;
        };
        let output = props.get_property(&value);
        if let Some(output) = output {
            match output {
                Value::Float(val) => *val as jdouble,
                _ => {
                    eprintln!(
                        "[Java_com_dropbear_ffi_JNINative_getFloatProperty] [WARN] Property is not a float"
                    );
                    f64::NAN
                }
            }
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getFloatProperty] [WARN] Property not found"
            );
            f64::NAN
        }
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_getFloatProperty] [ERROR] Failed to query entity for model properties"
        );
        f64::NAN
    }
}

/// `JNIEXPORT jboolean JNICALL Java_com_dropbear_ffi_JNINative_getBoolProperty
///   (JNIEnv *, jclass, jlong, jlong, jstring);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getBoolProperty(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    property_name: JString,
) -> jboolean {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_getBoolProperty] [ERROR] World pointer is null"
        );
        return 0;
    }

    let world = unsafe { &mut *world };
    let entity = convert_jlong_to_entity!(entity_id);
    if let Ok(mut q) = world.query_one::<(&MeshRenderer, &ModelProperties)>(entity)
        && let Some((_, props)) = q.get()
    {
        let string = env.get_string(&property_name);
        let value: String = if let Ok(str) = string {
            let value = str.to_string_lossy();
            value.to_string()
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getBoolProperty] [ERROR] Failed to get property name"
            );
            return 0;
        };
        let output = props.get_property(&value);
        if let Some(output) = output {
            match output {
                Value::Bool(val) => {
                    if *val {
                        1
                    } else {
                        0
                    }
                }
                _ => {
                    eprintln!(
                        "[Java_com_dropbear_ffi_JNINative_getBoolProperty] [WARN] Property is not a bool"
                    );
                    0
                }
            }
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getBoolProperty] [WARN] Property not found"
            );
            0
        }
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_getBoolProperty] [ERROR] Failed to query entity for model properties"
        );
        0
    }
}

/// `JNIEXPORT jfloatArray JNICALL Java_com_dropbear_ffi_JNINative_getVec3Property
///   (JNIEnv *, jclass, jlong, jlong, jstring);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getVec3Property(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    property_name: JString,
) -> jfloatArray {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_getVec3Property] [ERROR] World pointer is null"
        );
        return std::ptr::null_mut();
    }

    let world = unsafe { &mut *world };
    let entity = convert_jlong_to_entity!(entity_id);
    if let Ok(mut q) = world.query_one::<(&MeshRenderer, &ModelProperties)>(entity)
        && let Some((_, props)) = q.get()
    {
        let string = env.get_string(&property_name);
        let value: String = if let Ok(str) = string {
            let value = str.to_string_lossy();
            value.to_string()
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getVec3Property] [ERROR] Failed to get property name"
            );
            return std::ptr::null_mut();
        };
        let output = props.get_property(&value);
        if let Some(output) = output {
            match output {
                Value::Vec3([x, y, z]) => {
                    let arr = env.new_float_array(3);
                    if let Ok(arr) = arr {
                        let values = [*x, *y, *z];
                        if env.set_float_array_region(&arr, 0, &values).is_ok() {
                            arr.into_raw()
                        } else {
                            eprintln!(
                                "[Java_com_dropbear_ffi_JNINative_getVec3Property] [ERROR] Failed to set array region"
                            );
                            std::ptr::null_mut()
                        }
                    } else {
                        eprintln!(
                            "[Java_com_dropbear_ffi_JNINative_getVec3Property] [ERROR] Failed to create float array"
                        );
                        std::ptr::null_mut()
                    }
                }
                _ => {
                    eprintln!(
                        "[Java_com_dropbear_ffi_JNINative_getVec3Property] [WARN] Property is not a vec3"
                    );
                    std::ptr::null_mut()
                }
            }
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getVec3Property] [WARN] Property not found"
            );
            std::ptr::null_mut()
        }
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_getVec3Property] [ERROR] Failed to query entity for model properties"
        );
        std::ptr::null_mut()
    }
}

/// `JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setStringProperty
///   (JNIEnv *, jclass, jlong, jlong, jstring, jstring);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_setStringProperty(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    property_name: JString,
    value: JString,
) {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setStringProperty] [ERROR] World pointer is null"
        );
        return;
    }

    let world = unsafe { &mut *world };
    let entity = convert_jlong_to_entity!(entity_id);

    let key = env.get_string(&property_name);
    let key: String = if let Ok(str) = key {
        let value = str.to_string_lossy();
        value.to_string()
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setStringProperty] [ERROR] Failed to get property name"
        );
        return;
    };

    let string = env.get_string(&value);
    let value: String = if let Ok(str) = string {
        let value = str.to_string_lossy();
        value.to_string()
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setStringProperty] [ERROR] Failed to get property name"
        );
        return;
    };

    if let Ok((_, props)) = world.query_one_mut::<(&MeshRenderer, &mut ModelProperties)>(entity) {
        props.set_property(key, Value::String(value));
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setStringProperty] [ERROR] Failed to query entity for model properties"
        );
    }
}

/// `JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setIntProperty
///   (JNIEnv *, jclass, jlong, jlong, jstring, jint);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_setIntProperty(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    property_name: JString,
    value: jint,
) {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setIntProperty] [ERROR] World pointer is null");
        return;
    }

    let world = unsafe { &mut *world };
    let entity = convert_jlong_to_entity!(entity_id);

    let key = env.get_string(&property_name);
    let key: String = if let Ok(str) = key {
        let value = str.to_string_lossy();
        value.to_string()
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setIntProperty] [ERROR] Failed to get property name"
        );
        return;
    };

    if let Ok((_, props)) = world.query_one_mut::<(&MeshRenderer, &mut ModelProperties)>(entity) {
        props.set_property(key, Value::Int(value as i64));
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setIntProperty] [ERROR] Failed to query entity for model properties"
        );
    }
}

/// `JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setLongProperty
///   (JNIEnv *, jclass, jlong, jlong, jstring, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_setLongProperty(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    property_name: JString,
    value: jlong,
) {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setLongProperty] [ERROR] World pointer is null"
        );
        return;
    }

    let world = unsafe { &mut *world };
    let entity = convert_jlong_to_entity!(entity_id);

    let key = env.get_string(&property_name);
    let key: String = if let Ok(str) = key {
        let value = str.to_string_lossy();
        value.to_string()
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setLongProperty] [ERROR] Failed to get property name"
        );
        return;
    };

    if let Ok((_, props)) = world.query_one_mut::<(&MeshRenderer, &mut ModelProperties)>(entity) {
        props.set_property(key, Value::Int(value));
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setLongProperty] [ERROR] Failed to query entity for model properties"
        );
    }
}

/// `JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setFloatProperty
///   (JNIEnv *, jclass, jlong, jlong, jstring, jdouble);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_setFloatProperty(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    property_name: JString,
    value: jdouble,
) {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setFloatProperty] [ERROR] World pointer is null"
        );
        return;
    }

    let world = unsafe { &mut *world };
    let entity = convert_jlong_to_entity!(entity_id);

    let key = env.get_string(&property_name);
    let key: String = if let Ok(str) = key {
        let value = str.to_string_lossy();
        value.to_string()
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setFloatProperty] [ERROR] Failed to get property name"
        );
        return;
    };

    if let Ok((_, props)) = world.query_one_mut::<(&MeshRenderer, &mut ModelProperties)>(entity) {
        props.set_property(key, Value::Float(value));
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setFloatProperty] [ERROR] Failed to query entity for model properties"
        );
    }
}

/// `JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setBoolProperty
///   (JNIEnv *, jclass, jlong, jlong, jstring, jboolean);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_setBoolProperty(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    property_name: JString,
    value: jboolean,
) {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setBoolProperty] [ERROR] World pointer is null"
        );
        return;
    }

    let world = unsafe { &mut *world };
    let entity = convert_jlong_to_entity!(entity_id);

    let key = env.get_string(&property_name);
    let key: String = if let Ok(str) = key {
        let value = str.to_string_lossy();
        value.to_string()
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setBoolProperty] [ERROR] Failed to get property name"
        );
        return;
    };

    let bool_value = value != 0;

    if let Ok((_, props)) = world.query_one_mut::<(&MeshRenderer, &mut ModelProperties)>(entity) {
        props.set_property(key, Value::Bool(bool_value));
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setBoolProperty] [ERROR] Failed to query entity for model properties"
        );
    }
}

/// `JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setVec3Property
///   (JNIEnv *, jclass, jlong, jlong, jstring, jfloatArray);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_setVec3Property(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    property_name: JString,
    value: jfloatArray,
) {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setVec3Property] [ERROR] World pointer is null"
        );
        return;
    }

    if value.is_null() {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setVec3Property] [ERROR] Value array is null");
        return;
    }

    let world = unsafe { &mut *world };
    let entity = convert_jlong_to_entity!(entity_id);
    #[allow(unused_unsafe)]
    let val = unsafe { value };
    let array = unsafe { JPrimitiveArray::from_raw(val) };

    let key = env.get_string(&property_name);
    let key: String = if let Ok(str) = key {
        let value = str.to_string_lossy();
        value.to_string()
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setVec3Property] [ERROR] Failed to get property name"
        );
        return;
    };

    let length = env.get_array_length(&array);

    if let Ok(length) = length {
        if length != 3 {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_setVec3Property] [ERROR] Vec3 array must have exactly 3 elements, got {}",
                length
            );
            return;
        }
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setVec3Property] [ERROR] Failed to get array length"
        );
        return;
    }

    let mut values = [0.0f32; 3];
    if env.get_float_array_region(&array, 0, &mut values).is_err() {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setVec3Property] [ERROR] Failed to get array region"
        );
        return;
    }

    if let Ok((_, props)) = world.query_one_mut::<(&MeshRenderer, &mut ModelProperties)>(entity) {
        props.set_property(key, Value::Vec3(values));
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setVec3Property] [ERROR] Failed to query entity for model properties"
        );
    }
}

/// `JNIEXPORT jobject JNICALL Java_com_dropbear_ffi_JNINative_getCamera
///   (JNIEnv *, jclass, jlong, jstring);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getCamera(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    camera_name: JString,
) -> jobject {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!("[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] World pointer is null");
        return std::ptr::null_mut();
    }

    let world = unsafe { &*world };

    let label = env.get_string(&camera_name);
    let label: String = if let Ok(str) = label {
        str.to_string_lossy().to_string()
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Failed to get camera name");
        return std::ptr::null_mut();
    };

    if let Some((id, (cam, comp))) = world
        .query::<(&Camera, &CameraComponent)>()
        .iter()
        .find(|(_, (cam, _))| cam.label == label)
    {
        if matches!(comp.camera_type, CameraType::Debug) {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getCamera] [WARN] Querying a CameraType::Debug is illegal, returning null"
            );
            return std::ptr::null_mut();
        }

        let entity_id = if let Ok(v) = env.find_class("com/dropbear/EntityId") {
            v
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to find EntityId class"
            );
            return std::ptr::null_mut();
        };
        let entity_id = if let Ok(v) = env.new_object(
            entity_id,
            "(J)V",
            &[JValue::Long(id.to_bits().get() as i64)],
        ) {
            v
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to create new entity_id object"
            );
            return std::ptr::null_mut();
        };

        let label = if let Ok(v) = env.new_string(cam.label.as_str()) {
            v
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to create new string for label"
            );
            return std::ptr::null_mut();
        };

        let eye = if let Ok(v) = create_vector3(&mut env, cam.eye.x, cam.eye.y, cam.eye.z) {
            v
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to create vector3 for eye"
            );
            return std::ptr::null_mut();
        };

        let target = if let Ok(v) =
            create_vector3(&mut env, cam.target.x, cam.target.y, cam.target.z)
        {
            v
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to create vector3 for target"
            );
            return std::ptr::null_mut();
        };

        let up = if let Ok(v) = create_vector3(&mut env, cam.up.x, cam.up.y, cam.up.z) {
            v
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to create vector3 for up"
            );
            return std::ptr::null_mut();
        };

        let class = if let Ok(v) = env.find_class("com/dropbear/Camera") {
            v
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to locate camera class"
            );
            return std::ptr::null_mut();
        };

        let camera_obj = if let Ok(v) = env.new_object(
            class,
            "(Ljava/lang/String;Lcom/dropbear/EntityId;Lcom/dropbear/math/Vector3;Lcom/dropbear/math/Vector3;Lcom/dropbear/math/Vector3;DDDDDDDD)V",
            &[
                JValue::Object(&label),
                JValue::Object(&entity_id),
                JValue::Object(&eye),
                JValue::Object(&target),
                JValue::Object(&up),
                JValue::Double(cam.aspect),
                JValue::Double(cam.settings.fov_y),
                JValue::Double(cam.znear),
                JValue::Double(cam.zfar),
                JValue::Double(cam.yaw),
                JValue::Double(cam.pitch),
                JValue::Double(cam.settings.speed),
                JValue::Double(cam.settings.sensitivity),
            ],
        ) {
            v
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to create the camera object");
            return std::ptr::null_mut();
        };

        return camera_obj.as_raw();
    }

    std::ptr::null_mut()
}

/// `JNIEXPORT jobject JNICALL Java_com_dropbear_ffi_JNINative_getAttachedCamera
///   (JNIEnv *, jclass, jlong, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getAttachedCamera(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
) -> jobject {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_getAttachedCamera] [ERROR] World pointer is null"
        );
        return std::ptr::null_mut();
    }

    let world = unsafe { &*world };
    let entity = convert_jlong_to_entity!(entity_id);

    if let Ok(mut q) = world.query_one::<(&Camera, &CameraComponent)>(entity)
        && let Some((cam, comp)) = q.get()
    {
        if matches!(comp.camera_type, CameraType::Debug) {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getAttachedCamera] [WARN] Querying a CameraType::Debug is illegal, returning null"
            );
            return std::ptr::null_mut();
        }

        let entity_id = if let Ok(v) = env.find_class("com/dropbear/EntityId") {
            v
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getAttachedCamera] [ERROR] Unable to find EntityId class"
            );
            return std::ptr::null_mut();
        };
        let entity_id = if let Ok(v) = env.new_object(
            entity_id,
            "(J)V",
            &[JValue::Long(entity.to_bits().get() as i64)],
        ) {
            v
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getAttachedCamera] [ERROR] Unable to create new entity_id object"
            );
            return std::ptr::null_mut();
        };

        let label = if let Ok(v) = env.new_string(cam.label.as_str()) {
            v
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getAttachedCamera] [ERROR] Unable to create new string for label"
            );
            return std::ptr::null_mut();
        };

        let eye = if let Ok(v) = create_vector3(&mut env, cam.eye.x, cam.eye.y, cam.eye.z) {
            v
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getAttachedCamera] [ERROR] Unable to create vector3 for eye"
            );
            return std::ptr::null_mut();
        };

        let target = if let Ok(v) =
            create_vector3(&mut env, cam.target.x, cam.target.y, cam.target.z)
        {
            v
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getAttachedCamera] [ERROR] Unable to create vector3 for target"
            );
            return std::ptr::null_mut();
        };

        let up = if let Ok(v) = create_vector3(&mut env, cam.up.x, cam.up.y, cam.up.z) {
            v
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getAttachedCamera] [ERROR] Unable to create vector3 for up"
            );
            return std::ptr::null_mut();
        };

        let class = if let Ok(v) = env.find_class("com/dropbear/Camera") {
            v
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_getAttachedCamera] [ERROR] Unable to locate camera class"
            );
            return std::ptr::null_mut();
        };

        let camera_obj = if let Ok(v) = env.new_object(
            class,
            "(Ljava/lang/String;Lcom/dropbear/EntityId;Lcom/dropbear/math/Vector3;Lcom/dropbear/math/Vector3;Lcom/dropbear/math/Vector3;DDDDDDDD)V",
            &[
                JValue::Object(&label),
                JValue::Object(&entity_id),
                JValue::Object(&eye),
                JValue::Object(&target),
                JValue::Object(&up),
                JValue::Double(cam.aspect),
                JValue::Double(cam.settings.fov_y),
                JValue::Double(cam.znear),
                JValue::Double(cam.zfar),
                JValue::Double(cam.yaw),
                JValue::Double(cam.pitch),
                JValue::Double(cam.settings.speed),
                JValue::Double(cam.settings.sensitivity),
            ],
        ) {
            v
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getAttachedCamera] [ERROR] Unable to create the camera object");
            return std::ptr::null_mut();
        };

        return camera_obj.as_raw();
    }

    std::ptr::null_mut()
}

/// `JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setCamera
///   (JNIEnv *, jclass, jlong, jobject);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_setCamera(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    camera_obj: JObject,
) {
    let world = world_handle as *mut World;
    if world.is_null() {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] World pointer is null");
        return;
    }

    let world = unsafe { &mut *world };

    let entity_id_obj = if let Ok(v) =
        env.call_method(&camera_obj, "getId", "()Lcom/dropbear/EntityId;", &[])
    {
        if let Ok(obj) = v.l() {
            obj
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to extract EntityId object"
            );
            return;
        }
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to get EntityId from camera"
        );
        return;
    };

    let entity_id = if let Ok(v) = env.call_method(&entity_id_obj, "getId", "()J", &[]) {
        if let Ok(id) = v.j() {
            id as u32
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to extract entity id value"
            );
            return;
        }
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to call getId on EntityId"
        );
        return;
    };

    let entity = unsafe { world.find_entity_from_id(entity_id) };

    let eye_obj = if let Ok(v) =
        env.call_method(&camera_obj, "getEye", "()Lcom/dropbear/math/Vector3;", &[])
    {
        if let Ok(obj) = v.l() {
            obj
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to extract eye vector"
            );
            return;
        }
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to get eye from camera"
        );
        return;
    };

    let eye = if let Some(v) = extract_vector3(&mut env, &eye_obj) {
        v
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to extract eye vector values"
        );
        return;
    };

    let target_obj = if let Ok(v) = env.call_method(
        &camera_obj,
        "getTarget",
        "()Lcom/dropbear/math/Vector3;",
        &[],
    ) {
        if let Ok(obj) = v.l() {
            obj
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to extract target vector"
            );
            return;
        }
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to get target from camera"
        );
        return;
    };

    let target = if let Some(v) = extract_vector3(&mut env, &target_obj) {
        v
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to extract target vector values"
        );
        return;
    };

    let up_obj = if let Ok(v) =
        env.call_method(&camera_obj, "getUp", "()Lcom/dropbear/math/Vector3;", &[])
    {
        if let Ok(obj) = v.l() {
            obj
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to extract up vector"
            );
            return;
        }
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to get up from camera"
        );
        return;
    };

    let up = if let Some(v) = extract_vector3(&mut env, &up_obj) {
        v
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to extract up vector values"
        );
        return;
    };

    let fov_y = if let Ok(v) = env.call_method(&camera_obj, "getFov_y", "()D", &[]) {
        if let Ok(d) = v.d() {
            d
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to extract fov_y"
            );
            return;
        }
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to get fov_y from camera"
        );
        return;
    };

    let znear = if let Ok(v) = env.call_method(&camera_obj, "getZnear", "()D", &[]) {
        if let Ok(d) = v.d() {
            d
        } else {
            return;
        }
    } else {
        return;
    };

    let zfar = if let Ok(v) = env.call_method(&camera_obj, "getZfar", "()D", &[]) {
        if let Ok(d) = v.d() {
            d
        } else {
            return;
        }
    } else {
        return;
    };

    let yaw = if let Ok(v) = env.call_method(&camera_obj, "getYaw", "()D", &[]) {
        if let Ok(d) = v.d() {
            d
        } else {
            return;
        }
    } else {
        return;
    };

    let pitch = if let Ok(v) = env.call_method(&camera_obj, "getPitch", "()D", &[]) {
        if let Ok(d) = v.d() {
            d
        } else {
            return;
        }
    } else {
        return;
    };

    let speed = if let Ok(v) = env.call_method(&camera_obj, "getSpeed", "()D", &[]) {
        if let Ok(d) = v.d() {
            d
        } else {
            return;
        }
    } else {
        return;
    };

    let sensitivity = if let Ok(v) = env.call_method(&camera_obj, "getSensitivity", "()D", &[]) {
        if let Ok(d) = v.d() {
            d
        } else {
            return;
        }
    } else {
        return;
    };

    if let Ok(mut q) = world.query_one::<&mut Camera>(entity) {
        if let Some(cam) = q.get() {
            cam.eye = eye.as_dvec3();
            cam.target = target.as_dvec3();
            cam.up = up.as_dvec3();
            cam.settings.fov_y = fov_y;
            cam.znear = znear;
            cam.zfar = zfar;
            cam.yaw = yaw;
            cam.pitch = pitch;
            cam.settings.speed = speed;
            cam.settings.sensitivity = sensitivity;
        } else {
            eprintln!(
                "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Entity does not have a Camera component"
            );
        }
    } else {
        eprintln!(
            "[Java_com_dropbear_ffi_JNINative_setCamera] [ERROR] Unable to query camera component"
        );
    }
}

/// `JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setCursorHidden
///   (JNIEnv *, jclass, jlong, jlong, jboolean);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_setCursorHidden(
    _env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
    graphics_handle: jlong,
    hide: jboolean,
) {
    let input = input_handle as InputStatePtr;
    if input.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_setCursorHidden] [ERROR] Input state pointer is null"
        );
        return;
    }
    let input = unsafe { &mut *input };

    let graphics = graphics_handle as GraphicsPtr;
    if graphics.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_setCursorHidden] [ERROR] Input state pointer is null"
        );
        return;
    }
    let graphics = unsafe { &*graphics };

    let hide = hide != JNI_FALSE;

    if let Err(e) = graphics.send(GraphicsCommand::WindowCommand(WindowCommand::HideCursor(
        hide,
    ))) {
        println!(
            "[Java_com_dropbear_ffi_JNINative_setCursorHidden] [ERROR] Unable to send hide cursor command: {}",
            e
        );
    }

    input.is_cursor_hidden = hide;
}

/// `JNIEXPORT jboolean JNICALL Java_com_dropbear_ffi_JNINative_isCursorHidden
///   (JNIEnv *, jclass, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_isCursorHidden(
    _env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
) -> jboolean {
    let input = input_handle as InputStatePtr;
    if input.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_isCursorHidden] [ERROR] Input state pointer is null"
        );
        return false.into();
    }
    let input = unsafe { &*input };

    if input.is_cursor_hidden {
        true.into()
    } else {
        false.into()
    }
}

/// `JNIEXPORT jlong JNICALL Java_com_dropbear_ffi_JNINative_getModel
///   (JNIEnv *, jclass, jlong, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getModel(
    _env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
) -> jlong {
    let world = world_handle as WorldPtr;
    if world.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_getModel] [ERROR] World pointer is null");
        return -1;
    }

    let world = unsafe { &*world };
    let entity = convert_jlong_to_entity!(entity_id);

    if let Ok(mut q) = world.query_one::<&MeshRenderer>(entity)
        && let Some(model) = q.get()
    {
        let handle = model.asset_handle();
        handle.raw() as jlong
    } else {
        println!(
            "[Java_com_dropbear_ffi_JNINative_getModel] [ERROR] Unable to find entity in world"
        );
        -1
    }
}

/// `JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setModel
///   (JNIEnv *, jclass, jlong, jlong, jlong, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_setModel(
    _env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    asset_handle: jlong,
    entity_id: jlong,
    model_handle: jlong,
) {
    let world = world_handle as WorldPtr;
    if world.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_setModel] [ERROR] World pointer is null");
        return;
    }

    let asset = asset_handle as AssetRegistryPtr;
    if asset.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_setModel] [ERROR] Asset registry pointer is null"
        );
        return;
    }

    let world = unsafe { &*world };
    let asset = unsafe { &*asset };
    let entity = convert_jlong_to_entity!(entity_id);

    if let Ok(mut q) = world.query_one::<&mut MeshRenderer>(entity)
        && let Some(model) = q.get()
    {
        let asset_handle = AssetHandle::new(model_handle as u64);
        if !asset.contains_handle(asset_handle) {
            println!("[Java_com_dropbear_ffi_JNINative_setModel] [ERROR] Invalid model handle");
            return;
        }
        if let Err(e) = model.set_asset_handle_raw(asset, asset_handle) {
            println!(
                "[Java_com_dropbear_ffi_JNINative_setModel] [ERROR] Unable to set model: {}",
                e
            );
        }
    }
}

/// `JNIEXPORT jboolean JNICALL Java_com_dropbear_ffi_JNINative_isModelHandle
///   (JNIEnv *, jclass, jlong, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_isModelHandle(
    _env: JNIEnv,
    _class: JClass,
    asset_handle: jlong,
    model_id: jlong,
) -> jboolean {
    let asset = convert_ptr!(asset_handle, AssetRegistryPtr => AssetRegistry);

    asset
        .contains_handle(AssetHandle::new(model_id as u64))
        .into()
}

/// `JNIEXPORT jboolean JNICALL Java_com_dropbear_ffi_JNINative_isUsingModel
///   (JNIEnv *, jclass, jlong, jlong, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_isUsingModel(
    _env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    model_handle: jlong,
) -> jboolean {
    let world = convert_ptr!(world_handle, WorldPtr => World);
    let entity = convert_jlong_to_entity!(entity_id);

    let handle = AssetHandle::new(model_handle as u64);
    if let Ok(mut q) = world.query_one::<&MeshRenderer>(entity)
        && let Some(model) = q.get()
    {
        if model.asset_handle() == handle {
            true.into()
        } else {
            false.into()
        }
    } else {
        println!(
            "[Java_com_dropbear_ffi_JNINative_isUsingModel] [ERROR] Unable to find entity in world"
        );
        false.into()
    }
}

/// `JNIEXPORT jlong JNICALL Java_com_dropbear_ffi_JNINative_getTexture
///   (JNIEnv *, jclass, jlong, jlong, jlong, jstring);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getTexture(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    asset_handle: jlong,
    entity_id: jlong,
    name: JString,
) -> jlong {
    let world = world_handle as WorldPtr;
    if world.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_getTexture] [ERROR] World pointer is null");
        return -1;
    }

    let world = unsafe { &*world };

    let asset = convert_ptr!(asset_handle, AssetRegistryPtr => AssetRegistry);
    let entity = convert_jlong_to_entity!(entity_id);

    if let Ok(mut q) = world.query_one::<&MeshRenderer>(entity)
        && let Some(mesh) = q.get()
    {
        let str = convert_jstring!(env, name);
        if let Some(handle) = mesh.material_handle_raw(asset, str.as_str()) {
            handle.raw() as jlong
        } else {
            println!("[Java_com_dropbear_ffi_JNINative_getTexture] [ERROR] Invalid texture handle");
            0
        }
    } else {
        println!(
            "[Java_com_dropbear_ffi_JNINative_getTexture] [ERROR] Unable to find entity in world"
        );
        0
    }
}

/// `JNIEXPORT jobjectArray JNICALL Java_com_dropbear_ffi_JNINative_getAllTextures
///   (JNIEnv *, jclass, jlong, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getAllTextures(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
) -> jobjectArray {
    let world = convert_ptr!(world_handle, WorldPtr => World);
    let entity = convert_jlong_to_entity!(entity_id);

    let mut query = match world.query_one::<&mut MeshRenderer>(entity) {
        Ok(query) => query,
        Err(e) => {
            let message = format!(
                "[Java_com_dropbear_ffi_JNINative_getAllTextures] [ERROR] Failed to query entity: {}",
                e
            );
            set_last_error_message(&message);
            println!("{}", message);
            return std::ptr::null_mut();
        }
    };

    let renderer = match query.get() {
        Some(renderer) => renderer,
        None => {
            let message = "[Java_com_dropbear_ffi_JNINative_getAllTextures] [ERROR] Entity does not have a MeshRenderer component";
            set_last_error_message(message);
            println!("{}", message);
            return std::ptr::null_mut();
        }
    };

    renderer.clear_texture_identifier_cache();

    let model = renderer.model();
    let model_id = renderer.model_id();

    let mut seen = HashSet::new();
    let mut textures = Vec::new();

    for material in &model.materials {
        renderer.register_texture_identifier(material.name.clone(), material.name.clone());
        if let Some(tag) = &material.texture_tag {
            renderer.register_texture_identifier(tag.clone(), material.name.clone());
        }

        let reference = ASSET_REGISTRY
            .material_handle(model_id, &material.name)
            .and_then(|handle| ASSET_REGISTRY.material_reference_for_handle(handle))
            .and_then(|reference| reference.as_uri().map(|uri| uri.to_string()))
            .or_else(|| material.texture_tag.clone())
            .unwrap_or_else(|| material.name.clone());

        if seen.insert(reference.clone()) {
            renderer.register_texture_identifier(reference.clone(), material.name.clone());
            textures.push(reference);
        }
    }

    let string_class = match env.find_class("java/lang/String") {
        Ok(class) => class,
        Err(e) => {
            let message = format!(
                "[Java_com_dropbear_ffi_JNINative_getAllTextures] [ERROR] Failed to locate java/lang/String: {}",
                e
            );
            set_last_error_message(&message);
            println!("{}", message);
            return std::ptr::null_mut();
        }
    };

    let array = match env.new_object_array(textures.len() as i32, string_class, JObject::null()) {
        Ok(array) => array,
        Err(e) => {
            let message = format!(
                "[Java_com_dropbear_ffi_JNINative_getAllTextures] [ERROR] Failed to allocate string array: {}",
                e
            );
            set_last_error_message(&message);
            println!("{}", message);
            return std::ptr::null_mut();
        }
    };

    for (index, value) in textures.iter().enumerate() {
        let java_string = match env.new_string(value) {
            Ok(string) => string,
            Err(e) => {
                let message = format!(
                    "[Java_com_dropbear_ffi_JNINative_getAllTextures] [ERROR] Failed to create Java string: {}",
                    e
                );
                set_last_error_message(&message);
                println!("{}", message);
                return std::ptr::null_mut();
            }
        };

        if let Err(e) =
            env.set_object_array_element(&array, index as i32, JObject::from(java_string))
        {
            let message = format!(
                "[Java_com_dropbear_ffi_JNINative_getAllTextures] [ERROR] Failed to set array element: {}",
                e
            );
            set_last_error_message(&message);
            println!("{}", message);
            return std::ptr::null_mut();
        }
    }

    clear_last_error_message();
    array.into_raw()
}

/// `JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setTexture
///   (JNIEnv *, jclass, jlong, jlong, jlong, jstring, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_setTexture(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    asset_handle: jlong,
    entity_id: jlong,
    old_material_name: JString,
    new_texture_handle: jlong,
) {
    let world = world_handle as WorldPtr;
    if world.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] World pointer is null");
        return;
    }

    let asset = asset_handle as AssetRegistryPtr;
    if asset.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] Asset registry pointer is null"
        );
        return;
    }

    let asset = unsafe { &*asset };

    let world = unsafe { &*world };
    let entity = convert_jlong_to_entity!(entity_id);

    match world.query_one::<&mut MeshRenderer>(entity) {
        Ok(mut query) => {
            let Some(renderer) = query.get() else {
                println!(
                    "[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] Entity does not have a MeshRenderer component"
                );
                return;
            };

            let Some(cache) = asset.get_pointer(Const("model_cache")) else {
                println!(
                    "[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] Asset registry does not contain model cache"
                );
                return;
            };

            let cache = cache as *const Mutex<HashMap<String, Arc<Model>>>;
            let cache = unsafe { &*cache };

            let jni_result = env.get_string(&old_material_name);
            let target_identifier = match jni_result {
                Ok(java_string) => match java_string.to_str() {
                    Ok(rust_str) => rust_str.to_string(),
                    Err(e) => {
                        println!(
                            "[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] Failed to convert Java string to Rust string: {}",
                            e
                        );
                        return;
                    }
                },
                Err(e) => {
                    println!(
                        "[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] Failed to get string from JNI: {}",
                        e
                    );
                    return;
                }
            };

            let resolved_target_name = renderer
                .resolve_texture_identifier(&target_identifier)
                .map(|name| name.to_string())
                .or_else(|| {
                    let model = renderer.model();
                    let model_id = renderer.model_id();

                    if model
                        .materials
                        .iter()
                        .any(|material| material.name == target_identifier)
                    {
                        return Some(target_identifier.clone());
                    }

                    model.materials.iter().find_map(|material| {
                        if material.name == target_identifier {
                            return Some(material.name.clone());
                        }

                        let registry_reference = ASSET_REGISTRY
                            .material_handle(model_id, &material.name)
                            .and_then(|handle| ASSET_REGISTRY.material_reference_for_handle(handle))
                            .and_then(|reference| reference.as_uri().map(|uri| uri.to_string()));

                        if registry_reference
                            .as_ref()
                            .map(|value| value == &target_identifier)
                            .unwrap_or(false)
                        {
                            return Some(material.name.clone());
                        }

                        if material
                            .texture_tag
                            .as_ref()
                            .map(|tag| tag == &target_identifier)
                            .unwrap_or(false)
                        {
                            return Some(material.name.clone());
                        }

                        None
                    })
                });

            let Some(target_material) = resolved_target_name else {
                let message = format!(
                    "[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] Unable to resolve material '{}' on model '{}'",
                    target_identifier,
                    renderer.model().label
                );
                set_last_error_message(&message);
                println!("{}", message);
                return;
            };

            let handle = AssetHandle::new(new_texture_handle as u64);

            if !asset.contains_handle(handle) {
                println!(
                    "[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] Invalid texture handle: {}",
                    new_texture_handle
                );
                return;
            }

            if !asset.is_material(handle) {
                println!(
                    "[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] Handle {} does not refer to a material",
                    new_texture_handle
                );
                return;
            }

            let Some(material) = asset.get_material(handle) else {
                println!(
                    "[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] Invalid texture handle"
                );
                return;
            };

            let Some(owner_model_id) = asset.material_owner(handle) else {
                println!(
                    "[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] Unable to determine owning model for material handle {}",
                    new_texture_handle
                );
                return;
            };

            let Some(owner_model_handle) = asset.model_handle_from_id(owner_model_id) else {
                println!(
                    "[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] Unable to resolve model handle for owner id {:?}",
                    owner_model_id
                );
                return;
            };

            let Some(source_reference) = asset.model_reference_for_handle(owner_model_handle)
            else {
                println!(
                    "[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] Unable to resolve model reference for handle {}",
                    owner_model_handle.raw()
                );
                return;
            };

            if let Err(e) = renderer.apply_material_override_raw(
                asset,
                cache,
                target_material.as_str(),
                source_reference,
                material.name.as_str(),
            ) {
                println!(
                    "[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] Failed to apply material override: {}",
                    e
                );
            }
        }
        Err(err) => {
            println!(
                "[Java_com_dropbear_ffi_JNINative_setTexture] [ERROR] Unable to query MeshRenderer: {}",
                err
            );
        }
    }
}

/// `JNIEXPORT jstring JNICALL Java_com_dropbear_ffi_JNINative_getTextureName
///   (JNIEnv *, jclass, jlong, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getTextureName(
    env: JNIEnv,
    _class: JClass,
    asset_handle: jlong,
    texture_id: jlong,
) -> jstring {
    let asset = convert_ptr!(asset_handle, AssetRegistryPtr => AssetRegistry);

    let texture_id = AssetHandle::new(texture_id as u64);
    asset.get_material(texture_id).map_or_else(
        || {
            println!(
                "[Java_com_dropbear_ffi_JNINative_getTextureName] [ERROR] Invalid texture handle"
            );
            return std::ptr::null_mut();
        },
        |material| {
            let Ok(str) = env.new_string(material.name.as_str()) else {
                return std::ptr::null_mut();
            };

            str.into_raw()
        },
    )
}

/// `JNIEXPORT jboolean JNICALL Java_com_dropbear_ffi_JNINative_isTextureHandle
///   (JNIEnv *, jclass, jlong, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_isTextureHandle(
    _env: JNIEnv,
    _class: JClass,
    asset_handle: jlong,
    texture_id: jlong,
) -> jboolean {
    let asset = convert_ptr!(asset_handle, AssetRegistryPtr => AssetRegistry);
    let texture_id = AssetHandle::new(texture_id as u64);
    if asset.is_material(texture_id) {
        true.into()
    } else {
        false.into()
    }
}

/// `JNIEXPORT jboolean JNICALL Java_com_dropbear_ffi_JNINative_isUsingTexture
///   (JNIEnv *, jclass, jlong, jlong, jstring);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_isUsingTexture(
    _env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    texture_handle: jlong,
) -> jboolean {
    let world = convert_ptr!(world_handle, WorldPtr => World);
    let entity = convert_jlong_to_entity!(entity_id);

    if let Ok(mut q) = world.query_one::<&MeshRenderer>(entity)
        && let Some(mesh) = q.get()
    {
        mesh.contains_material_handle(AssetHandle::new(texture_handle as u64))
            .into()
    } else {
        println!(
            "[Java_com_dropbear_ffi_JNINative_isUsingTexture] [ERROR] Unable to find entity in world"
        );
        false.into()
    }
}

/// `JNIEXPORT jlong JNICALL Java_com_dropbear_ffi_JNINative_getAsset
///   (JNIEnv *, jclass, jlong, jstring);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getAsset(
    mut env: JNIEnv,
    _class: JClass,
    asset_handle: jlong,
    euca_uri: JString,
) -> jlong {
    let asset = asset_handle as AssetRegistryPtr;
    if asset.is_null() {
        println!(
            "[Java_com_dropbear_ffi_JNINative_getAsset] [ERROR] Asset registry pointer is null"
        );
        return -1;
    }

    let asset = unsafe { &*asset };

    let jni_result = env.get_string(&euca_uri);
    let str = match jni_result {
        Ok(java_string) => match java_string.to_str() {
            Ok(rust_str) => rust_str.to_string(),
            Err(e) => {
                println!(
                    "[Java_com_dropbear_ffi_JNINative_getAsset] [ERROR] Failed to convert Java string to Rust string: {}",
                    e
                );
                return -1;
            }
        },
        Err(e) => {
            println!(
                "[Java_com_dropbear_ffi_JNINative_getAsset] [ERROR] Failed to get string from JNI: {}",
                e
            );
            return -1;
        }
    };
    if let Ok(res) = ResourceReference::from_euca_uri(str)
        && let Some(asset_handle) = asset.get_handle_from_reference(&res)
    {
        return asset_handle.raw() as jlong;
    };
    0 as jlong
}

/// `JNIEXPORT jstring JNICALL Java_com_dropbear_ffi_JNINative_getLastErrorMsg
///   (JNIEnv *, jclass, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getLastErrorMsg(
    env: JNIEnv,
    _class: JClass,
    asset_handle: jlong,
) -> jstring {
    let asset = convert_ptr!(asset_handle, AssetRegistryPtr => AssetRegistry);
    if asset.get_pointer(Const("last_error_msg")).is_none() {
        let ptr = get_last_error_message_ptr() as usize;
        asset.add_pointer(Const("last_error_msg"), ptr);
    }
    let message = get_last_error_message();
    match env.new_string(&message) {
        Ok(java_string) => {
            clear_last_error_message();
            java_string.into_raw()
        }
        Err(e) => {
            let fallback = format!(
                "[Java_com_dropbear_ffi_JNINative_getLastErrorMsg] [ERROR] Failed to allocate Java string: {}",
                e
            );
            println!("{}", fallback);
            set_last_error_message(&fallback);
            std::ptr::null_mut()
        }
    }
}

/// `JNIEXPORT jlong JNICALL Java_com_dropbear_ffi_JNINative_getLastErrMsgPtr
///   (JNIEnv *, jclass, jlong);`
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getLastErrMsgPtr(
    _env: JNIEnv,
    _class: JClass,
    asset_handle: jlong,
) -> jlong {
    let asset = convert_ptr!(asset_handle, AssetRegistryPtr => AssetRegistry);
    let pointer = asset
        .get_pointer(Const("last_error_msg"))
        .unwrap_or_else(|| {
            let ptr = get_last_error_message_ptr() as usize;
            asset.add_pointer(Const("last_error_msg"), ptr);
            ptr
        });
    pointer as jlong
}
