#![allow(non_snake_case)]
//! Deals with the Java Native Interface (JNI) with the help of the [`jni`] crate

use dropbear_engine::entity::AdoptedEntity;
use hecs::World;
use jni::objects::{GlobalRef, JClass, JString, JValue};
use jni::sys::jlong;
use jni::sys::jobject;
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use std::path::{Path, PathBuf};
use crate::APP_INFO;
use crate::logging::{LOG_LEVEL};
use crate::ptr::WorldPtr;

const LIBRARY_PATH: &[u8] = include_bytes!("../../../build/libs/dropbear-1.0-SNAPSHOT-all.jar");

/// Provides a context for any eucalyptus-core JNI calls and JVM hosting.
pub struct JavaContext {
    pub(crate) jvm: JavaVM,
    dropbear_engine_class: Option<GlobalRef>,
    system_manager_instance: Option<GlobalRef>,
    pub(crate) jar_path: PathBuf,
}

impl JavaContext {
    pub fn new(jar_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let root = app_dirs2::app_root(app_dirs2::AppDataType::UserData, &APP_INFO)?;
        let deps = root.join("dependencies");
        let host_jar_filename = "dropbear-1.0-SNAPSHOT-all.jar";
        let host_jar_path = deps.join(host_jar_filename);

        std::fs::create_dir_all(&deps)?;

        if !host_jar_path.exists() {
            log::info!("Host library JAR not found at {:?}, writing embedded JAR.", host_jar_path);
            std::fs::write(&host_jar_path, LIBRARY_PATH)?;
            log::info!("Host library JAR written to {:?}", host_jar_path);
        } else {
            log::debug!("Host library JAR found at {:?}", host_jar_path);
        }

        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option(format!(
                "-Djava.class.path={}",
                host_jar_path.display()
            ))
            .build()?;
        let jvm = JavaVM::new(jvm_args)?;

        log::info!("Created JVM instance");

        Ok(Self {
            jvm,
            dropbear_engine_class: None,
            system_manager_instance: None,
            jar_path: jar_path.as_ref().to_owned(),
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

        let world_handle = world as jlong;
        log::trace!("Calling NativeEngine.init() with arg [{} as JValue::Long]", world_handle);
        env.call_method(
            &native_engine_obj,
            "init",
            "(J)V",
            &[JValue::Long(world_handle)],
        )?;

        let dropbear_class: JClass = env.find_class("com/dropbear/DropbearEngine")?;
        log::trace!("Creating DropbearEngine constructor with arg (NativeEngine_object)");
        let dropbear_obj = env.new_object(
            dropbear_class,
            "(Lcom/dropbear/ffi/NativeEngine;)V",
            &[JValue::Object(&native_engine_obj)],
        )?;

        log::trace!("Creating new global ref for DropbearEngine");
        let engine_global_ref = env.new_global_ref(dropbear_obj)?;
        self.dropbear_engine_class = Some(engine_global_ref.clone());

        let jar_path_jstring = env.new_string(self.jar_path.to_string_lossy())?;
        let log_level_str = {LOG_LEVEL.lock().to_string()};
        let log_level_enum_class = env.find_class("com/dropbear/logging/LogLevel")?;
        let log_level_enum_instance = env.get_static_field(
            log_level_enum_class,
            log_level_str,
            "Lcom/dropbear/logging/LogLevel;"
        )?.l()?;

        let std_out_writer_class = env.find_class("com/dropbear/logging/StdoutWriter")?;
        let log_writer_obj = env.new_object(std_out_writer_class, "()V", &[])?;

        log::trace!("Locating \"com/dropbear/host/SystemManager\" class");
        let system_manager_class: JClass = env.find_class("com/dropbear/host/SystemManager")?;
        log::trace!("Creating SystemManager constructor with args (jar_path_string, dropbear_engine_object, log_writer_object, log_level_enum, log_target_string)");

        let log_target_jstring = env.new_string("dropbear_rust_host")?;

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

        log::trace!("Creating new global ref for SystemManager");
        let manager_global_ref = env.new_global_ref(system_manager_obj)?;
        self.system_manager_instance = Some(manager_global_ref);

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

            log::debug!("Updated all systems with dt: {}", dt);
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

#[unsafe(no_mangle)]
// JNIEXPORT jlong JNICALL Java_com_dropbear_ffi_JNINative_getEntity
// (JNIEnv *, jobject, jlong, jstring);
pub fn Java_com_dropbear_ffi_JNINative_getEntity(
    env: &mut JNIEnv,
    _obj: jobject,
    world_handle: jlong,
    label: JString,
) -> jlong {
    let label = env.get_string(&label).unwrap();
    let world = world_handle as *mut World;

    let world = unsafe { &mut *world };

    let rust_label = label.to_str().unwrap().to_string();

    for (id, entity) in world.query::<&AdoptedEntity>().iter() {
        if entity.model.label == rust_label {
            return jlong::from(id.id());
        }
    }
    jlong::from(-1)
}
