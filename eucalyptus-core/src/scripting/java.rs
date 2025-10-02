use jni::{InitArgsBuilder, JNIEnv, JNIVersion, JavaVM};
use jni::objects::{JObject, JString};

/// A dropbear wrapper for Java Virtual Machine (JVM) based functions
pub(crate) struct JavaContext {
    jvm: JavaVM,
}

impl JavaContext {
    /// Creates a new [`JavaContext`]
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

// JNIEXPORT jstring JNICALL Java_com_dropbear_ffi_NativeEngine_Ping
//    (JNIEnv *, jobject, jstring);
#[unsafe(no_mangle)]
#[allow(non_snake_case)]
pub extern "system" fn Java_com_dropbear_ffi_NativeEngine_Ping<'local>(
    mut env: JNIEnv<'local>,
    _this: JObject<'local>,
    input: JString<'local>,
) -> JString<'local> {
    let message: String = env.get_string(&input)
        .expect("Failed to get input string")
        .into();

    let response = format!("Pong! You sent: {}", message);

    let output = env.new_string(response)
        .expect("Failed to create output string");

    output.into()
}