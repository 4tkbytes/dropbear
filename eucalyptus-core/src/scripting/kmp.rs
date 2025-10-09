//! Deals with Kotlin/Native library loading for different platforms.

use std::ffi::{c_char, CStr};
use dropbear_engine::entity::AdoptedEntity;

/// Looks up an entity by its string label in the given world and writes the result to `out_entity`.
///
/// # Safety
///
/// - `label` must be a valid null-terminated C string.
/// - `world_ptr` must be a non-null pointer to a valid, initialized `hecs::World`
///   that is not being mutably aliased (i.e., no concurrent mutable access).
/// - `out_entity` must be a non-null pointer to a `uint64_t` (`u64`) that the caller owns.
/// - The `hecs::World` must outlive the duration of this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dropbear_get_entity(
    label: *const c_char,
    world_ptr: *const hecs::World,
    out_entity: *mut u64,
) -> i32 {
    if label.is_null() || world_ptr.is_null() || out_entity.is_null() {
        log::warn!("dropbear_get_entity: received null pointer");
        return -1;
    }

    let world = unsafe { &*world_ptr };

    let label_str = unsafe {
        match CStr::from_ptr(label).to_str() {
            Ok(s) => s,
            Err(_) => {
                log::warn!("dropbear_get_entity: invalid UTF-8 in label");
                return -1;
            }
        }
    };

    for (id, entity) in world.query::<&AdoptedEntity>().iter() {
        if entity.model.label == label_str {
            *out_entity = id.id() as u64;
            log::debug!("Found entity with label: {:?}", label_str);
            return 0;
        }
    }

    log::warn!("Entity with label '{:?}' not found", label_str);
    -1
}