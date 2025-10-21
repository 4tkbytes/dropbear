use crate::ptr::InputStatePtr;
use crate::scripting::jni::utils::{create_vector3, java_button_to_rust, new_float_array};
use crate::utils::keycode_from_ordinal;
use dropbear_engine::entity::{AdoptedEntity, Transform};
use glam::{DQuat, DVec3};
use hecs::World;
use jni::objects::{JClass, JObject, JPrimitiveArray, JString, JValue};
use jni::sys::{jboolean, jclass, jdouble, jfloatArray, jint, jlong, jobject, jstring};
use jni::JNIEnv;
use dropbear_engine::camera::Camera;
use crate::camera::{CameraComponent, CameraType};
use crate::states::{ModelProperties, Value};

// JNIEXPORT jlong JNICALL Java_com_dropbear_ffi_JNINative_getEntity
//   (JNIEnv *, jclass, jlong, jstring);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getEntity(
    mut env: JNIEnv,
    _obj: JClass,
    world_handle: jlong,
    label: JString,
) -> jlong {
    let label_jni_result = env.get_string(&label);
    let label_str = match label_jni_result {
        Ok(java_string) => {
            match java_string.to_str() {
                Ok(rust_str) => rust_str.to_string(),
                Err(e) => {
                    println!("[Java_com_dropbear_ffi_JNINative_getEntity] [ERROR] Failed to convert Java string to Rust string: {}", e);
                    return -1;
                }
            }
        },
        Err(e) => {
            println!("[Java_com_dropbear_ffi_JNINative_getEntity] [ERROR] Failed to get string from JNI: {}", e);
            return -1;
        }
    };

    let world = world_handle as *mut World;

    if world.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_getEntity] [ERROR] World pointer is null");
        return -1;
    }

    let world = unsafe { &mut *world };

    for (id, entity) in world.query::<&AdoptedEntity>().iter() {
        if entity.model.label == label_str {
            return id.id() as jlong;
        }
    }
    -1
}

// JNIEXPORT jobject JNICALL Java_com_dropbear_ffi_JNINative_getTransform
//   (JNIEnv *, jclass, jlong, jlong);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getTransform(
    mut env: JNIEnv,
    _class: jclass,
    world_handle: jlong,
    entity_id: jlong,
) -> JObject {
    let world = world_handle as *mut World;

    if world.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_getTransform] [ERROR] World pointer is null");
        return JObject::null();
    }

    let world = unsafe { &mut *world };

    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    if let Ok(mut q) = world.query_one::<(&AdoptedEntity, &Transform)>(entity)
        && let Some((_, transform)) = q.get()
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
                println!("[Java_com_dropbear_ffi_JNINative_getTransform] [ERROR] Failed to create Transform object");
                JObject::null()
            }
        };
    }

    println!("[Java_com_dropbear_ffi_JNINative_getTransform] [ERROR] Failed to query for transform value for entity: {}", entity_id);
    JObject::null()
}

// JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setTransform
//   (JNIEnv *, jclass, jlong, jlong, jobject);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_setTransform(
    mut env: JNIEnv,
    _class: JClass,
    world_handle: jlong,
    entity_id: jlong,
    transform_obj: JObject,
) {
    let world = world_handle as *mut World;

    if world.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] World pointer is null");
        return;
    }

    let world = unsafe { &mut *world };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    let get_number_field = |env: &mut JNIEnv, obj: &JObject, field_name: &str| -> f64 {
        match env.get_field(obj, field_name, "Ljava/lang/Number;") {
            Ok(v) => match v.l() {
                Ok(num_obj) => {
                    match env.call_method(&num_obj, "doubleValue", "()D", &[]) {
                        Ok(result) => result.d().unwrap_or_else(|_| {
                            println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] Failed to extract double from {}", field_name);
                            0.0
                        }),
                        Err(_) => {
                            println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] Failed to call doubleValue on {}", field_name);
                            0.0
                        }
                    }
                }
                Err(_) => {
                    println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] Failed to extract Number object for {}", field_name);
                    0.0
                }
            },
            Err(_) => {
                println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] Failed to get field {}", field_name);
                0.0
            }
        }
    };

    let position_obj: JObject = match env.get_field(&transform_obj, "position", "Lcom/dropbear/math/Vector3;") {
        Ok(v) => v.l().unwrap_or_else(|_| JObject::null()),
        Err(_) => JObject::null(),
    };

    let rotation_obj: JObject = match env.get_field(&transform_obj, "rotation", "Lcom/dropbear/math/Quaternion;") {
        Ok(v) => v.l().unwrap_or_else(|_| JObject::null()),
        Err(_) => JObject::null(),
    };

    let scale_obj: JObject = match env.get_field(&transform_obj, "scale", "Lcom/dropbear/math/Vector3;") {
        Ok(v) => v.l().unwrap_or_else(|_| JObject::null()),
        Err(_) => JObject::null(),
    };

    if position_obj.is_null() || rotation_obj.is_null() || scale_obj.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] Failed to extract position/rotation/scale objects");
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

    let new_transform = Transform {
        position: DVec3::new(px, py, pz),
        rotation: DQuat::from_axis_angle(DVec3::new(rx, ry, rz), rw),
        scale: DVec3::new(sx, sy, sz),
    };

    if let Ok(mut q) = world.query_one::<&mut Transform>(entity) {
        if let Some(transform) = q.get() {
            *transform = new_transform;
        } else {
            println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] Failed to query for transform component");
        }
    } else {
        println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] Failed to query entity for transform component");
    }
}

// JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_printInputState
//   (JNIEnv *, jclass, jlong);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_printInputState(
    _env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
) {
    let input = input_handle as InputStatePtr;

    if input.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_printInputState] [ERROR] Input state pointer is null");
        return;
    }

    let input = unsafe { &*input };
    println!("{:#?}", input);
}

// JNIEXPORT jboolean JNICALL Java_com_dropbear_ffi_JNINative_isKeyPressed
//   (JNIEnv *, jclass, jlong, jint);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_isKeyPressed(
    _env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
    key: jint,
) -> jboolean {
    let input = input_handle as InputStatePtr;
    if input.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_isKeyPressed] [ERROR] Input state pointer is null");
        return false.into();
    }
    let input = unsafe { &*input };

    println!("[Java_com_dropbear_ffi_JNINative_isKeyPressed] [DEBUG] Original code: {:?}", key);

    match keycode_from_ordinal(key) {
        Some(k) => {
            println!("[Java_com_dropbear_ffi_JNINative_isKeyPressed] [DEBUG] Keycode: {:?}", k);
            if input.pressed_keys.contains(&k) {
                true.into()
            } else {
                false.into()
            }
        }
        None => {
            println!("[Java_com_dropbear_ffi_JNINative_isKeyPressed] [WARN] Ordinal keycode is invalid");
            false.into()
        }
    }
}

// JNIEXPORT jfloatArray JNICALL Java_com_dropbear_ffi_JNINative_getMousePosition
//   (JNIEnv *, jclass, jlong);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getMousePosition(
    mut env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
) -> jfloatArray {
    let input = input_handle as InputStatePtr;
    if input.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_getMousePosition] [ERROR] Input state pointer is null");
        return new_float_array(&mut env, -1.0, -1.0);
    }

    let input = unsafe { &*input };

    new_float_array(&mut env, input.mouse_pos.0 as f32, input.mouse_pos.1 as f32)
}

// JNIEXPORT jboolean JNICALL Java_com_dropbear_ffi_JNINative_isMouseButtonPressed
//   (JNIEnv *, jclass, jlong, jint);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_isMouseButtonPressed(
    _env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
    button: jint,
) -> jboolean {
    let input_ptr = input_handle as InputStatePtr;

    if input_ptr.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_isMouseButtonPressed] [ERROR] Input state pointer is null");
        return false as jboolean;
    }

    let input = unsafe { &*input_ptr };

    if let Some(rust_button) = java_button_to_rust(button) {
        let is_pressed = input.mouse_button.contains(&rust_button);
        is_pressed as jboolean
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_isMouseButtonPressed] [ERROR] Invalid button code: {}", button);
        false as jboolean
    }
}

