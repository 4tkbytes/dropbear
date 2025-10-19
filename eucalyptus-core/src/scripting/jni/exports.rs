use glam::{DQuat, DVec3};
use hecs::World;
use jni::JNIEnv;
use jni::objects::{JObject, JString};
use jni::sys::{jboolean, jclass, jint, jlong};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::scancode::PhysicalKeyExtScancode;
use dropbear_engine::entity::{AdoptedEntity, Transform};
use crate::ptr::InputStatePtr;
use num_enum::FromPrimitive;
use crate::utils::keycode_from_ordinal;

// JNIEXPORT jlong JNICALL Java_com_dropbear_ffi_JNINative_getEntity
//   (JNIEnv *, jclass, jlong, jstring);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getEntity(
    mut env: JNIEnv,
    _obj: jclass,
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
    _class: jclass,
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
    _class: jclass,
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
    _class: jclass,
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