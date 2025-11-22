use crate::ptr::{AssetRegistryPtr, GraphicsPtr, InputStatePtr, WorldPtr};
/// Different signatures for Native implementations
use std::ffi::c_char;

/// CName: `dropbear_init`
pub type Init = unsafe extern "C" fn(
    world: WorldPtr,
    input: InputStatePtr,
    graphics: GraphicsPtr,
    asset: AssetRegistryPtr,
) -> i32;
/// CName: `dropbear_load_tagged`
pub type LoadTagged = unsafe extern "C" fn(tag: *const c_char) -> i32;
/// CName: `dropbear_update_all`
pub type UpdateAll = unsafe extern "C" fn(dt: f32) -> i32;
/// CName: `dropbear_update_tagged`
pub type UpdateTagged = unsafe extern "C" fn(tag: *const c_char, dt: f32) -> i32;
/// CName: `dropbear_destroy_tagged`
pub type DestroyTagged = unsafe extern "C" fn(tag: *const c_char) -> i32;
/// CName: `dropbear_destroy_all`
pub type DestroyAll = unsafe extern "C" fn() -> i32;

/// CName: `dropbear_get_last_error_message`
pub type GetLastErrorMessage = unsafe extern "C" fn() -> *const c_char;
/// CName: `dropbear_set_last_error_message`
pub type SetLastErrorMessage = unsafe extern "C" fn(msg: *const c_char);
