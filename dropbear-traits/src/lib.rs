use std::any::Any;
use std::fmt::Debug;

/// A type of component that gets serialized and deserialized into a scene config file.
#[typetag::serde(tag = "type")]
pub trait SerializableComponent: Send + Sync + Debug {
    /// Converts a [SerializableComponent] to an [Any] type.
    fn as_any(&self) -> &dyn Any;
    /// Converts a [SerializableComponent] to a mutable [Any] type
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Fetches the type name of that component
    fn type_name(&self) -> &'static str;
    /// Allows you to clone the dynamic object. 
    fn clone_boxed(&self) -> Box<dyn SerializableComponent>;
}

impl Clone for Box<dyn SerializableComponent> {
    fn clone(&self) -> Self {
        self.clone_boxed()
    }
}