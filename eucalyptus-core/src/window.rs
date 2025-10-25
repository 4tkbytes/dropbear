use std::sync::{Arc, OnceLock};
use crossbeam_channel::{unbounded, Receiver, Sender};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use winit::window::{CursorGrabMode, Window};

pub static GRAPHICS_COMMAND: Lazy<(Box<Sender<GraphicsCommand>>, Receiver<GraphicsCommand>)> = Lazy::new(|| { let (tx, rx) = unbounded::<GraphicsCommand>(); (Box::new(tx), rx) });
static PREVIOUS_CONFIG: OnceLock<RwLock<CommandCache>> = OnceLock::new();

fn get_config() -> &'static RwLock<CommandCache> {
    PREVIOUS_CONFIG.get_or_init(|| {
        RwLock::new(CommandCache::new())
    })
}

struct CommandCache {
    is_locked: bool,
    is_hidden: bool,
}

impl CommandCache {
    fn new() -> Self {
        Self {
            is_locked: false,
            is_hidden: false,
        }
    }
}

#[derive(Debug)]
pub enum GraphicsCommand {
    WindowCommand(WindowCommand)
}

#[derive(Debug)]
pub enum WindowCommand {
    WindowGrab(bool),
    HideCursor(bool),
}

pub fn poll(window: Arc<Window>) {
    while let Ok(cmd) = GRAPHICS_COMMAND.1.try_recv() {
        log::trace!("Received GRAPHICS_COMMAND update: {:?}", cmd);
        match cmd {
            GraphicsCommand::WindowCommand(w_cmd) => {
                match w_cmd {
                    WindowCommand::WindowGrab(is_locked) => {
                        let mut cfg = get_config().write();
                        if cfg.is_locked != is_locked {
                            if is_locked {
                                if let Err(e) = window
                                    .set_cursor_grab(CursorGrabMode::Confined)
                                    .or_else(|_| { window.set_cursor_grab(CursorGrabMode::Locked) })
                                {
                                    log_once::warn_once!("Failed to grab cursor: {:?}", e);
                                } else {
                                    log_once::info_once!("Grabbed cursor");
                                    cfg.is_locked = true;
                                }
                            } else if let Err(e) = window
                                .set_cursor_grab(CursorGrabMode::None)
                            {
                                log_once::warn_once!("Failed to release cursor: {:?}", e);
                            } else {
                                log_once::info_once!("Released cursor");
                                cfg.is_locked = false;
                            }
                        }
                    }
                    WindowCommand::HideCursor(should_hide) => {
                        let cfg = get_config().write();
                        if cfg.is_hidden != should_hide {
                            if should_hide {
                                window.set_cursor_visible(false);
                            } else {
                                window.set_cursor_visible(true);
                            }
                        }
                    }
                }
            }
        }
    }
}