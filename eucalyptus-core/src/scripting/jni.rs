#![allow(non_snake_case)]
//! Deals with the Java Native Interface (JNI) with the help of the [`jni`] crate

pub mod exception;

use std::fs;
use dropbear_engine::entity::{AdoptedEntity, Transform};
use hecs::World;
use jni::objects::{GlobalRef, JClass, JObject, JString, JValue};
use jni::sys::{jclass, jlong};
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use std::path::{PathBuf};
use glam::{DQuat, DVec3};
use crate::{success, APP_INFO};
use sha2::{Digest, Sha256};
use crate::logging::{LOG_LEVEL};
use crate::ptr::WorldPtr;
use crate::scripting::jni::exception::get_exception_info;

const LIBRARY_PATH: &[u8] = include_bytes!("../../../build/libs/dropbear-1.0-SNAPSHOT-all.jar");

/// Provides a context for any eucalyptus-core JNI calls and JVM hosting.
pub struct JavaContext {
    pub(crate) jvm: JavaVM,
    dropbear_engine_class: Option<GlobalRef>,
    system_manager_instance: Option<GlobalRef>,
    pub(crate) jar_path: PathBuf,
}

impl JavaContext {
    /// Creates a new JVM instance
    pub fn new() -> anyhow::Result<Self> {
        let root = app_dirs2::app_root(app_dirs2::AppDataType::UserData, &APP_INFO)?;
        let deps = root.join("dependencies");
        let host_jar_filename = "dropbear-jvm-fat-1.0-SNAPSHOT.jar";
        let host_jar_path = deps.join(host_jar_filename);
        let hash_filename = format!("{}.sha256", host_jar_filename);
        let hash_file_path = deps.join(hash_filename);

        fs::create_dir_all(&deps)?;

        let embedded_jar_hash = {
            let mut hasher = Sha256::new();
            hasher.update(LIBRARY_PATH);
            format!("{:x}", hasher.finalize())
        };

        let stored_hash = fs::read_to_string(&hash_file_path).ok();

        let should_update = match stored_hash {
            Some(stored) => {
                if stored.trim() == embedded_jar_hash {
                    log::debug!("Host library JAR hash matches stored hash. No update needed.");
                    false
                } else {
                    log::info!("Host library JAR hash differs from stored hash. Update required.");
                    true
                }
            }
            None => {
                log::info!("Host library JAR hash file not found. Update required.");
                true
            }
        };

        if should_update {
            log::info!("Writing (or updating) Host library JAR to {:?}.", host_jar_path);
            fs::write(&host_jar_path, LIBRARY_PATH)?;
            log::info!("Host library JAR written to {:?}.", host_jar_path);

            fs::write(&hash_file_path, &embedded_jar_hash)?;
            log::debug!("Host library JAR hash written to {:?}.", hash_file_path);
        } else {
            log::debug!("Host library JAR at {:?} is up-to-date.", host_jar_path);
        }

        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option(format!(
                "-Djava.class.path={}",
                host_jar_path.display()
            ));

        #[cfg(feature = "editor")]
        let jvm_args = jvm_args
            .option(
                "-agentlib:jdwp=transport=dt_socket,server=y,suspend=n,address=*:6741"
            );

        let jvm_args = jvm_args.build()?;
        let jvm = JavaVM::new(jvm_args)?;

        #[cfg(feature = "editor")]
        success!("JDB debugger enabled on localhost:6741");

        log::info!("Created JVM instance");

        Ok(Self {
            jvm,
            dropbear_engine_class: None,
            system_manager_instance: None,
            jar_path: PathBuf::new(),
        })
    }

