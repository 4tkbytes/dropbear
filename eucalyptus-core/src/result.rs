use std::ptr;
use jni::objects::{JClass, JObject, JString};

/// Trait used by the `jni` crate for easier error matching.
pub trait ResultToNull {
    type Output;

    /// If the output is of a type [`jni::Error`](jni::errors::Error),
    /// it will return a null pointer. Pretty useful for when you don't want to have
    /// to deal with error matching.
    ///
    /// Specifically: converts result to the inner value on [`Ok`], or a null pointer on [`Err`]
    fn or_null(self) -> Self::Output;
}

impl ResultToNull for Result<JObject<'_>, jni::errors::Error> {
    type Output = JObject<'static>;

    fn or_null(self) -> Self::Output {
        match self {
            Ok(val) => unsafe { JObject::from_raw(val.into_raw()) },
            Err(_) => JObject::null(),
        }
    }
}

impl ResultToNull for anyhow::Result<JObject<'_>> {
    type Output = JObject<'static>;

    fn or_null(self) -> Self::Output {
        match self {
            Ok(val) => unsafe { JObject::from_raw(val.into_raw()) },
            Err(_) => JObject::null(),
        }
    }
}

impl ResultToNull for Result<JClass<'_>, jni::errors::Error> {
    type Output = JClass<'static>;

    fn or_null(self) -> Self::Output {
        match self {
            Ok(val) => unsafe { JClass::from_raw(val.into_raw()) },
            Err(_) => unsafe { JClass::from_raw(ptr::null_mut()) },
        }
    }
}

impl ResultToNull for anyhow::Result<JClass<'_>> {
    type Output = JClass<'static>;

    fn or_null(self) -> Self::Output {
        match self {
            Ok(val) => unsafe { JClass::from_raw(val.into_raw()) },
            Err(_) => unsafe { JClass::from_raw(ptr::null_mut()) },
        }
    }
}

impl ResultToNull for Result<JString<'_>, jni::errors::Error> {
    type Output = JString<'static>;

    fn or_null(self) -> Self::Output {
        match self {
            Ok(val) => unsafe { JString::from_raw(val.into_raw()) },
            Err(_) => unsafe { JString::from_raw(ptr::null_mut()) },
        }
    }
}

impl ResultToNull for anyhow::Result<JString<'_>> {
    type Output = JString<'static>;

    fn or_null(self) -> Self::Output {
        match self {
            Ok(val) => unsafe { JString::from_raw(val.into_raw()) },
            Err(_) => unsafe { JString::from_raw(ptr::null_mut()) },
        }
    }
}