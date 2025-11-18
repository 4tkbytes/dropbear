use std::any::Any;

/// A trait for components that can be attached to entities.
#[typetag::serde(tag = "type")]
pub trait Component: Send + Sync {
    /// Converts a [Component] to [Any]
    fn as_any(&self) -> &dyn Any;
    /// Converts a [Component] to [&mut Any]
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Clones the component's contents
    fn clone_component(&self) -> Box<dyn Component>;
    /// Fetches the original type name of the component
    fn type_name(&self) -> &'static str;
}
