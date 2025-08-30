use gleek_proc_macro::{gleek_export, gleek_impl};

#[gleek_export]
/// A struct used to store a vector of 3 values. Can be useful for position. 
pub struct Vector3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

#[gleek_impl]
impl Vector3 {
    /// Creates a new instance of a Vector3
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            x, y, z
        }
    }
}