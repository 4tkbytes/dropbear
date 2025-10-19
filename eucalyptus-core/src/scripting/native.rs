//! Deals with Kotlin/Native library loading for different platforms.
#![allow(clippy::missing_safety_doc)]

pub mod sig;
mod exports;

use crate::ptr::{InputStatePtr, WorldPtr};
use crate::scripting::native::sig::{DestroyAll, DestroyTagged, Init, LoadTagged, UpdateAll, UpdateTagged};
use libloading::{Library, Symbol};
use std::ffi::CString;
use std::path::Path;

pub struct NativeLibrary {
    #[allow(dead_code)]
    library: Library,
    init_fn: Symbol<'static, Init>,
    load_systems_fn: Symbol<'static, LoadTagged>,
    update_all_fn: Symbol<'static, UpdateAll>,
    update_tag_fn: Symbol<'static, UpdateTagged>,
    destroy_all_fn: Symbol<'static, DestroyAll>,
    destroy_tagged_fn: Symbol<'static, DestroyTagged>,
}

impl NativeLibrary {
    pub fn new(lib_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let lib_path = lib_path.as_ref();
        unsafe {
            let library: Library = Library::new(lib_path)?;

            let init_fn: Symbol<'static, Init> = std::mem::transmute(
                library.get::<Init>(b"dropbear_init\0")?
            );
            let load_systems_fn: Symbol<'static, LoadTagged> = std::mem::transmute(
                library.get::<LoadTagged>(b"dropbear_load_systems\0")?
            );
            let update_all_fn: Symbol<'static, UpdateAll> = std::mem::transmute(
                library.get::<UpdateAll>(b"dropbear_update_all\0")?
            );
            let update_tag_fn: Symbol<'static, UpdateTagged> = std::mem::transmute(
                library.get::<UpdateTagged>(b"dropbear_update_tagged\0")?
            );
            let destroy_all_fn: Symbol<'static, DestroyAll> = std::mem::transmute(
                library.get::<DestroyAll>(b"dropbear_destroy_all\0")?
            );
            let destroy_tagged_fn: Symbol<'static, DestroyTagged> = std::mem::transmute(
                library.get::<DestroyTagged>(b"dropbear_destroy_tagged\0")?
            );

            Ok(Self {
                library,
                init_fn,
                load_systems_fn,
                update_all_fn,
                update_tag_fn,
                destroy_all_fn,
                destroy_tagged_fn,
            })
        }
    }

    pub fn init(&mut self, world_ptr: WorldPtr, input_state_ptr: InputStatePtr) -> anyhow::Result<()> {
        unsafe {
            let result = (self.init_fn)(world_ptr, input_state_ptr);
            if result != 0 {
                anyhow::bail!("Init function failed with code: {}", result);
            }
            Ok(())
        }
    }

    pub fn load_systems(&mut self, tag: String) -> anyhow::Result<()> {
        unsafe {
            let c_string: CString = CString::new(tag)?;
            let result = (self.load_systems_fn)(c_string.as_ptr());
            if result != 0 {
                anyhow::bail!("Load systems failed with code: {}", result);
            }
            Ok(())
        }
    }

    pub fn update_all(&mut self, dt: f32) -> anyhow::Result<()> {
        unsafe {
            (self.update_all_fn)(dt);
            Ok(())
        }
    }

    pub fn update_tagged(&mut self, tag: String, dt: f32) -> anyhow::Result<()> {
        unsafe {
            let c_string: CString = CString::new(tag)?;
            (self.update_tag_fn)(c_string.as_ptr(), dt);
            Ok(())
        }
    }

    pub fn destroy_all(&mut self) -> anyhow::Result<()> {
        unsafe {
            (self.destroy_all_fn)();
            Ok(())
        }
    }

    pub fn destroy_tagged(&mut self, tag: String) -> anyhow::Result<()> {
        unsafe {
            let c_string: CString = CString::new(tag)?;
            (self.destroy_tagged_fn)(c_string.as_ptr());
            Ok(())
        }
    }
}

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
    /// The number `1274` comes from the total sum of the word "UnknownError" in decimal
    UnknownError = -1274,
}