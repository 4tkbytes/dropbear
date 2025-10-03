#![allow(non_snake_case)]
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use jni::objects::{JClass, JObject};
use jni::sys::{jdouble, jlong, jboolean};
use parking_lot::RwLock;
use once_cell::sync::Lazy;
use dropbear_engine::entity::Transform;
use glam::{DVec3, DQuat};
use hecs::{Entity, World};
use winit::keyboard::KeyCode;

// Thread-safe global context using RwLock instead of Mutex for better performance
static CURRENT_ENTITY: Lazy<RwLock<Option<Entity>>> = Lazy::new(|| RwLock::new(None));
static CURRENT_WORLD: Lazy<RwLock<Option<usize>>> = Lazy::new(|| RwLock::new(None)); // Store as usize
static CURRENT_POSITION: Lazy<RwLock<DVec3>> = Lazy::new(|| RwLock::new(DVec3::ZERO));
static CURRENT_ROTATION: Lazy<RwLock<DQuat>> = Lazy::new(|| RwLock::new(DQuat::IDENTITY));
static CURRENT_SCALE: Lazy<RwLock<DVec3>> = Lazy::new(|| RwLock::new(DVec3::ONE));
static CURRENT_MOUSE: Lazy<RwLock<(f64, f64)>> = Lazy::new(|| RwLock::new((0.0, 0.0)));
static CURRENT_KEYS: Lazy<RwLock<Vec<i64>>> = Lazy::new(|| RwLock::new(Vec::new()));

/// A dropbear wrapper for Java Virtual Machine (JVM) based functions
pub(crate) struct JavaContext {
    jvm: JavaVM,
}

impl JavaContext {
    /// Creates a new [`JavaContext`]
    pub fn new() -> anyhow::Result<Self> {
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option("-Djava.class.path=./build/libs/dropbear-1.0-SNAPSHOT.jar")
            .build()?;

        let jvm = JavaVM::new(jvm_args)?;
        log::info!("Initialised JVM");
        Ok(Self {
            jvm
        })
    }

    /// Get a JNI environment for the current thread
    pub fn get_env(&self) -> anyhow::Result<jni::AttachGuard<'_>> {
        Ok(self.jvm.attach_current_thread()?)
    }

    /// Call a script's load method
    pub fn call_script_load(&self, script_instance: JObject) -> anyhow::Result<()> {
        let mut env = self.get_env()?;
        env.call_method(script_instance, "load", "()V", &[])?;
        Ok(())
    }

    /// Call a script's update method
    pub fn call_script_update(&self, script_instance: JObject) -> anyhow::Result<()> {
        let mut env = self.get_env()?;
        env.call_method(script_instance, "update", "()V", &[])?;
        Ok(())
    }

    /// Load a script class and create an instance
    pub fn load_script_class(&self, class_name: &str) -> anyhow::Result<JObject<'_>> {
        let mut env = self.get_env()?;
        
        // Find the class
        let class = env.find_class(class_name)?;
        
        // Create an instance
        let instance = env.new_object(class, "()V", &[])?;
        
        Ok(instance)
    }
}

/// Update context from Rust side before calling scripts
pub fn update_script_context(entity: Entity, world: &World, mouse_pos: (f64, f64), pressed_keys: &[KeyCode]) {
    // Store entity
    *CURRENT_ENTITY.write() = Some(entity);
    
    // Store world pointer as usize
    *CURRENT_WORLD.write() = Some(world as *const World as usize);
    
    // Read transform from world
    if let Ok(transform) = world.get::<&Transform>(entity) {
        *CURRENT_POSITION.write() = transform.position;
        *CURRENT_ROTATION.write() = transform.rotation;
        *CURRENT_SCALE.write() = transform.scale;
    }
    
    // Store input state
    *CURRENT_MOUSE.write() = mouse_pos;
    *CURRENT_KEYS.write() = pressed_keys.iter().map(|k| keycode_to_i64(k)).collect();
}

/// Sync changes back to world after script execution
pub fn sync_transform_to_world(world: &mut World) {
    if let Some(entity) = *CURRENT_ENTITY.read() {
        if let Ok(mut transform) = world.get::<&mut Transform>(entity) {
            transform.position = *CURRENT_POSITION.read();
            transform.rotation = *CURRENT_ROTATION.read();
            transform.scale = *CURRENT_SCALE.read();
        }
    }
}

// =============================================================================
// JNI Native Functions - Called from Kotlin
// =============================================================================

