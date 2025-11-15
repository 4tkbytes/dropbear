
/// A trait for components that can be attached to entities.
#[typetag::serde(tag = "type")]
pub trait Component: Send + Sync {
    fn as_any(&self) -> &dyn std::any::Any;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    fn clone_component(&self) -> Box<dyn Component>;
}