use jni::{InitArgsBuilder, JNIVersion, JavaVM};

/// A dropbear wrapper for Java Virtual Machine (JVM) based functions
pub(crate) struct JavaContext {
    jvm: JavaVM,
}

impl JavaContext {
    pub fn new() -> anyhow::Result<Self> {
        let jvm_args = InitArgsBuilder::new()
            .version(JNIVersion::V8)
            .build()?;

        let jvm = JavaVM::new(jvm_args)?;
        log::info!("Initialised JVM");
        Ok(Self {
            jvm
        })
    }
}