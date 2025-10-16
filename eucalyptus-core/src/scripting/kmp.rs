//! Deals with Kotlin/Native library loading for different platforms.
#![allow(clippy::missing_safety_doc)]

use dropbear_engine::entity::{AdoptedEntity, Transform};
use std::ffi::{CStr, c_char};

/// Displays the types of errors that can be returned by the native library.
pub enum DropbearNativeError {
    Success = 0,
    NullPointer = -1,
    QueryFailed = -2,
    EntityNotFound = -3,
    NoSuchComponent = -4,
    NoSuchEntity = -5,
    WorldInsertError = -6,

    InvalidUTF8 = -108,
    /// A generic error when the library doesn't know what happened or cannot find a
    /// suitable error code.
    ///
    /// The number `1274` comes from the total sum of the word "UnknownError" into decimal
    UnknownError = -1274,
}

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
        return -1;
    }

    if out_transform.is_null() {
        eprintln!("[dropbear_get_transform] [ERROR] Output transform pointer is null");
        return -1;
    }

    let world = unsafe { &*world_ptr };

    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    match world.query_one::<&Transform>(entity) {
        Ok(mut q) => {
            if let Some(transform) = q.get() {
                unsafe { *out_transform = *transform };
                0
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

pub unsafe extern "C" fn dropbear_set_transform(
    world_ptr: *mut hecs::World,
    entity_id: i64,
    transform: Transform,
) -> i32 {
    if world_ptr.is_null() {
        eprintln!("[dropbear_get_transform] [ERROR] World pointer is null");
        return -1;
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

