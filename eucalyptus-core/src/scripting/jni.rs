#![allow(non_snake_case)]
//! Deals with the Java Native Interface (JNI) with the help of the [`jni`] crate

use std::path::Path;
use std::sync::Arc;
use hecs::World;
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use jni::objects::{GlobalRef, JClass, JString, JValue};
use jni::sys::jobject;
use jni::sys::jlong;
use dropbear_engine::entity::AdoptedEntity;

pub struct JavaContext {
    jvm: Arc<JavaVM>,
    dropbear_engine_class: Option<GlobalRef>,
}

impl JavaContext {
    pub fn new(jar_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option(format!("-Djava.class.path={}", jar_path.as_ref().display()))
            .build()?;

        let jvm = Arc::new(JavaVM::new(jvm_args)?);

        log::info!("Created JVM instance");

        Ok(Self {
            jvm,
            dropbear_engine_class: None,
        })
    }

    pub fn init(&mut self, world: &World) -> anyhow::Result<()> {
            let mut env = self.jvm.attach_current_thread()?;

            // create native engine first
            let native_engine_class: JClass = env.find_class("com/dropbear/ffi/NativeEngine")?;
            let native_engine_obj = env.new_object(native_engine_class, "()V", &[])?;

            let world_ptr = world as *const World;

            let world_handle = world_ptr as jlong;
            env.call_method(
                &native_engine_obj,
                "init",
                "(J)V",
                &[JValue::Long(world_handle)],
            )?;

            // create dropbear engine after
            let dropbear_class: JClass = env.find_class("com/dropbear/DropbearEngine")?;
            let dropbear_obj = env.new_object(
                dropbear_class,
                "(Lcom/dropbear/ffi/NativeEngine;)V",
                &[JValue::Object(&native_engine_obj)],
            )?;

            let global_ref = env.new_global_ref(dropbear_obj)?;

        self.dropbear_engine_class = Some(global_ref);

        if let Some(global_ref) = &self.dropbear_engine_class {
            env.call_method(&global_ref, "init", "()V", &[])?;
        }

        Ok(())
    }
}

#[unsafe(no_mangle)]
// JNIEXPORT jlong JNICALL Java_com_dropbear_ffi_JNINative_getEntity
// (JNIEnv *, jobject, jlong, jstring);
pub fn Java_com_dropbear_ffi_JNINative_getEntity(env: &mut JNIEnv, _obj: jobject, world_handle: jlong, label: JString) -> jlong {
    let label = env.get_string(&label).unwrap();
    let world = world_handle as *mut World;

    let world = unsafe { &mut *world };

    let rust_label = label.to_str().unwrap().to_string();

    for (id, entity) in world.query::<&AdoptedEntity>().iter() {
        if entity.model.label == rust_label {
            return jlong::from(id.id())
        }
    }
    jlong::from(-1)
}