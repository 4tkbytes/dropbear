use std::ffi::{c_char, CStr};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::platform::scancode::PhysicalKeyExtScancode;
use dropbear_engine::entity::{AdoptedEntity, Transform};
use crate::ptr::InputStatePtr;
use crate::scripting::native::DropbearNativeError;
use crate::utils::keycode_from_ordinal;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_entity(
    label: *const c_char,
    world_ptr: *const hecs::World,
    out_entity: *mut i64,
) -> i32 {
    unsafe {
        if label.is_null() || world_ptr.is_null() || out_entity.is_null() {
            println!("[dropbear_get_entity] [ERROR] received null pointer");
            return -1;
        }

        let world =  &*world_ptr;

        let label_str = match CStr::from_ptr(label).to_str() {
            Ok(s) => s,
            Err(_) => {
                println!("[dropbear_get_entity] [ERROR] invalid UTF-8 in label");
                return -108;
            }
        };

        let mut hit: bool = false;

        for (id, entity) in world.query::<&AdoptedEntity>().iter() {
            if entity.model.label == label_str {
                #[allow(unused_assignments)]
                { hit = true; }
                *out_entity = id.id() as i64;
                log::debug!("Found entity with label: {:?}", label_str);
                return 0;
            }
        }

        if !hit {
            println!("[dropbear_get_entity] [ERROR] Entity with label '{:?}' not found", label_str);
            -3
        } else {
            DropbearNativeError::UnknownError as i32
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_transform(
    world_ptr: *const hecs::World,
    entity_id: i64,
    out_transform: *mut Transform,
) -> i32 {
    if world_ptr.is_null() {
        eprintln!("[dropbear_get_transform] [ERROR] World pointer is null");
        return DropbearNativeError::NullPointer as i32;
    }

    if out_transform.is_null() {
        eprintln!("[dropbear_get_transform] [ERROR] Output transform pointer is null");
        return DropbearNativeError::NullPointer as i32;
    }

    let world = unsafe { &*world_ptr };

    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    match world.query_one::<&Transform>(entity) {
        Ok(mut q) => {
            if let Some(transform) = q.get() {
                unsafe { *out_transform = *transform };
                DropbearNativeError::Success as i32
            } else {
                eprintln!("[dropbear_get_transform] [ERROR] Entity has no Transform component");
                -4
            }
        }
        Err(_) => {
            eprintln!("[dropbear_get_transform] [ERROR] Failed to query entity for Transform component");
            -2
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_set_transform(
    world_ptr: *mut hecs::World,
    entity_id: i64,
    transform: Transform,
) -> i32 {
    if world_ptr.is_null() {
        eprintln!("[dropbear_get_transform] [ERROR] World pointer is null");
        return DropbearNativeError::NullPointer as i32;
    }

    let world = unsafe { &mut *world_ptr };

    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    let result = world.insert_one(entity, transform);

    match result {
        Ok(_) => 0,
        Err(_) => {
            eprintln!("[dropbear_set_transform] [ERROR] Failed to insert transform component");
            -6
        },
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_print_input_state(
    input_state_ptr: InputStatePtr,
) {
    if input_state_ptr.is_null() {
        eprintln!("[dropbear_print_inputstate] [ERROR] Input state pointer is null");
        return;
    }

    let input_state = unsafe { &*input_state_ptr };
    println!("{:#?}", input_state);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_is_key_pressed(
    input_state_ptr: InputStatePtr,
    key: i32,
    out_is_pressed: *mut bool,
) -> i32 {
    if input_state_ptr.is_null() {
        eprintln!("[dropbear_is_key_pressed] [ERROR] Input state pointer is null");
        unsafe { *out_is_pressed = false };
        return DropbearNativeError::NullPointer as i32;
    }

    let input = unsafe { &*input_state_ptr };

    match keycode_from_ordinal(key) {
        Some(k) => {
            println!("[dropbear_is_key_pressed] [DEBUG] Keycode: {:?}", k);
            if input.pressed_keys.contains(&k) {
                true.into()
            } else {
                false.into()
            }
        }
        None => {
            println!("[dropbear_is_key_pressed] [WARN] Ordinal keycode is invalid");
            false.into()
        }
    }
}