/// Get the transform position of the current entity
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_getPositionX(
    _env: JNIEnv,
    _class: JClass,
) -> jdouble {
    CURRENT_POSITION.read().x
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_getPositionY(
    _env: JNIEnv,
    _class: JClass,
) -> jdouble {
    CURRENT_POSITION.read().y
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_getPositionZ(
    _env: JNIEnv,
    _class: JClass,
) -> jdouble {
    CURRENT_POSITION.read().z
}

/// Set the transform position of the current entity
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_setPosition(
    _env: JNIEnv,
    _class: JClass,
    x: jdouble,
    y: jdouble,
    z: jdouble,
) {
    *CURRENT_POSITION.write() = DVec3::new(x, y, z);
}

/// Get rotation quaternion components
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_getRotationX(
    _env: JNIEnv,
    _class: JClass,
) -> jdouble {
    CURRENT_ROTATION.read().x
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_getRotationY(
    _env: JNIEnv,
    _class: JClass,
) -> jdouble {
    CURRENT_ROTATION.read().y
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_getRotationZ(
    _env: JNIEnv,
    _class: JClass,
) -> jdouble {
    CURRENT_ROTATION.read().z
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_getRotationW(
    _env: JNIEnv,
    _class: JClass,
) -> jdouble {
    CURRENT_ROTATION.read().w
}

/// Set rotation quaternion
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_setRotation(
    _env: JNIEnv,
    _class: JClass,
    x: jdouble,
    y: jdouble,
    z: jdouble,
    w: jdouble,
) {
    *CURRENT_ROTATION.write() = DQuat::from_xyzw(x, y, z, w);
}

/// Get scale components
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_getScaleX(
    _env: JNIEnv,
    _class: JClass,
) -> jdouble {
    CURRENT_SCALE.read().x
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_getScaleY(
    _env: JNIEnv,
    _class: JClass,
) -> jdouble {
    CURRENT_SCALE.read().y
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_getScaleZ(
    _env: JNIEnv,
    _class: JClass,
) -> jdouble {
    CURRENT_SCALE.read().z
}

/// Set scale
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_setScale(
    _env: JNIEnv,
    _class: JClass,
    x: jdouble,
    y: jdouble,
    z: jdouble,
) {
    *CURRENT_SCALE.write() = DVec3::new(x, y, z);
}

// =============================================================================
// Input System JNI Functions
// =============================================================================

/// Check if a key is pressed
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_isKeyPressed(
    _env: JNIEnv,
    _class: JClass,
    keycode: jlong,
) -> jboolean {
    let keys = CURRENT_KEYS.read();
    keys.contains(&keycode) as jboolean
}

/// Get mouse X position
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_getMouseX(
    _env: JNIEnv,
    _class: JClass,
) -> jdouble {
    CURRENT_MOUSE.read().0
}

/// Get mouse Y position
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_getMouseY(
    _env: JNIEnv,
    _class: JClass,
) -> jdouble {
    CURRENT_MOUSE.read().1
}

/// Helper function to convert KeyCode to i64
fn keycode_to_i64(key: &KeyCode) -> i64 {
    match key {
        KeyCode::KeyA => 65,
        KeyCode::KeyB => 66,
        KeyCode::KeyC => 67,
        KeyCode::KeyD => 68,
        KeyCode::KeyE => 69,
        KeyCode::KeyF => 70,
        KeyCode::KeyG => 71,
        KeyCode::KeyH => 72,
        KeyCode::KeyI => 73,
        KeyCode::KeyJ => 74,
        KeyCode::KeyK => 75,
        KeyCode::KeyL => 76,
        KeyCode::KeyM => 77,
        KeyCode::KeyN => 78,
        KeyCode::KeyO => 79,
        KeyCode::KeyP => 80,
        KeyCode::KeyQ => 81,
        KeyCode::KeyR => 82,
        KeyCode::KeyS => 83,
        KeyCode::KeyT => 84,
        KeyCode::KeyU => 85,
        KeyCode::KeyV => 86,
        KeyCode::KeyW => 87,
        KeyCode::KeyX => 88,
        KeyCode::KeyY => 89,
        KeyCode::KeyZ => 90,
        KeyCode::ArrowLeft => 37,
        KeyCode::ArrowUp => 38,
        KeyCode::ArrowRight => 39,
        KeyCode::ArrowDown => 40,
        KeyCode::Space => 32,
        KeyCode::ShiftLeft | KeyCode::ShiftRight => 16,
        KeyCode::ControlLeft | KeyCode::ControlRight => 17,
        KeyCode::Escape => 27,
        _ => 0,
    }
}

