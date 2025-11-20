use crate::camera::{CameraComponent, CameraType};
use crate::ptr::{GraphicsPtr, InputStatePtr};
use crate::scripting::native::DropbearNativeError;
use crate::scripting::native::types::{NativeCamera, NativeTransform, Vector3D};
use crate::states::{Label, ModelProperties, Value};
use crate::utils::keycode_from_ordinal;
use crate::window::{GraphicsCommand, WindowCommand};
use dropbear_engine::camera::Camera;
use dropbear_engine::entity::{EntityTransform, MeshRenderer};
use glam::{DVec3};
use hecs::World;
use std::ffi::{CStr, c_char};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_entity(
    label: *const c_char,
    world_ptr: *const World,
    out_entity: *mut i64,
) -> i32 {
    if label.is_null() || world_ptr.is_null() || out_entity.is_null() {
        eprintln!("[dropbear_get_entity] [ERROR] received null pointer");
        return -1;
    }

    let world = unsafe { &*world_ptr };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("[dropbear_get_entity] [ERROR] invalid UTF-8 in label");
            return -108;
        }
    };

    for (id, entity_label) in world.query::<&Label>().iter() {
        if entity_label.as_str() == label_str {
            unsafe { *out_entity = id.id() as i64 };
            return 0;
        }
    }

    eprintln!(
        "[dropbear_get_entity] [ERROR] Entity with label '{}' not found",
        label_str
    );
    -3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_world_transform(
    world_ptr: *const World,
    entity_id: i64,
    out_transform: *mut NativeTransform,
) -> i32 {
    if world_ptr.is_null() || out_transform.is_null() {
        eprintln!("[dropbear_get_world_transform] [ERROR] Null pointer received");
        return -1;
    }

    let world = unsafe { &*world_ptr };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    match world.query_one::<&EntityTransform>(entity) {
        Ok(mut q) => {
            if let Some(transform) = q.get() {
                let transform = transform.world();
                unsafe {
                    (*out_transform).position_x = transform.position.x;
                    (*out_transform).position_y = transform.position.y;
                    (*out_transform).position_z = transform.position.z;
                    (*out_transform).rotation_x = transform.rotation.x;
                    (*out_transform).rotation_y = transform.rotation.y;
                    (*out_transform).rotation_z = transform.rotation.z;
                    (*out_transform).rotation_w = transform.rotation.w;
                    (*out_transform).scale_x = transform.scale.x;
                    (*out_transform).scale_y = transform.scale.y;
                    (*out_transform).scale_z = transform.scale.z;
                }
                0
            } else {
                eprintln!(
                    "[dropbear_get_transform] [ERROR] Entity has no WorldTransform component"
                );
                DropbearNativeError::NoSuchComponent as i32
            }
        }
        Err(_) => {
            eprintln!("[dropbear_get_transform] [ERROR] Failed to query entity");
            DropbearNativeError::QueryFailed as i32
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_local_transform(
    world_ptr: *const World,
    entity_id: i64,
    out_transform: *mut NativeTransform,
) -> i32 {
    if world_ptr.is_null() || out_transform.is_null() {
        eprintln!("[dropbear_get_local_transform] [ERROR] Null pointer received");
        return DropbearNativeError::NullPointer as i32;
    }

    let world = unsafe { &*world_ptr };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    match world.query_one::<&EntityTransform>(entity) {
        Ok(mut q) => {
            if let Some(transform) = q.get() {
                let transform = transform.local();
                unsafe {
                    (*out_transform).position_x = transform.position.x;
                    (*out_transform).position_y = transform.position.y;
                    (*out_transform).position_z = transform.position.z;
                    (*out_transform).rotation_x = transform.rotation.x;
                    (*out_transform).rotation_y = transform.rotation.y;
                    (*out_transform).rotation_z = transform.rotation.z;
                    (*out_transform).rotation_w = transform.rotation.w;
                    (*out_transform).scale_x = transform.scale.x;
                    (*out_transform).scale_y = transform.scale.y;
                    (*out_transform).scale_z = transform.scale.z;
                }
                0
            } else {
                eprintln!(
                    "[dropbear_get_local_transform] [ERROR] Entity has no LocalTransform component"
                );
                DropbearNativeError::NoSuchComponent as i32
            }
        }
        Err(_) => {
            eprintln!("[dropbear_get_local_transform] [ERROR] Failed to query entity");
            DropbearNativeError::QueryFailed as i32
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_string_property(
    world_ptr: *const World,
    entity_handle: i64,
    label: *const c_char,
    out_value: *mut c_char,
    out_value_max_length: i32,
) -> i32 {
    if world_ptr.is_null() || label.is_null() || out_value.is_null() {
        eprintln!("[dropbear_get_string_property] [ERROR] Null pointer received");
        return -1;
    }

    let world = unsafe { &*world_ptr };
    let entity = unsafe { world.find_entity_from_id(entity_handle as u32) };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("[dropbear_get_string_property] [ERROR] Invalid UTF-8 in label");
            return -108;
        }
    };

    match world.query_one::<(&MeshRenderer, &ModelProperties)>(entity) {
        Ok(mut q) => {
            if let Some((_, props)) = q.get() {
                if let Some(Value::String(val)) = props.get_property(label_str) {
                    let bytes = val.as_bytes();
                    let copy_len = std::cmp::min(bytes.len(), (out_value_max_length - 1) as usize);
                    unsafe {
                        std::ptr::copy_nonoverlapping(
                            bytes.as_ptr(),
                            out_value as *mut u8,
                            copy_len,
                        );
                        *out_value.add(copy_len) = 0; // null terminator
                    }
                    0
                } else {
                    eprintln!(
                        "[dropbear_get_string_property] [WARN] Property not found or wrong type"
                    );
                    -3
                }
            } else {
                -4
            }
        }
        Err(_) => {
            eprintln!("[dropbear_get_string_property] [ERROR] Failed to query entity");
            -2
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_int_property(
    world_ptr: *const World,
    entity_handle: i64,
    label: *const c_char,
    out_value: *mut i32,
) -> i32 {
    if world_ptr.is_null() || label.is_null() || out_value.is_null() {
        eprintln!("[dropbear_get_int_property] [ERROR] Null pointer received");
        return -1;
    }

    let world = unsafe { &*world_ptr };
    let entity = unsafe { world.find_entity_from_id(entity_handle as u32) };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s,
        Err(_) => return -108,
    };

    match world.query_one::<(&MeshRenderer, &ModelProperties)>(entity) {
        Ok(mut q) => {
            if let Some((_, props)) = q.get() {
                if let Some(Value::Int(val)) = props.get_property(label_str) {
                    unsafe { *out_value = *val as i32 };
                    0
                } else {
                    -3
                }
            } else {
                -4
            }
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_long_property(
    world_ptr: *const World,
    entity_handle: i64,
    label: *const c_char,
    out_value: *mut i64,
) -> i32 {
    if world_ptr.is_null() || label.is_null() || out_value.is_null() {
        return -1;
    }

    let world = unsafe { &*world_ptr };
    let entity = unsafe { world.find_entity_from_id(entity_handle as u32) };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s,
        Err(_) => return -108,
    };

    match world.query_one::<(&MeshRenderer, &ModelProperties)>(entity) {
        Ok(mut q) => {
            if let Some((_, props)) = q.get() {
                if let Some(Value::Int(val)) = props.get_property(label_str) {
                    unsafe { *out_value = *val };
                    0
                } else {
                    -3
                }
            } else {
                -4
            }
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_float_property(
    world_ptr: *const World,
    entity_handle: i64,
    label: *const c_char,
    out_value: *mut f32,
) -> i32 {
    if world_ptr.is_null() || label.is_null() || out_value.is_null() {
        return -1;
    }

    let world = unsafe { &*world_ptr };
    let entity = unsafe { world.find_entity_from_id(entity_handle as u32) };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s,
        Err(_) => return -108,
    };

    match world.query_one::<(&MeshRenderer, &ModelProperties)>(entity) {
        Ok(mut q) => {
            if let Some((_, props)) = q.get() {
                if let Some(Value::Float(val)) = props.get_property(label_str) {
                    unsafe { *out_value = *val as f32 };
                    0
                } else {
                    -3
                }
            } else {
                -4
            }
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_double_property(
    world_ptr: *const World,
    entity_handle: i64,
    label: *const c_char,
    out_value: *mut f64,
) -> i32 {
    if world_ptr.is_null() || label.is_null() || out_value.is_null() {
        return -1;
    }

    let world = unsafe { &*world_ptr };
    let entity = unsafe { world.find_entity_from_id(entity_handle as u32) };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s,
        Err(_) => return -108,
    };

    match world.query_one::<(&MeshRenderer, &ModelProperties)>(entity) {
        Ok(mut q) => {
            if let Some((_, props)) = q.get() {
                if let Some(Value::Float(val)) = props.get_property(label_str) {
                    unsafe { *out_value = *val };
                    0
                } else {
                    -3
                }
            } else {
                -4
            }
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_bool_property(
    world_ptr: *const World,
    entity_handle: i64,
    label: *const c_char,
    out_value: *mut i32,
) -> i32 {
    if world_ptr.is_null() || label.is_null() || out_value.is_null() {
        return -1;
    }

    let world = unsafe { &*world_ptr };
    let entity = unsafe { world.find_entity_from_id(entity_handle as u32) };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s,
        Err(_) => return -108,
    };

    match world.query_one::<(&MeshRenderer, &ModelProperties)>(entity) {
        Ok(mut q) => {
            if let Some((_, props)) = q.get() {
                if let Some(Value::Bool(val)) = props.get_property(label_str) {
                    unsafe { *out_value = if *val { 1 } else { 0 } };
                    0
                } else {
                    -3
                }
            } else {
                -4
            }
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_vec3_property(
    world_ptr: *const World,
    entity_handle: i64,
    label: *const c_char,
    out_x: *mut f32,
    out_y: *mut f32,
    out_z: *mut f32,
) -> i32 {
    if world_ptr.is_null()
        || label.is_null()
        || out_x.is_null()
        || out_y.is_null()
        || out_z.is_null()
    {
        return -1;
    }

    let world = unsafe { &*world_ptr };
    let entity = unsafe { world.find_entity_from_id(entity_handle as u32) };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s,
        Err(_) => return -108,
    };

    match world.query_one::<(&MeshRenderer, &ModelProperties)>(entity) {
        Ok(mut q) => {
            if let Some((_, props)) = q.get() {
                if let Some(Value::Vec3([x, y, z])) = props.get_property(label_str) {
                    unsafe {
                        *out_x = *x;
                        *out_y = *y;
                        *out_z = *z;
                    }
                    0
                } else {
                    -3
                }
            } else {
                -4
            }
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_set_string_property(
    world_ptr: *const World,
    entity_handle: i64,
    label: *const c_char,
    value: *const c_char,
) -> i32 {
    if world_ptr.is_null() || label.is_null() || value.is_null() {
        return -1;
    }

    let world = unsafe { &mut *(world_ptr as *mut World) };
    let entity = unsafe { world.find_entity_from_id(entity_handle as u32) };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return -108,
    };

    let value_str = match unsafe { CStr::from_ptr(value) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return -108,
    };

    match world.query_one_mut::<(&MeshRenderer, &mut ModelProperties)>(entity) {
        Ok((_, props)) => {
            props.set_property(label_str, Value::String(value_str));
            0
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_set_int_property(
    world_ptr: *const World,
    entity_handle: i64,
    label: *const c_char,
    value: i32,
) -> i32 {
    if world_ptr.is_null() || label.is_null() {
        return -1;
    }

    let world = unsafe { &mut *(world_ptr as *mut World) };
    let entity = unsafe { world.find_entity_from_id(entity_handle as u32) };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return -108,
    };

    match world.query_one_mut::<(&MeshRenderer, &mut ModelProperties)>(entity) {
        Ok((_, props)) => {
            props.set_property(label_str, Value::Int(value as i64));
            0
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_set_long_property(
    world_ptr: *const World,
    entity_handle: i64,
    label: *const c_char,
    value: i64,
) -> i32 {
    if world_ptr.is_null() || label.is_null() {
        return -1;
    }

    let world = unsafe { &mut *(world_ptr as *mut World) };
    let entity = unsafe { world.find_entity_from_id(entity_handle as u32) };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return -108,
    };

    match world.query_one_mut::<(&MeshRenderer, &mut ModelProperties)>(entity) {
        Ok((_, props)) => {
            props.set_property(label_str, Value::Int(value));
            0
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_set_float_property(
    world_ptr: *const World,
    entity_handle: i64,
    label: *const c_char,
    value: f32,
) -> i32 {
    if world_ptr.is_null() || label.is_null() {
        return -1;
    }

    let world = unsafe { &mut *(world_ptr as *mut World) };
    let entity = unsafe { world.find_entity_from_id(entity_handle as u32) };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return -108,
    };

    match world.query_one_mut::<(&MeshRenderer, &mut ModelProperties)>(entity) {
        Ok((_, props)) => {
            props.set_property(label_str, Value::Float(value as f64));
            0
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_set_double_property(
    world_ptr: *const World,
    entity_handle: i64,
    label: *const c_char,
    value: f64,
) -> i32 {
    if world_ptr.is_null() || label.is_null() {
        return -1;
    }

    let world = unsafe { &mut *(world_ptr as *mut World) };
    let entity = unsafe { world.find_entity_from_id(entity_handle as u32) };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return -108,
    };

    match world.query_one_mut::<(&MeshRenderer, &mut ModelProperties)>(entity) {
        Ok((_, props)) => {
            props.set_property(label_str, Value::Float(value));
            0
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_set_bool_property(
    world_ptr: *const World,
    entity_handle: i64,
    label: *const c_char,
    value: i32,
) -> i32 {
    if world_ptr.is_null() || label.is_null() {
        return -1;
    }

    let world = unsafe { &mut *(world_ptr as *mut World) };
    let entity = unsafe { world.find_entity_from_id(entity_handle as u32) };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return -108,
    };

    match world.query_one_mut::<(&MeshRenderer, &mut ModelProperties)>(entity) {
        Ok((_, props)) => {
            props.set_property(label_str, Value::Bool(value != 0));
            0
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_set_vec3_property(
    world_ptr: *const World,
    entity_handle: i64,
    label: *const c_char,
    x: f32,
    y: f32,
    z: f32,
) -> i32 {
    if world_ptr.is_null() || label.is_null() {
        return -1;
    }

    let world = unsafe { &mut *(world_ptr as *mut World) };
    let entity = unsafe { world.find_entity_from_id(entity_handle as u32) };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s.to_string(),
        Err(_) => return -108,
    };

    match world.query_one_mut::<(&MeshRenderer, &mut ModelProperties)>(entity) {
        Ok((_, props)) => {
            props.set_property(label_str, Value::Vec3([x, y, z]));
            0
        }
        Err(_) => -2,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_print_input_state(input_state_ptr: InputStatePtr) {
    if input_state_ptr.is_null() {
        eprintln!("[dropbear_print_input_state] [ERROR] Input state pointer is null");
        return;
    }

    let input_state = unsafe { &*input_state_ptr };
    println!("{:#?}", input_state);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_is_key_pressed(
    input_state_ptr: InputStatePtr,
    keycode: i32,
    out_value: *mut i32,
) -> i32 {
    if input_state_ptr.is_null() || out_value.is_null() {
        return -1;
    }

    let input = unsafe { &*input_state_ptr };

    match keycode_from_ordinal(keycode) {
        Some(k) => {
            let is_pressed = input.pressed_keys.contains(&k);
            unsafe { *out_value = if is_pressed { 1 } else { 0 } };
            0
        }
        None => {
            eprintln!("[dropbear_is_key_pressed] [WARN] Invalid keycode");
            unsafe { *out_value = 0 };
            0
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_mouse_position(
    input_state_ptr: InputStatePtr,
    out_x: *mut f32,
    out_y: *mut f32,
) -> i32 {
    if input_state_ptr.is_null() || out_x.is_null() || out_y.is_null() {
        return -1;
    }

    let input = unsafe { &*input_state_ptr };

    unsafe {
        *out_x = input.mouse_pos.0 as f32;
        *out_y = input.mouse_pos.1 as f32;
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_is_mouse_button_pressed(
    input_state_ptr: InputStatePtr,
    button_code: i32,
    out_pressed: *mut i32,
) -> i32 {
    if input_state_ptr.is_null() || out_pressed.is_null() {
        return -1;
    }

    let input = unsafe { &*input_state_ptr };

    match keycode_from_ordinal(button_code) {
        None => {
            eprintln!("[dropbear_is_mouse_button_pressed] [WARN] Invalid button code");
            unsafe { *out_pressed = 0 };
            return 0;
        }
        Some(key) => {
            if input.pressed_keys.contains(&key) {
                unsafe { *out_pressed = 1 };
            } else {
                unsafe { *out_pressed = 0 };
            }
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_mouse_delta(
    input_state_ptr: InputStatePtr,
    out_delta_x: *mut f32,
    out_delta_y: *mut f32,
) -> i32 {
    if input_state_ptr.is_null() || out_delta_x.is_null() || out_delta_y.is_null() {
        return -1;
    }

    let input = unsafe { &mut *(input_state_ptr as InputStatePtr) };

    if let Some(pos) = input.mouse_delta.take() {
        unsafe {
            *out_delta_x = pos.0 as f32;
            *out_delta_y = pos.1 as f32;
        }
    } else {
        unsafe {
            *out_delta_x = 0.0;
            *out_delta_y = 0.0;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_is_cursor_locked(
    input_state_ptr: InputStatePtr,
    out_locked: *mut i32,
) -> i32 {
    if input_state_ptr.is_null() || out_locked.is_null() {
        return -1;
    }

    let input = unsafe { &*input_state_ptr };

    unsafe { *out_locked = if input.is_cursor_locked { 1 } else { 0 } };

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_set_cursor_locked(
    queue_ptr: GraphicsPtr,
    input_state_ptr: InputStatePtr,
    locked: i32,
) -> i32 {
    if input_state_ptr.is_null() || queue_ptr.is_null() {
        return DropbearNativeError::NullPointer as i32;
    }

    let input = unsafe { &mut *(input_state_ptr as InputStatePtr) };

    let graphics = unsafe { &*(queue_ptr as GraphicsPtr) };

    input.is_cursor_locked = locked != 0;

    if graphics
        .send(GraphicsCommand::WindowCommand(WindowCommand::WindowGrab(
            input.is_cursor_locked,
        )))
        .is_err()
    {
        DropbearNativeError::SendError as i32
    } else {
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_camera(
    world_ptr: *const World,
    label: *const c_char,
    out_camera: *mut NativeCamera,
) -> i32 {
    if world_ptr.is_null() || label.is_null() || out_camera.is_null() {
        eprintln!("[dropbear_get_camera] [ERROR] Null pointer received");
        return -1;
    }

    let world = unsafe { &*world_ptr };

    let label_str = match unsafe { CStr::from_ptr(label) }.to_str() {
        Ok(s) => s,
        Err(_) => {
            eprintln!("[dropbear_get_camera] [ERROR] Invalid UTF-8 in label");
            return DropbearNativeError::InvalidUTF8 as i32;
        }
    };

    if let Some((id, (cam, comp))) = world
        .query::<(&Camera, &CameraComponent)>()
        .iter()
        .find(|(_, (cam, _))| cam.label == label_str)
    {
        if matches!(comp.camera_type, CameraType::Debug) {
            eprintln!("[dropbear_get_camera] [WARN] Querying a CameraType::Debug is illegal");
            return -5;
        }

        // We need to allocate the label string on the heap so it persists
        // The caller should be aware this needs to be managed
        let label_cstring = std::ffi::CString::new(cam.label.as_str()).unwrap();

        unsafe {
            (*out_camera).label = label_cstring.into_raw();
            (*out_camera).entity_id = id.id() as i64;

            (*out_camera).eye = Vector3D {
                x: cam.eye.x as f32,
                y: cam.eye.y as f32,
                z: cam.eye.z as f32,
            };

            (*out_camera).target = Vector3D {
                x: cam.target.x as f32,
                y: cam.target.y as f32,
                z: cam.target.z as f32,
            };

            (*out_camera).up = Vector3D {
                x: cam.up.x as f32,
                y: cam.up.y as f32,
                z: cam.up.z as f32,
            };

            (*out_camera).aspect = cam.aspect;
            (*out_camera).fov_y = cam.settings.fov_y;
            (*out_camera).znear = cam.znear;
            (*out_camera).zfar = cam.zfar;
            (*out_camera).yaw = cam.yaw;
            (*out_camera).pitch = cam.pitch;
            (*out_camera).speed = cam.settings.speed;
            (*out_camera).sensitivity = cam.settings.sensitivity;
        }

        return 0;
    }

    eprintln!(
        "[dropbear_get_camera] [ERROR] Camera with label '{}' not found",
        label_str
    );
    -3
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_attached_camera(
    world_ptr: *const World,
    id: i64,
    out_camera: *mut NativeCamera,
) -> i32 {
    if world_ptr.is_null() || out_camera.is_null() {
        eprintln!("[dropbear_get_attached_camera] [ERROR] Null pointer received");
        return -1;
    }

    let world = unsafe { &*world_ptr };
    let entity = unsafe { world.find_entity_from_id(id as u32) };

    match world.query_one::<(&Camera, &CameraComponent)>(entity) {
        Ok(mut q) => {
            if let Some((cam, comp)) = q.get() {
                if matches!(comp.camera_type, CameraType::Debug) {
                    eprintln!(
                        "[dropbear_get_attached_camera] [WARN] Querying a CameraType::Debug is illegal"
                    );
                    return -5;
                }

                let label_cstring = std::ffi::CString::new(cam.label.as_str()).unwrap();

                unsafe {
                    (*out_camera).label = label_cstring.into_raw();
                    (*out_camera).entity_id = id;

                    (*out_camera).eye = Vector3D {
                        x: cam.eye.x as f32,
                        y: cam.eye.y as f32,
                        z: cam.eye.z as f32,
                    };

                    (*out_camera).target = Vector3D {
                        x: cam.target.x as f32,
                        y: cam.target.y as f32,
                        z: cam.target.z as f32,
                    };

                    (*out_camera).up = Vector3D {
                        x: cam.up.x as f32,
                        y: cam.up.y as f32,
                        z: cam.up.z as f32,
                    };

                    (*out_camera).aspect = cam.aspect;
                    (*out_camera).fov_y = cam.settings.fov_y;
                    (*out_camera).znear = cam.znear;
                    (*out_camera).zfar = cam.zfar;
                    (*out_camera).yaw = cam.yaw;
                    (*out_camera).pitch = cam.pitch;
                    (*out_camera).speed = cam.settings.speed;
                    (*out_camera).sensitivity = cam.settings.sensitivity;
                }

                0
            } else {
                eprintln!("[dropbear_get_attached_camera] [ERROR] Entity has no Camera component");
                -4
            }
        }
        Err(_) => {
            eprintln!("[dropbear_get_attached_camera] [ERROR] Failed to query entity");
            -2
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_set_camera(
    world_ptr: *mut World,
    camera: *const NativeCamera,
) -> i32 {
    if world_ptr.is_null() || camera.is_null() {
        eprintln!("[dropbear_set_camera] [ERROR] Null pointer received");
        return -1;
    }

    let world = unsafe { &mut *(world_ptr) };
    let cam_data = unsafe { &*camera };

    let entity = unsafe { world.find_entity_from_id(cam_data.entity_id as u32) };

    match world.query_one_mut::<&mut Camera>(entity) {
        Ok(cam) => {
            cam.eye = DVec3::new(
                cam_data.eye.x as f64,
                cam_data.eye.y as f64,
                cam_data.eye.z as f64,
            );

            cam.target = DVec3::new(
                cam_data.target.x as f64,
                cam_data.target.y as f64,
                cam_data.target.z as f64,
            );

            cam.up = DVec3::new(
                cam_data.up.x as f64,
                cam_data.up.y as f64,
                cam_data.up.z as f64,
            );

            cam.aspect = cam_data.aspect;
            cam.settings.fov_y = cam_data.fov_y;
            cam.znear = cam_data.znear;
            cam.zfar = cam_data.zfar;
            cam.yaw = cam_data.yaw;
            cam.pitch = cam_data.pitch;
            cam.settings.speed = cam_data.speed;
            cam.settings.sensitivity = cam_data.sensitivity;

            0
        }
        Err(_) => {
            eprintln!("[dropbear_set_camera] [ERROR] Unable to query camera component");
            -2
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_last_mouse_pos(
    input_state_ptr: InputStatePtr,
    out_x: *mut f32,
    out_y: *mut f32,
) -> i32 {
    if input_state_ptr.is_null() || out_x.is_null() || out_y.is_null() {
        return -1;
    }

    let input = unsafe { &*input_state_ptr };

    if let Some(pos) = input.last_mouse_pos {
        unsafe {
            *out_x = pos.0 as f32;
            *out_y = pos.1 as f32;
        }
    } else {
        unsafe {
            *out_x = 0.0;
            *out_y = 0.0;
        }
    }

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_is_cursor_hidden(
    input_state_ptr: InputStatePtr,
    out_hidden: *mut i32,
) -> i32 {
    if input_state_ptr.is_null() || out_hidden.is_null() {
        return -1;
    }

    let input = unsafe { &*input_state_ptr };

    unsafe { *out_hidden = if input.is_cursor_hidden { 1 } else { 0 } };

    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_set_cursor_hidden(
    queue_ptr: GraphicsPtr,
    input_state_ptr: InputStatePtr,
    hidden: i32,
) -> i32 {
    if input_state_ptr.is_null() || queue_ptr.is_null() {
        return DropbearNativeError::NullPointer as i32;
    }

    let input = unsafe { &mut *(input_state_ptr as InputStatePtr) };
    let graphics = unsafe { &*(queue_ptr as GraphicsPtr) };
    input.is_cursor_hidden = hidden != 0;

    if graphics
        .send(GraphicsCommand::WindowCommand(WindowCommand::HideCursor(
            input.is_cursor_hidden,
        )))
        .is_err()
    {
        DropbearNativeError::SendError as i32
    } else {
        DropbearNativeError::Success as i32
    }
}