// JNIEXPORT jfloatArray JNICALL Java_com_dropbear_ffi_JNINative_getMouseDelta
//   (JNIEnv *, jclass, jlong);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getMouseDelta(
    mut env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
) -> jfloatArray {
    let input = input_handle as InputStatePtr;
    if input.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_getMouseDelta] [ERROR] Input state pointer is null");
        return new_float_array(&mut env, -1.0, -1.0);
    }

    let input = unsafe { &*input };

    if let Some(pos) = input.mouse_delta {
        new_float_array(&mut env, pos.0 as f32, pos.1 as f32)
    } else {
        println!("[Java_com_dropbear_ffi_JNINative_getMouseDelta] [WARN] input_state.mouse_delta returns \"(None)\". Returning (0.0, 0.0)");
        new_float_array(&mut env, 0.0, 0.0)
    }
}

// JNIEXPORT jboolean JNICALL Java_com_dropbear_ffi_JNINative_isCursorLocked
//   (JNIEnv *, jclass, jlong);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_isCursorLocked(
    _env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
) -> jboolean {
    let input = input_handle as InputStatePtr;
    if input.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_isCursorLocked] [ERROR] Input state pointer is null");
        return false as jboolean;
    }

    let input = unsafe { &*input };

    input.is_cursor_locked as jboolean
}

// JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setCursorLocked
//   (JNIEnv *, jclass, jlong, jboolean);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_setCursorLocked(
    _env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
    locked: jboolean,
) {
    let input = input_handle as InputStatePtr;

    if input.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_isCursorLocked] [ERROR] Input state pointer is null");
        return;
    }

    let input = unsafe { &mut *input };

    input.is_cursor_locked = locked != 0;
}

// JNIEXPORT jfloatArray JNICALL Java_com_dropbear_ffi_JNINative_getLastMousePos
//   (JNIEnv *, jclass, jlong);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getLastMousePos(
    mut env: JNIEnv,
    _class: JClass,
    input_handle: jlong,
) -> jfloatArray {
    let input = input_handle as InputStatePtr;
    if input.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_getLastMousePos] [ERROR] Input state pointer is null");
        return new_float_array(&mut env, -1.0, -1.0);
    }

    let input = unsafe { &*input };
    if let Some(pos) = input.last_mouse_pos {
        new_float_array(&mut env, pos.0 as f32, pos.1 as f32)
    } else {
        new_float_array(&mut env, 0.0, 0.0)
    }
}

// JNIEXPORT jstring JNICALL Java_com_dropbear_ffi_JNINative_getStringProperty
//   (JNIEnv *, jclass, jlong, jlong, jstring);
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
        eprintln!("[Java_com_dropbear_ffi_JNINative_getStringProperty] [ERROR] World pointer is null");
        return std::ptr::null_mut();
    }

    let world = unsafe { &mut *world };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };
    if let Ok(mut q) = world.query_one::<(&AdoptedEntity, &ModelProperties)>(entity)
        && let Some((_, props)) = q.get()
    {
        let string = env.get_string(&property_name);
        let value: String = if let Ok(str) = string {
            let value = str.to_string_lossy();
            value.to_string()
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getStringProperty] [ERROR] Failed to get property name");
            return std::ptr::null_mut();
        };
        let output = props.get_property(&value);
        if let Some(output) = output {
            match output {
                Value::String(val) => {
                    match env.new_string(val) {
                        Ok(string) => {
                            string.as_raw()
                        }
                        Err(e) => {
                            eprintln!("[Java_com_dropbear_ffi_JNINative_getStringProperty] [ERROR] Failed to create string: {}", e);
                            std::ptr::null_mut()
                        }
                    }
                }
                _ => {
                    println!("[Java_com_dropbear_ffi_JNINative_getStringProperty] [WARN] Property is not a string");
                    std::ptr::null_mut()
                }
            }
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getStringProperty] [WARN] Property not found");
            std::ptr::null_mut()
        }
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_getStringProperty] [ERROR] Failed to query entity for model properties");
        std::ptr::null_mut()
    }
}

