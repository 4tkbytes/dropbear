use std::sync::Arc;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use winit::window::{CursorGrabMode, Window};

#[derive(Debug)]
pub enum WindowCommand {
    SetCursorGrab(bool),
}

pub static WINDOW_COMMANDS: Lazy<Mutex<Vec<WindowCommand>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

pub fn poll(window: Arc<Window>) {
    for cmd in WINDOW_COMMANDS.lock().drain(..) {
        match cmd {
            WindowCommand::SetCursorGrab(locked) => {
                println!("Setting cursor grab to {}", locked);
                if locked {
                    window.set_cursor_visible(false);
                    if let Err(e) = window.set_cursor_grab(CursorGrabMode::Confined)
                        .or_else(|_| window.set_cursor_grab(CursorGrabMode::Locked)) {
                        eprintln!("Unable to grab mouse: {}", e);
                    }
                } else {
                    window.set_cursor_visible(true);
                    if let Err(e) = window.set_cursor_grab(CursorGrabMode::None) {
                        eprintln!("Unable to release mouse grab: {}", e);
                    }
                }
            }
        }
    }
}