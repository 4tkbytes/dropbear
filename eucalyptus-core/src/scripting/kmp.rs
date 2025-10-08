//! Deals with Kotlin/Native library loading for different platforms.

use std::ffi::c_char;
use hecs::World;
use dropbear_engine::entity::AdoptedEntity;

#[unsafe(no_mangle)]
pub extern "C" fn dropbear_get_entity(label: *const c_char, world_ptr: *const World) -> usize {
    if world_ptr.is_null() {
        log::debug!("World pointer is null");
        return 0;
    }

    unsafe {
        let world = &*world_ptr;

        for (id, entity) in world.query::<&AdoptedEntity>().iter() {
            if let Ok(label) = std::ffi::CStr::from_ptr(label).to_str()
                && entity.model.label == label
            {
                log::debug!("Found entity with label: {}", label);
                return id.id() as usize;
            }
        }
        log::warn!("Entity with label: {} not found", std::ffi::CStr::from_ptr(label).to_str().unwrap());
        0
    }
}