    pub fn init(&mut self, world: WorldPtr) -> anyhow::Result<()> {
        let mut env = self.jvm.attach_current_thread()?;

        if let Some(old_ref) = self.dropbear_engine_class.take() {
            let _ = old_ref; // drop
        }

        if let Some(old_ref) = self.system_manager_instance.take() {
            let _ = old_ref; // drop
        }

        log::trace!("Locating \"com/dropbear/ffi/NativeEngine\" class");
        let native_engine_class: JClass = env.find_class("com/dropbear/ffi/NativeEngine")?;
        log::trace!("Creating new instance of NativeEngine");
        let native_engine_obj = env.new_object(native_engine_class, "()V", &[])?;

        let result = get_exception_info(&mut env);
        if result.is_some() { return Err(anyhow::anyhow!("{}", result.unwrap())); }

        let world_handle = world as jlong;
        log::trace!("Calling NativeEngine.init() with arg [{} as JValue::Long]", world_handle);
        env.call_method(
            &native_engine_obj,
            "init",
            "(J)V",
            &[JValue::Long(world_handle)],
        )?;

        let result = get_exception_info(&mut env);
        if result.is_some() { return Err(anyhow::anyhow!("{}", result.unwrap())); }

        let dropbear_class: JClass = env.find_class("com/dropbear/DropbearEngine")?;
        log::trace!("Creating DropbearEngine constructor with arg (NativeEngine_object)");
        let dropbear_obj = env.new_object(
            dropbear_class,
            "(Lcom/dropbear/ffi/NativeEngine;)V",
            &[JValue::Object(&native_engine_obj)],
        )?;

        let result = get_exception_info(&mut env);
        if result.is_some() { return Err(anyhow::anyhow!("{}", result.unwrap())); }

        log::trace!("Creating new global ref for DropbearEngine");
        let engine_global_ref = env.new_global_ref(dropbear_obj)?;
        self.dropbear_engine_class = Some(engine_global_ref.clone());

        let jar_path_jstring = env.new_string(self.jar_path.to_string_lossy())?;
        let log_level_str = { LOG_LEVEL.lock().to_string() };
        let log_level_enum_class = env.find_class("com/dropbear/logging/LogLevel")?;
        let log_level_enum_instance = env.get_static_field(
            log_level_enum_class,
            log_level_str,
            "Lcom/dropbear/logging/LogLevel;"
        )?.l()?;

        let result = get_exception_info(&mut env);
        if result.is_some() { return Err(anyhow::anyhow!("{}", result.unwrap())); }

        let std_out_writer_class = env.find_class("com/dropbear/logging/StdoutWriter")?;
        let log_writer_obj = env.new_object(std_out_writer_class, "()V", &[])?;

        log::trace!("Locating \"com/dropbear/host/SystemManager\" class");
        let system_manager_class: JClass = env.find_class("com/dropbear/host/SystemManager")?;
        log::trace!("Creating SystemManager constructor with args (jar_path_string, dropbear_engine_object, log_writer_object, log_level_enum, log_target_string)");

        let result = get_exception_info(&mut env);
        if result.is_some() { return Err(anyhow::anyhow!("{}", result.unwrap())); }

        let log_target_jstring = env.new_string("dropbear_rust_host")?;

        let result = get_exception_info(&mut env);
        if result.is_some() { return Err(anyhow::anyhow!("{}", result.unwrap())); }

        let system_manager_obj = env.new_object(
            system_manager_class,
            "(Ljava/lang/String;Ljava/lang/Object;Lcom/dropbear/logging/LogWriter;Lcom/dropbear/logging/LogLevel;Ljava/lang/String;)V",
            &[
                JValue::Object(&jar_path_jstring),
                JValue::Object(engine_global_ref.as_obj()),
                JValue::Object(&log_writer_obj),
                JValue::Object(&log_level_enum_instance),
                JValue::Object(&log_target_jstring),
            ],
        )?;

        let result = get_exception_info(&mut env);
        if result.is_some() { return Err(anyhow::anyhow!("{}", result.unwrap())); }

        log::trace!("Creating new global ref for SystemManager");
        let manager_global_ref = env.new_global_ref(system_manager_obj)?;
        self.system_manager_instance = Some(manager_global_ref);

        let result = get_exception_info(&mut env);
        if result.is_some() { return Err(anyhow::anyhow!("{}", result.unwrap())); }

        Ok(())
    }