// JNIEXPORT jint JNICALL Java_com_dropbear_ffi_JNINative_getIntProperty
//   (JNIEnv *, jclass, jlong, jlong, jstring);
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
        return 0;
    }

    let world = unsafe { &mut *world };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };
    if let Ok(mut q) = world.query_one::<(&AdoptedEntity, &ModelProperties)>(entity)
        && let Some((_, props)) = q.get()
    {
        let string = env.get_string(&property_name);
        let value: String = if let Ok(str) = string {
            let value = str.to_string_lossy();
            value.to_string()
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getIntProperty] [ERROR] Failed to get property name");
            return 0;
        };
        let output = props.get_property(&value);
        if let Some(output) = output {
            match output {
                Value::Int(val) => *val as jint,
                _ => {
                    eprintln!("[Java_com_dropbear_ffi_JNINative_getIntProperty] [WARN] Property is not an int");
                    0
                }
            }
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getIntProperty] [WARN] Property not found");
            0
        }
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_getIntProperty] [ERROR] Failed to query entity for model properties");
        0
    }
}

// JNIEXPORT jlong JNICALL Java_com_dropbear_ffi_JNINative_getLongProperty
//   (JNIEnv *, jclass, jlong, jlong, jstring);
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
        eprintln!("[Java_com_dropbear_ffi_JNINative_getLongProperty] [ERROR] World pointer is null");
        return 0;
    }

    let world = unsafe { &mut *world };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };
    if let Ok(mut q) = world.query_one::<(&AdoptedEntity, &ModelProperties)>(entity)
        && let Some((_, props)) = q.get()
    {
        let string = env.get_string(&property_name);
        let value: String = if let Ok(str) = string {
            let value = str.to_string_lossy();
            value.to_string()
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getLongProperty] [ERROR] Failed to get property name");
            return 0;
        };
        let output = props.get_property(&value);
        if let Some(output) = output {
            match output {
                Value::Int(val) => *val as jlong,
                _ => {
                    eprintln!("[Java_com_dropbear_ffi_JNINative_getLongProperty] [WARN] Property is not a long");
                    0
                }
            }
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getLongProperty] [WARN] Property not found");
            0
        }
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_getLongProperty] [ERROR] Failed to query entity for model properties");
        0
    }
}

// JNIEXPORT jdouble JNICALL Java_com_dropbear_ffi_JNINative_getFloatProperty
//   (JNIEnv *, jclass, jlong, jlong, jstring);
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
        eprintln!("[Java_com_dropbear_ffi_JNINative_getFloatProperty] [ERROR] World pointer is null");
        return 0.0;
    }

    let world = unsafe { &mut *world };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };
    if let Ok(mut q) = world.query_one::<(&AdoptedEntity, &ModelProperties)>(entity)
        && let Some((_, props)) = q.get()
    {
        let string = env.get_string(&property_name);
        let value: String = if let Ok(str) = string {
            let value = str.to_string_lossy();
            value.to_string()
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getFloatProperty] [ERROR] Failed to get property name");
            return 0.0;
        };
        let output = props.get_property(&value);
        if let Some(output) = output {
            match output {
                Value::Float(val) => *val as jdouble,
                _ => {
                    eprintln!("[Java_com_dropbear_ffi_JNINative_getFloatProperty] [WARN] Property is not a float");
                    0.0
                }
            }
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getFloatProperty] [WARN] Property not found");
            0.0
        }
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_getFloatProperty] [ERROR] Failed to query entity for model properties");
        0.0
    }
}

