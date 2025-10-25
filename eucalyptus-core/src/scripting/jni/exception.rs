use jni::JNIEnv;
use jni::objects::JThrowable;

#[derive(Debug, Clone)]
pub struct JavaExceptionInfo {
    pub message: Option<String>,
    pub class_name: String,
    pub stack_trace: Option<String>,
}

impl std::fmt::Display for JavaExceptionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {}",
            self.class_name,
            self.message.as_deref().unwrap_or("<no message>")
        )?;
        if let Some(stack) = &self.stack_trace {
            write!(f, "\n{}", stack)?;
        }
        Ok(())
    }
}

pub fn get_exception_info(env: &mut JNIEnv) -> Option<JavaExceptionInfo> {
    if !env.exception_check().ok()? {
        return None;
    }

    let exception = env.exception_occurred().ok()?;

    env.exception_clear().ok()?;

    let exception_class = env.get_object_class(&exception).ok()?;
    let class_obj = env
        .call_method(&exception_class, "getName", "()Ljava/lang/String;", &[])
        .ok()?;
    let class_name_jstring = class_obj.l().ok()?;
    let class_name = env.get_string((&class_name_jstring).into()).ok()?;
    let class_name = class_name.to_string_lossy().to_string();

    let message = env
        .call_method(&exception, "getMessage", "()Ljava/lang/String;", &[])
        .ok()
        .and_then(|m| m.l().ok())
        .and_then(|m| {
            if m.is_null() {
                None
            } else {
                env.get_string((&m).into())
                    .ok()
                    .map(|s| s.to_string_lossy().to_string())
            }
        });

    let stack_trace = get_stack_trace(env, &exception);

    Some(JavaExceptionInfo {
        message,
        class_name,
        stack_trace,
    })
}

fn get_stack_trace(env: &mut JNIEnv, exception: &JThrowable) -> Option<String> {
    let string_writer_class = env.find_class("java/io/StringWriter").ok()?;
    let string_writer = env.new_object(string_writer_class, "()V", &[]).ok()?;

    let print_writer_class = env.find_class("java/io/PrintWriter").ok()?;
    let print_writer = env
        .new_object(
            print_writer_class,
            "(Ljava/io/Writer;)V",
            &[(&string_writer).into()],
        )
        .ok()?;

    env.call_method(
        exception,
        "printStackTrace",
        "(Ljava/io/PrintWriter;)V",
        &[(&print_writer).into()],
    )
    .ok()?;

    let stack_trace_obj = env
        .call_method(&string_writer, "toString", "()Ljava/lang/String;", &[])
        .ok()?;
    let stack_trace_jstring = stack_trace_obj.l().ok()?;
    let stack_trace = env.get_string((&stack_trace_jstring).into()).ok()?;

    Some(stack_trace.to_string_lossy().to_string())
}