    pub fn reload(&mut self, _world: WorldPtr) -> anyhow::Result<()> {
        log::info!("Reloading JAR using SystemManager: {}", self.jar_path.display());

        if let Some(ref manager_ref) = self.system_manager_instance {
            let mut env = self.jvm.attach_current_thread()?;

            log::trace!("Calling SystemManager.reloadJar()");
            let jar_path_jstring = env.new_string(self.jar_path.to_string_lossy())?;
            env.call_method(
                manager_ref,
                "reloadJar",
                "(Ljava/lang/String;)V",
                &[JValue::Object(&jar_path_jstring)],
            )?;

            let result = get_exception_info(&mut env);
            if result.is_some() { return Err(anyhow::anyhow!("{}", result.unwrap())); }
        } else {
            log::warn!("SystemManager instance not found during reload.");
            // self.init(world)?;
            return Err(anyhow::anyhow!("SystemManager not initialised for reload."));
        }

        log::info!("Reload complete via SystemManager!");

        Ok(())
    }

    pub fn load_systems_for_tag(&mut self, tag: &str) -> anyhow::Result<()> {
        if let Some(ref manager_ref) = self.system_manager_instance {
            let mut env = self.jvm.attach_current_thread()?;

            log::trace!("Calling SystemManager.loadSystemsForTag() with tag: {}", tag);
            let tag_jstring = env.new_string(tag)?;
            env.call_method(
                manager_ref,
                "loadSystemsForTag",
                "(Ljava/lang/String;)V",
                &[JValue::Object(&tag_jstring)],
            )?;

            let result = get_exception_info(&mut env);
            if result.is_some() { return Err(anyhow::anyhow!("{}", result.unwrap())); }

            log::debug!("Loaded systems for tag: {}", tag);
        } else {
            return Err(anyhow::anyhow!("SystemManager not initialised when loading systems for tag: {}", tag));
        }
        Ok(())
    }

    pub fn update_all_systems(&self, dt: f32) -> anyhow::Result<()> {
        if let Some(ref manager_ref) = self.system_manager_instance {
            let mut env = self.jvm.attach_current_thread()?;

            log::trace!("Calling SystemManager.updateAllSystems() with dt: {}", dt);
            env.call_method(
                manager_ref,
                "updateAllSystems",
                "(F)V",
                &[JValue::Float(dt)],
            )?;

            let result = get_exception_info(&mut env);
            if result.is_some() { return Err(anyhow::anyhow!("{}", result.unwrap())); }

            log::trace!("Updated all systems with dt: {}", dt);
        } else {
            return Err(anyhow::anyhow!("SystemManager not initialised when updating systems."));
        }
        Ok(())
    }

    pub fn update_systems_for_tag(&self, tag: &str, dt: f32) -> anyhow::Result<()> {
        if let Some(ref manager_ref) = self.system_manager_instance {
            let mut env = self.jvm.attach_current_thread()?;

            log::trace!("Calling SystemManager.updateSystemsByTag() with tag: {}, dt: {}", tag, dt);
            let tag_jstring = env.new_string(tag)?;
            env.call_method(
                manager_ref,
                "updateSystemsByTag",
                "(Ljava/lang/String;F)V",
                &[JValue::Object(&tag_jstring), JValue::Float(dt)],
            )?;

            let result = get_exception_info(&mut env);
            if result.is_some() { return Err(anyhow::anyhow!("{}", result.unwrap())); }

            log::debug!("Updated systems for tag: {} with dt: {}", tag, dt);
        } else {
            return Err(anyhow::anyhow!("SystemManager not initialised when updating systems for tag: {}", tag));
        }
        Ok(())
    }

    pub fn get_system_count_for_tag(&self, tag: &str) -> anyhow::Result<i32> {
        if let Some(ref manager_ref) = self.system_manager_instance {
            let mut env = self.jvm.attach_current_thread()?;

            log::trace!("Calling SystemManager.getSystemCount() for tag: {}", tag);
            let tag_jstring = env.new_string(tag)?;
            let result = env.call_method(
                manager_ref,
                "getSystemCount",
                "(Ljava/lang/String;)I",
                &[JValue::Object(&tag_jstring)],
            )?;

            let exception = get_exception_info(&mut env);
            if exception.is_some() { return Err(anyhow::anyhow!("{}", exception.unwrap())); }

            Ok(result.i()?)
        } else {
            Err(anyhow::anyhow!("SystemManager not initialised when getting system count for tag: {}", tag))
        }
    }

