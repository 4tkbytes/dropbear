//! A module for dealing with the FFI between Swift dropbear-engine scripting API and 
//! the game engine in Rust.

use serde::{Deserialize, Serialize};
use winit::event::MouseButton;
use crate::input::InputState;
use winit::platform::scancode::PhysicalKeyExtScancode;

#[derive(Serialize, Deserialize)]
pub struct FFIInputState {
    pub keys_pressed: Vec<u32>,
    pub mouse_position: [f32; 2],
    pub mouse_delta: [f32; 2],
    pub mouse_buttons: Vec<u32>,
}

impl From<InputState> for FFIInputState {
    fn from(value: InputState) -> Self {

        let keys_pressed = value.pressed_keys.iter().map(|k| {
            k.to_scancode().unwrap()
        }).collect::<Vec<_>>();

        let mut mouse_buttons = Vec::new();
        for m in value.mouse_button {
            let r = match m {
                MouseButton::Left => 1u32,
                MouseButton::Right => 2u32,
                MouseButton::Middle => 3u32,
                _ => 0u32
            };
            mouse_buttons.push(r);
        }

        Self {
            keys_pressed,
            mouse_position: [value.mouse_pos.0 as f32, value.mouse_pos.1 as f32],
            mouse_delta: [value.mouse_delta.unwrap_or_default().0 as f32, value.mouse_delta.unwrap_or_default().1 as f32],
            mouse_buttons,
        }
    }
}

mod vec {
    use std::mem::ManuallyDrop;

    /// An expandable array of [`u32`], similar to a [`Vec`] but for C API's
    #[repr(C)]
    pub struct CArrayU32 {
        /// Pointer/Reference to the contents
        pub ptr: *const u32,
        /// Number of elements in vector
        pub len: usize,
        /// Maximum capacity of elements in vector
        pub cap: usize,
    }

    /// An expandable array of [`u8`], similar to a [`Vec`] but for C API's
    #[repr(C)]
    pub struct CArrayU8 {
        /// Pointer/Reference to the contents
        pub ptr: *const u8,
        /// Number of elements in vector
        pub len: usize,
        /// Maximum capacity of elements in vector
        pub cap: usize,
    }

    /// Creates a new [`CArrayU32`] from a vector (used for Rust)
    pub fn vec_into_carray_u32(v: Vec<u32>) -> CArrayU32 {
        let mut v = ManuallyDrop::new(v);
        CArrayU32 { ptr: v.as_mut_ptr(), len: v.len(), cap: v.capacity() }
    }

    /// Created a nw [`CArrayU8`] from a vector (used for Rust)
    pub fn vec_into_carray_u8(v: Vec<u8>) -> CArrayU8 {
        let mut v = ManuallyDrop::new(v);
        CArrayU8 { ptr: v.as_mut_ptr(), len: v.len(), cap: v.capacity() }
    }

    /// Create a new empty CArrayU32
    #[unsafe(no_mangle)]
    pub extern "C" fn new_array_u32() -> CArrayU32 {
        vec_into_carray_u32(Vec::new())
    }

    /// Create a new empty CArrayU8
    #[unsafe(no_mangle)]
    pub extern "C" fn new_array_u8() -> CArrayU8 {
        vec_into_carray_u8(Vec::new())
    }

    /// Frees the vector from memory.
    ///
    /// Since Rust is unable to manage an FFI from outside the language, this function is required.
    ///
    /// This function is exposed under the name of `free_array_u32`
    #[unsafe(no_mangle)]
    pub extern "C" fn free_array_u32(arr: CArrayU32) {
        unsafe { Vec::from_raw_parts(arr.ptr as *mut u32, arr.len, arr.cap); }
    }

    /// Frees the vector from memory.
    ///
    /// Since Rust is unable to manage an FFI from outside the language, this function is required.
    ///
    /// This function is exposed under the name of `free_array_u8`
    #[unsafe(no_mangle)]
    pub extern "C" fn free_array_u8(arr: CArrayU8) {
        unsafe { Vec::from_raw_parts(arr.ptr as *mut u8, arr.len, arr.cap); }
    }
}