// JNIEXPORT jboolean JNICALL Java_com_dropbear_ffi_JNINative_getBoolProperty
//   (JNIEnv *, jclass, jlong, jlong, jstring);
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
        eprintln!("[Java_com_dropbear_ffi_JNINative_getBoolProperty] [ERROR] World pointer is null");
        return 0;
    }

    let world = unsafe { &mut *world };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };
    if let Ok(mut q) = world.query_one::<(&AdoptedEntity, &ModelProperties)>(entity)
        && let Some((_, props)) = q.get()
    {
        let string = env.get_string(&property_name);
        let value: String = if let Ok(str) = string {
            let value = str.to_string_lossy();
            value.to_string()
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getBoolProperty] [ERROR] Failed to get property name");
            return 0;
        };
        let output = props.get_property(&value);
        if let Some(output) = output {
            match output {
                Value::Bool(val) => if *val { 1 } else { 0 },
                _ => {
                    eprintln!("[Java_com_dropbear_ffi_JNINative_getBoolProperty] [WARN] Property is not a bool");
                    0
                }
            }
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getBoolProperty] [WARN] Property not found");
            0
        }
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_getBoolProperty] [ERROR] Failed to query entity for model properties");
        0
    }
}

// JNIEXPORT jfloatArray JNICALL Java_com_dropbear_ffi_JNINative_getVec3Property
//   (JNIEnv *, jclass, jlong, jlong, jstring);
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
        eprintln!("[Java_com_dropbear_ffi_JNINative_getVec3Property] [ERROR] World pointer is null");
        return std::ptr::null_mut();
    }

    let world = unsafe { &mut *world };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };
    if let Ok(mut q) = world.query_one::<(&AdoptedEntity, &ModelProperties)>(entity)
        && let Some((_, props)) = q.get()
    {
        let string = env.get_string(&property_name);
        let value: String = if let Ok(str) = string {
            let value = str.to_string_lossy();
            value.to_string()
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getVec3Property] [ERROR] Failed to get property name");
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
                            eprintln!("[Java_com_dropbear_ffi_JNINative_getVec3Property] [ERROR] Failed to set array region");
                            std::ptr::null_mut()
                        }
                    } else {
                        eprintln!("[Java_com_dropbear_ffi_JNINative_getVec3Property] [ERROR] Failed to create float array");
                        std::ptr::null_mut()
                    }
                }
                _ => {
                    eprintln!("[Java_com_dropbear_ffi_JNINative_getVec3Property] [WARN] Property is not a vec3");
                    std::ptr::null_mut()
                }
            }
        } else {
            eprintln!("[Java_com_dropbear_ffi_JNINative_getVec3Property] [WARN] Property not found");
            std::ptr::null_mut()
        }
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_getVec3Property] [ERROR] Failed to query entity for model properties");
        std::ptr::null_mut()
    }
}

// JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setStringProperty
//   (JNIEnv *, jclass, jlong, jlong, jstring, jstring);
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
        eprintln!("[Java_com_dropbear_ffi_JNINative_setStringProperty] [ERROR] World pointer is null");
        return;
    }

    let world = unsafe { &mut *world };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    let key = env.get_string(&property_name);
    let key: String = if let Ok(str) = key {
        let value = str.to_string_lossy();
        value.to_string()
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setStringProperty] [ERROR] Failed to get property name");
        return;
    };

    let string = env.get_string(&value);
    let value: String = if let Ok(str) = string {
        let value = str.to_string_lossy();
        value.to_string()
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setStringProperty] [ERROR] Failed to get property name");
        return;
    };

    if let Ok((_, props)) = world.query_one_mut::<(&AdoptedEntity, &mut ModelProperties)>(entity) {
        props.set_property(key, Value::String(value));
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setStringProperty] [ERROR] Failed to query entity for model properties");
    }
}

// JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setIntProperty
//   (JNIEnv *, jclass, jlong, jlong, jstring, jint);
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
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    let key = env.get_string(&property_name);
    let key: String = if let Ok(str) = key {
        let value = str.to_string_lossy();
        value.to_string()
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setIntProperty] [ERROR] Failed to get property name");
        return;
    };

    if let Ok((_, props)) = world.query_one_mut::<(&AdoptedEntity, &mut ModelProperties)>(entity)
    {
        props.set_property(key, Value::Int(value as i64));
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setIntProperty] [ERROR] Failed to query entity for model properties");
    }
}

// JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setLongProperty
//   (JNIEnv *, jclass, jlong, jlong, jstring, jlong);
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
        eprintln!("[Java_com_dropbear_ffi_JNINative_setLongProperty] [ERROR] World pointer is null");
        return;
    }

    let world = unsafe { &mut *world };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    let key = env.get_string(&property_name);
    let key: String = if let Ok(str) = key {
        let value = str.to_string_lossy();
        value.to_string()
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setLongProperty] [ERROR] Failed to get property name");
        return;
    };

    if let Ok((_, props))= world.query_one_mut::<(&AdoptedEntity, &mut ModelProperties)>(entity)
    {
        props.set_property(key, Value::Int(value));
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setLongProperty] [ERROR] Failed to query entity for model properties");
    }
}

// JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setFloatProperty
//   (JNIEnv *, jclass, jlong, jlong, jstring, jdouble);
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
        eprintln!("[Java_com_dropbear_ffi_JNINative_setFloatProperty] [ERROR] World pointer is null");
        return;
    }

    let world = unsafe { &mut *world };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    let key = env.get_string(&property_name);
    let key: String = if let Ok(str) = key {
        let value = str.to_string_lossy();
        value.to_string()
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setFloatProperty] [ERROR] Failed to get property name");
        return;
    };

    if let Ok((_, props)) = world.query_one_mut::<(&AdoptedEntity, &mut ModelProperties)>(entity)
    {
        props.set_property(key, Value::Float(value));
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setFloatProperty] [ERROR] Failed to query entity for model properties");
    }
}

// JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setBoolProperty
//   (JNIEnv *, jclass, jlong, jlong, jstring, jboolean);
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
        eprintln!("[Java_com_dropbear_ffi_JNINative_setBoolProperty] [ERROR] World pointer is null");
        return;
    }

    let world = unsafe { &mut *world };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    let key = env.get_string(&property_name);
    let key: String = if let Ok(str) = key {
        let value = str.to_string_lossy();
        value.to_string()
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setBoolProperty] [ERROR] Failed to get property name");
        return;
    };

    let bool_value = value != 0;

    if let Ok((_, props)) = world.query_one_mut::<(&AdoptedEntity, &mut ModelProperties)>(entity)
    {
        props.set_property(key, Value::Bool(bool_value));
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setBoolProperty] [ERROR] Failed to query entity for model properties");
    }
}

// JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setVec3Property
//   (JNIEnv *, jclass, jlong, jlong, jstring, jfloatArray);
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
        eprintln!("[Java_com_dropbear_ffi_JNINative_setVec3Property] [ERROR] World pointer is null");
        return;
    }

    if value.is_null() {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setVec3Property] [ERROR] Value array is null");
        return;
    }

    let world = unsafe { &mut *world };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    let array = unsafe { JPrimitiveArray::from_raw(value) };

    let key = env.get_string(&property_name);
    let key: String = if let Ok(str) = key {
        let value = str.to_string_lossy();
        value.to_string()
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setVec3Property] [ERROR] Failed to get property name");
        return;
    };

    let length = env.get_array_length(&array);

    if let Ok(length) = length {
        if length != 3 {
            eprintln!("[Java_com_dropbear_ffi_JNINative_setVec3Property] [ERROR] Vec3 array must have exactly 3 elements, got {}", length);
            return;
        }
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setVec3Property] [ERROR] Failed to get array length");
        return;
    }

    let mut values = [0.0f32; 3];
    if env.get_float_array_region(&array, 0, &mut values).is_err() {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setVec3Property] [ERROR] Failed to get array region");
        return;
    }

    if let Ok((_, props)) = world.query_one_mut::<(&AdoptedEntity, &mut ModelProperties)>(entity)
    {
        props.set_property(key, Value::Vec3(values));
    } else {
        eprintln!("[Java_com_dropbear_ffi_JNINative_setVec3Property] [ERROR] Failed to query entity for model properties");
    }
}