    pub fn has_systems_for_tag(&self, tag: &str) -> anyhow::Result<bool> {
        if let Some(ref manager_ref) = self.system_manager_instance {
            let mut env = self.jvm.attach_current_thread()?;

            log::trace!("Calling SystemManager.hasSystemsForTag() for tag: {}", tag);
            let tag_jstring = env.new_string(tag)?;
            let result = env.call_method(
                manager_ref,
                "hasSystemsForTag",
                "(Ljava/lang/String;)Z",
                &[JValue::Object(&tag_jstring)],
            )?;

            let exception = get_exception_info(&mut env);
            if exception.is_some() { return Err(anyhow::anyhow!("{}", exception.unwrap())); }

            Ok(result.z()?)
        } else {
            Err(anyhow::anyhow!("SystemManager not initialised when checking for systems for tag: {}", tag))
        }
    }

    pub fn get_total_system_count(&self) -> anyhow::Result<i32> {
        if let Some(ref manager_ref) = self.system_manager_instance {
            let mut env = self.jvm.attach_current_thread()?;

            log::trace!("Calling SystemManager.getTotalSystemCount()");
            let result = env.call_method(
                manager_ref,
                "getTotalSystemCount",
                "()I",
                &[],
            )?;

            let exception = get_exception_info(&mut env);
            if exception.is_some() { return Err(anyhow::anyhow!("{}", exception.unwrap())); }

            Ok(result.i()?)
        } else {
            Err(anyhow::anyhow!("SystemManager not initialised when getting total system count."))
        }
    }

    pub fn clear_engine(&mut self) -> anyhow::Result<()> {
        if let Some(old_engine_ref) = self.dropbear_engine_class.take() {
            let _ = old_engine_ref; // drop
        }
        if let Some(old_manager_ref) = self.system_manager_instance.take() {
            let _ = old_manager_ref; // drop
        }
        Ok(())
    }
}

impl Drop for JavaContext {
    fn drop(&mut self) {
        if let Some(ref global_ref) = self.dropbear_engine_class {
            let _ = global_ref;
        }
        if let Some(old_ref) = self.system_manager_instance.take() {
            let _ = old_ref;
        }
    }
}

// JNIEXPORT jlong JNICALL Java_com_dropbear_ffi_JNINative_getEntity
//   (JNIEnv *, jclass, jlong, jstring);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getEntity(
    mut env: JNIEnv,
    _obj: jclass,
    world_handle: jlong,
    label: JString,
) -> jlong {
    let label_jni_result = env.get_string(&label);
    let label_str = match label_jni_result {
        Ok(java_string) => {
            match java_string.to_str() {
                Ok(rust_str) => rust_str.to_string(),
                Err(e) => {
                    println!("[Java_com_dropbear_ffi_JNINative_getEntity] [ERROR] Failed to convert Java string to Rust string: {}", e);
                    return -1;
                }
            }
        },
        Err(e) => {
            println!("[Java_com_dropbear_ffi_JNINative_getEntity] [ERROR] Failed to get string from JNI: {}", e);
            return -1;
        }
    };

    let world = world_handle as *mut World;

    if world.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_getEntity] [ERROR] World pointer is null");
        return -1;
    }

    let world = unsafe { &mut *world };

    for (id, entity) in world.query::<&AdoptedEntity>().iter() {
        if entity.model.label == label_str {
            return id.id() as jlong;
        }
    }
    -1
}

// JNIEXPORT jobject JNICALL Java_com_dropbear_ffi_JNINative_getTransform
//   (JNIEnv *, jclass, jlong, jlong);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_getTransform(
    mut env: JNIEnv,
    _class: jclass,
    world_handle: jlong,
    entity_id: jlong,
) -> JObject {
    let world = world_handle as *mut World;

    if world.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_getTransform] [ERROR] World pointer is null");
        return JObject::null();
    }

    let world = unsafe { &mut *world };

    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    if let Ok(mut q) = world.query_one::<(&AdoptedEntity, &Transform)>(entity)
        && let Some((_, transform)) = q.get()
    {
        let new_transform = *transform;

        let transform_class = match env.find_class("com/dropbear/math/Transform") {
            Ok(c) => c,
            Err(_) => return JObject::null(),
        };

        return match env.new_object(
            &transform_class,
            "(DDDDDDDDDD)V",
            &[
                new_transform.position.x.into(),
                new_transform.position.y.into(),
                new_transform.position.z.into(),
                new_transform.rotation.x.into(),
                new_transform.rotation.y.into(),
                new_transform.rotation.z.into(),
                new_transform.rotation.w.into(),
                new_transform.scale.x.into(),
                new_transform.scale.y.into(),
                new_transform.scale.z.into(),
            ],
        ) {
            Ok(java_transform) => java_transform,
            Err(_) => {
                println!("[Java_com_dropbear_ffi_JNINative_getTransform] [ERROR] Failed to create Transform object");
                JObject::null()
            }
        };
    }

    println!("[Java_com_dropbear_ffi_JNINative_getTransform] [ERROR] Failed to query for transform value for entity: {}", entity_id);
    JObject::null()
}

