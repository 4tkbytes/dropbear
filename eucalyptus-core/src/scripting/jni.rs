#![allow(non_snake_case)]
//! Deals with the Java Native Interface (JNI) with the help of the [`jni`] crate

use dropbear_engine::entity::AdoptedEntity;
use hecs::World;
use jni::objects::{GlobalRef, JClass, JString, JValue};
use jni::sys::jlong;
use jni::sys::jobject;
use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use std::path::{Path};

pub type WorldPtr = *mut World;

/// Provides a context for any eucalyptus-core JNI calls and JVM hosting.
pub struct JavaContext {
    pub(crate) jvm: JavaVM,
    dropbear_engine_class: Option<GlobalRef>,
}

impl JavaContext {
    pub fn new(jar_path: impl AsRef<Path>) -> anyhow::Result<Self> {
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .option(format!("-Djava.class.path={}", jar_path.as_ref().display()))
            .build()?;

        let jvm = JavaVM::new(jvm_args)?;

        log::info!("Created JVM instance");

        Ok(Self {
            jvm,
            dropbear_engine_class: None,
        })
    }

    pub fn init(&mut self, world: WorldPtr) -> anyhow::Result<()> {
        let mut env = self.jvm.attach_current_thread()?;

        if let Some(old_ref) = self.dropbear_engine_class.take() {
            let _ = old_ref; // drop
        }

        // create native engine first
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

        // create dropbear engine after
        let dropbear_class: JClass = env.find_class("com/dropbear/DropbearEngine")?;
        log::trace!("Creating DropbearEngine constructor with arg (NativeEngine_object)");
        let dropbear_obj = env.new_object(
            dropbear_class,
            "(Lcom/dropbear/ffi/NativeEngine;)V",
            &[JValue::Object(&native_engine_obj)],
        )?;

        log::trace!("Creating new global ref for DropbearEngine");
        let global_ref = env.new_global_ref(dropbear_obj)?;
        self.dropbear_engine_class = Some(global_ref);

        Ok(())
    }

    pub fn clear_engine(&mut self) -> anyhow::Result<()> {
        if let Some(old_ref) = self.dropbear_engine_class.take() {
            let _ = old_ref; // drop
        }
        self.dropbear_engine_class = None;
        Ok(())
    }

    pub fn load_scripts_for_entity(
        &mut self,
        world: WorldPtr,
        entity_id: u32,
        tag: &str,
    ) -> anyhow::Result<Vec<GlobalRef>> {
        log::trace!("Loading scripts for entity {} with tag {}", entity_id, tag);
        let mut env = self.jvm.attach_current_thread()?;

        let registry_class = env.find_class("com/dropbear/decl/RunnableRegistry")?;
        log::trace!("Getting RunnableRegistry instance");
        let registry_instance = env.get_static_field(
            registry_class,
            "INSTANCE",
            "Lcom/dropbear/decl/RunnableRegistry;",
        )?;

        let tag_jstring = env.new_string(tag)?;

        log::trace!("Calling RunnableRegistry.instantiateScripts() with arg [{} as Object]", tag);
        let scripts_list = env.call_method(
            registry_instance.l()?,
            "instantiateScripts",
            "(Ljava/lang/String;)Ljava/util/List;",
            &[JValue::Object(&tag_jstring)],
        )?;

        log::trace!("Getting List instance");
        let scripts_list = scripts_list.l()?;
        log::trace!("Getting List");
        let scripts = env.get_list(&scripts_list)?;

        let mut script_instances = Vec::new();

        for i in 0..scripts.size(&mut env)? {
            log::trace!("Getting script instance at index {}", i);
            let script_obj = scripts.get(&mut env, i)?;

            if let Some(script_system) = script_obj {
                log::trace!("Locating class \"com/dropbear/EntityRef\"");
                let entity_id_class = env.find_class("com/dropbear/EntityId")?;
                log::trace!("Initialising new EntityId with arg [{} as JValue::Long]", entity_id as i64);
                let entity_id_obj = env.new_object(
                    entity_id_class,
                    "(J)V",
                    &[JValue::Long(entity_id as i64)],
                )?;

                log::trace!("Locating class \"com/dropbear/EntityRef\"");
                let entity_ref_class = env.find_class("com/dropbear/EntityRef")?;
                log::trace!("Initialising new EntityRef with arg [entity_id as Object]");
                let entity_ref_obj = env.new_object(
                    entity_ref_class,
                    "(Lcom/dropbear/EntityId;)V",
                    &[JValue::Object(&entity_id_obj)],
                )?;

                log::trace!("Setting field \"currentEntity\" on script system with arg [entity_ref as Object]");
                env.set_field(
                    &script_system,
                    "currentEntity",
                    "Lcom/dropbear/EntityRef;",
                    JValue::Object(&entity_ref_obj),
                )?;

                log::trace!("Creating engine for entity");
                let engine_ref = self.create_engine_for_entity(world, entity_id)?;

                log::trace!("Calling script's load method with arg [engine as Object]");
                env.call_method(
                    &script_system,
                    "load",
                    "(Lcom/dropbear/DropbearEngine;)V",
                    &[JValue::Object(engine_ref.as_obj())],
                )?;

                log::trace!("Creating global ref for script");
                script_instances.push(env.new_global_ref(script_system)?);
            }
        }

        Ok(script_instances)
    }

    pub fn create_engine_for_entity(&self, world: WorldPtr, entity_id: u32) -> anyhow::Result<GlobalRef> {
        let mut env = self.jvm.attach_current_thread()?;

        log::trace!("Locating \"com/dropbear/ffi/NativeEngine\" class");
        let native_engine_class = env.find_class("com/dropbear/ffi/NativeEngine")?;
        log::trace!("Creating new instance of NativeEngine");
        let native_engine_obj = env.new_object(native_engine_class, "()V", &[])?;

        log::trace!("Calling NativeEngine.init() with arg {}", world as jlong);
        env.call_method(
            &native_engine_obj,
            "init",
            "(J)V",
            &[JValue::Long(world as jlong)],
        )?;

        log::trace!("Locating \"com/dropbear/EntityId\"");
        let entity_id_class = env.find_class("com/dropbear/EntityId")?;
        log::trace!("Creating new EntityId with arg {}", entity_id as i64);
        let entity_id_obj = env.new_object(
            entity_id_class,
            "(J)V",
            &[JValue::Long(entity_id as i64)],
        )?;

        log::trace!("Locating \"com/dropbear/EntityRef\"");
        let entity_ref_class = env.find_class("com/dropbear/EntityRef")?;
        log::trace!("Creating new EntityRef with arg [entity_id as Object]");
        env.new_object(
            entity_ref_class,
            "(Lcom/dropbear/EntityId;)V",
            &[JValue::Object(&entity_id_obj)],
        )?;

        log::trace!("Locating class \"com/dropbear/DropbearEngine\"");
        let dropbear_class = env.find_class("com/dropbear/DropbearEngine")?;
        log::trace!("Creating DropbearEngine constructor with arg (NativeEngine_object)");
        let dropbear_obj = env.new_object(
            dropbear_class,
            "(Lcom/dropbear/ffi/NativeEngine;)V",
            &[
                JValue::Object(&native_engine_obj),
                // JValue::Object(&entity_ref_obj),
            ],
        )?;

        log::trace!("Creating new global ref for DropbearEngine and returning");
        Ok(env.new_global_ref(dropbear_obj)?)
    }
}

impl Drop for JavaContext {
    fn drop(&mut self) {
        if let Some(ref global_ref) = self.dropbear_engine_class {
            let _ = global_ref;
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
