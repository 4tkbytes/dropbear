use std::sync::Arc;
use crossbeam_channel::{unbounded, Receiver, Sender};
use once_cell::sync::Lazy;
use winit::window::{CursorGrabMode, Window};
use dropbear_engine::graphics::{GraphicsCommand, WindowCommand};

pub static GRAPHICS_COMMAND: Lazy<(Box<Sender<GraphicsCommand>>, Receiver<GraphicsCommand>)> = Lazy::new(|| { let (tx, rx) = unbounded::<GraphicsCommand>(); (Box::new(tx), rx) });

pub fn poll(window: Arc<Window>) {
    while let Ok(cmd) = GRAPHICS_COMMAND.1.try_recv() {
        log::trace!("Received GRAPHICS_COMMAND update: {:?}", cmd);
        match cmd {
            GraphicsCommand::WindowCommand(w_cmd) => {
                match w_cmd {
                    WindowCommand::WindowGrab(is_locked) => {
                        if is_locked {
                            if let Err(e) = window
                                .set_cursor_grab(CursorGrabMode::Locked)
                                .or_else(|_| { window.set_cursor_grab(CursorGrabMode::Confined) })
                            {
                                log_once::warn_once!("Failed to grab cursor: {:?}", e);
                            } else {
                                log_once::info_once!("Grabbed cursor");
                            }
                        } else if let Err(e) = window
                            .set_cursor_grab(CursorGrabMode::None)
                        {
                            log_once::warn_once!("Failed to release cursor: {:?}", e);
                        } else {
                            log_once::info_once!("Released cursor");
                        }
                    }
                }
            }
        }
    }
}