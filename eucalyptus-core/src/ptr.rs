use crossbeam_channel::Sender;
use hecs::World;
use crate::input::InputState;
use crate::window::GraphicsCommand;

pub type WorldPtr = *mut World;
pub type InputStatePtr = *mut InputState;
pub type GraphicsPtr = *const Sender<GraphicsCommand>;