// JNIEXPORT jobject JNICALL Java_com_dropbear_ffi_JNINative_getCamera
//   (JNIEnv *, jclass, jlong, jstring);
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

    if let Some((id, (cam, comp))) = world.query::<(&Camera, &CameraComponent)>().iter().next() {
        return if cam.label == label {
            if matches!(comp.camera_type, CameraType::Debug) {
                eprintln!("[Java_com_dropbear_ffi_JNINative_getCamera] [WARN] Querying a CameraType::Debug is illegal, returning null");
                std::ptr::null_mut()
            } else {
                let entity_id = if let Ok(v) = env.find_class("com/dropbear/EntityId") {
                    v
                } else {
                    eprintln!("[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to find EntityId class");
                    return std::ptr::null_mut();
                };
                let entity_id = if let Ok(v) = env.new_object(
                    entity_id,
                    "(J)V",
                    &[JValue::Long(id.id() as i64)],
                ) {
                    v
                } else {
                    eprintln!("[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to create new entity_id object");
                    return std::ptr::null_mut();
                };

                let label = if let Ok(v) = env.new_string(cam.label.as_str()) {
                    v
                } else {
                    eprintln!("[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to create new string for label");
                    return std::ptr::null_mut();
                };

                let eye = if let Ok(v) = create_vector3(&mut env, cam.eye.x, cam.eye.y, cam.eye.z) {
                    v
                } else {
                    eprintln!("[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to create vector3 for eye");
                    return std::ptr::null_mut();
                };

                let target = if let Ok(v) = create_vector3(&mut env, cam.target.x, cam.target.y, cam.target.z) {
                    v
                } else {
                    eprintln!("[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to create vector3 for target");
                    return std::ptr::null_mut();
                };

                let up = if let Ok(v) = create_vector3(&mut env, cam.up.x, cam.up.y, cam.up.z) {
                    v
                } else {
                    eprintln!("[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to create vector3 for up");
                    return std::ptr::null_mut();
                };

                let class = if let Ok(v) = env.find_class("com/dropbear/Camera") {
                    v
                } else {
                    eprintln!("[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to locate camera class");
                    return std::ptr::null_mut();
                };

                let camera_obj = if let Ok(v) = env.new_object(
                    class,
                    "(Ljava/lang/String;Lcom/dropbear/EntityId;Lcom/dropbear/math/Vector3<Ljava/lang/Double;>;Lcom/dropbear/math/Vector3<Ljava/lang/Double;>;Lcom/dropbear/math/Vector3<Ljava/lang/Double;>;DDDDDDDD)V",
                    &[
                        JValue::Object(&label),
                        JValue::Object(&entity_id),
                        JValue::Object(&eye),
                        JValue::Object(&target),
                        JValue::Object(&up),
                        JValue::Double(cam.aspect),
                        JValue::Double(cam.fov_y),
                        JValue::Double(cam.znear),
                        JValue::Double(cam.zfar),
                        JValue::Double(cam.yaw),
                        JValue::Double(cam.pitch),
                        JValue::Double(cam.speed),
                        JValue::Double(cam.sensitivity),
                    ]
                ) {
                    v
                } else {
                    eprintln!("[Java_com_dropbear_ffi_JNINative_getCamera] [ERROR] Unable to create the camera object");
                    return std::ptr::null_mut();
                };

                camera_obj.as_raw()
            }
        } else {
            std::ptr::null_mut()
        }
    }

    std::ptr::null_mut()
}