// JNIEXPORT void JNICALL Java_com_dropbear_ffi_JNINative_setTransform
//   (JNIEnv *, jclass, jlong, jlong, jobject);
#[unsafe(no_mangle)]
pub fn Java_com_dropbear_ffi_JNINative_setTransform(
    mut env: JNIEnv,
    _class: jclass,
    world_handle: jlong,
    entity_id: jlong,
    transform_obj: JObject,
) {
    let world = world_handle as *mut World;

    if world.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] World pointer is null");
        return;
    }

    let world = unsafe { &mut *world };
    let entity = unsafe { world.find_entity_from_id(entity_id as u32) };

    let get_number_field = |env: &mut JNIEnv, obj: &JObject, field_name: &str| -> f64 {
        match env.get_field(obj, field_name, "Ljava/lang/Number;") {
            Ok(v) => match v.l() {
                Ok(num_obj) => {
                    match env.call_method(&num_obj, "doubleValue", "()D", &[]) {
                        Ok(result) => result.d().unwrap_or_else(|_| {
                            println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] Failed to extract double from {}", field_name);
                            0.0
                        }),
                        Err(_) => {
                            println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] Failed to call doubleValue on {}", field_name);
                            0.0
                        }
                    }
                }
                Err(_) => {
                    println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] Failed to extract Number object for {}", field_name);
                    0.0
                }
            },
            Err(_) => {
                println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] Failed to get field {}", field_name);
                0.0
            }
        }
    };

    let position_obj: JObject = match env.get_field(&transform_obj, "position", "Lcom/dropbear/math/Vector3;") {
        Ok(v) => v.l().unwrap_or_else(|_| JObject::null()),
        Err(_) => JObject::null(),
    };

    let rotation_obj: JObject = match env.get_field(&transform_obj, "rotation", "Lcom/dropbear/math/Quaternion;") {
        Ok(v) => v.l().unwrap_or_else(|_| JObject::null()),
        Err(_) => JObject::null(),
    };

    let scale_obj: JObject = match env.get_field(&transform_obj, "scale", "Lcom/dropbear/math/Vector3;") {
        Ok(v) => v.l().unwrap_or_else(|_| JObject::null()),
        Err(_) => JObject::null(),
    };

    if position_obj.is_null() || rotation_obj.is_null() || scale_obj.is_null() {
        println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] Failed to extract position/rotation/scale objects");
        return;
    }

    let px = get_number_field(&mut env, &position_obj, "x");
    let py = get_number_field(&mut env, &position_obj, "y");
    let pz = get_number_field(&mut env, &position_obj, "z");

    let rx = get_number_field(&mut env, &rotation_obj, "x");
    let ry = get_number_field(&mut env, &rotation_obj, "y");
    let rz = get_number_field(&mut env, &rotation_obj, "z");
    let rw = get_number_field(&mut env, &rotation_obj, "w");

    let sx = get_number_field(&mut env, &scale_obj, "x");
    let sy = get_number_field(&mut env, &scale_obj, "y");
    let sz = get_number_field(&mut env, &scale_obj, "z");

    let new_transform = Transform {
        position: DVec3::new(px, py, pz),
        rotation: DQuat::from_axis_angle(DVec3::new(rx, ry, rz), rw),
        scale: DVec3::new(sx, sy, sz),
    };

    if let Ok(mut q) = world.query_one::<&mut Transform>(entity) {
        if let Some(transform) = q.get() {
            *transform = new_transform;
        } else {
            println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] Failed to query for transform component");
        }
    } else {
        println!("[Java_com_dropbear_ffi_JNINative_setTransform] [ERROR] Failed to query entity for transform component");
    }
}