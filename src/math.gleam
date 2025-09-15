/// The generic type of an entity, containing a position, rotation and scale. 
pub type Transform {
    Transform (
        position: Vector3(Float),
        rotation: Quaternion(Float),
        scale: Vector3(Float),
    )
}

/// Creates a new transform
pub fn new_transform() -> Transform {
    Transform(
        position: zero_vector3f(),
        rotation: identity_quatf(),
        scale: zero_vector3f(),
    )
}

/// A type used to show 3 instances of a value. 
pub type Vector3(a) {
    Vector3(
        /// X value
        x: a,
        /// Y value
        y: a,
        /// Z value
        z: a,
    )
}

/// Creates a new Vector3(Float) of all 0.0. 
pub fn zero_vector3f() -> Vector3(Float) {
    Vector3(
        x: 0.0,
        y: 0.0,
        z: 0.0,
    )
}

pub type Quaternion(a) {
    Quaternion(
        w: a,
        x: a,
        y: a,
        z: a,
    )
}

/// Creates a new quaternion with 1.0 as the scale and 0.0 for the x, y and z. 
pub fn identity_quatf() -> Quaternion(Float) {
    Quaternion(
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    )
}