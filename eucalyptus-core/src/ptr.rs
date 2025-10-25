use crate::input::InputState;
use crate::window::GraphicsCommand;
use crossbeam_channel::Sender;
use hecs::World;

pub type WorldPtr = *mut World;
pub type InputStatePtr = *mut InputState;
pub type GraphicsPtr = *const Sender<GraphicsCommand>;
