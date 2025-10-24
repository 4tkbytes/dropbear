use crossbeam_channel::Sender;
use hecs::World;
use dropbear_engine::graphics::{GraphicsCommand};
use crate::input::InputState;

pub type WorldPtr = *mut World;
pub type InputStatePtr = *mut InputState;
pub type GraphicsPtr = *const Sender<GraphicsCommand>;
