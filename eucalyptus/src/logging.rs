//! This crate will allow for logging to specific locations. Essentially, just removing boilerplate
//! 
//! # Supported logging locations:
//! - Toasts (egui)
//! - Console
//! - File (to be implemented)

use std::sync::Mutex;

use egui::Context;
use egui_toast_fork::Toasts;
use once_cell::sync::Lazy;

pub static GLOBAL_TOASTS: Lazy<Mutex<Toasts>> = Lazy::new(|| {
    Mutex::new(
        Toasts::new()
            .anchor(egui::Align2::RIGHT_BOTTOM, (-10.0, -10.0))
            .direction(egui::Direction::BottomUp),
    )
});

/// Renders the toasts. Requires an egui context. 
/// 
/// Useful when paired with a function that contains [`crate`]
pub(crate) fn render(context: &Context) {
    match GLOBAL_TOASTS.lock() {
        Ok(mut toasts) => {
            toasts.show(context);
        },
        Err(e) => {
            log::error!("Unable to render toast: {e}")
        }
    }
}

/// Fatal log macro
/// 
/// This is useful for when there is a fatal error like a missing file cannot be found. 
/// 
/// This macro creates a toast under the [`egui_toast_fork::ToastKind::Error`] and logs
/// with [`log::error!`]
#[macro_export]
macro_rules! fatal {
    ($($arg:tt)*) => {{
        let _msg = format!($($arg)*);
        log::error!("{}", _msg);

        {
            use egui_toast_fork::{Toast, ToastKind};
            use crate::logging::GLOBAL_TOASTS;
            match GLOBAL_TOASTS.lock() {
            Ok(mut toasts) => {
                toasts.add(Toast {
                    text: _msg.into(),
                    kind: ToastKind::Error,
                    options: egui_toast_fork::ToastOptions::default()
                        .duration_in_seconds(3.0)
                        .show_progress(true),
                    style: egui_toast_fork::ToastStyle::default(),
                });
            },
            Err(e) => {
                log::error!("Unable to render toast: {e}")
            }
        }
        }
    }};
}

/// Success log macro
/// 
/// This is useful for when loading a save is successful. 
/// 
/// This macro creates a toast under the [`egui_toast_fork::ToastKind::Success`] and logs
/// with [`log::info!`]
#[macro_export]
macro_rules! success {
    ($($arg:tt)*) => {{
        let _msg = format!($($arg)*);
        log::info!("{}", _msg);

        {
            use egui_toast_fork::{Toast, ToastKind};
            use crate::logging::GLOBAL_TOASTS;
            match GLOBAL_TOASTS.lock() {
                Ok(mut toasts) => {
                    toasts.add(Toast {
                        text: _msg.into(),
                        kind: ToastKind::Success,
                        options: egui_toast_fork::ToastOptions::default()
                            .duration_in_seconds(3.0)
                            .show_progress(true),
                        style: egui_toast_fork::ToastStyle::default(),
                    });
                },
                Err(e) => {
                    log::error!("Unable to render toast: {e}")
                }
            }
        }
    }};
}

/// Warn log macro
/// 
/// This is useful for when there is a non-fatal error like unable to copy.
/// 
/// This macro creates a toast under the [`egui_toast_fork::ToastKind::Warning`] and logs
/// with [`log::warn!`]
#[macro_export]
macro_rules! warn {
    ($($arg:tt)*) => {{
        let _msg = format!($($arg)*);
        log::warn!("{}", _msg);

        {
            use egui_toast_fork::{Toast, ToastKind};
            use crate::logging::GLOBAL_TOASTS;
            match GLOBAL_TOASTS.lock() {
                Ok(mut toasts) => {
                    toasts.add(Toast {
                        text: _msg.into(),
                        kind: ToastKind::Warning,
                        options: egui_toast_fork::ToastOptions::default()
                            .duration_in_seconds(3.0)
                            .show_progress(true),
                        style: egui_toast_fork::ToastStyle::default(),
                    });
                },
                Err(e) => {
                    log::error!("Unable to render toast: {e}")
                }
            }
        }
    }};
}

/// Info log macro
/// 
/// This is useful for notifying the user of a change, where it doesn't have to be important. 
/// 
/// This macro creates a toast under the [`egui_toast_fork::ToastKind::Info`] and logs
/// with [`log::debug!`]
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {{
        let _msg = format!($($arg)*);
        log::debug!("{}", _msg);

        {
            use egui_toast_fork::{Toast, ToastKind};
            use crate::logging::GLOBAL_TOASTS;
            match GLOBAL_TOASTS.lock() {
                Ok(mut toasts) => {
                    toasts.add(Toast {
                        text: _msg.into(),
                        kind: ToastKind::Info,
                        options: egui_toast_fork::ToastOptions::default()
                            .duration_in_seconds(1.0)
                            .show_progress(false),
                        style: egui_toast_fork::ToastStyle::default(),
                    });
                },
                Err(e) => {
                    log::error!("Unable to render toast: {e}")
                }
            }
        }
    }};
}

/// Macro for logging info without the console
/// 
/// This macro should be "info_toast", however in the case that I ever need to add some more functionality, 
/// this would be useful. 
/// 
/// Its feature-heavy counterpart would be [`crate::success!`].
/// 
/// It creates a toast under [`egui_toast_fork::ToastKind::Info`]. 
#[macro_export]
macro_rules! info_without_console {
    ($($arg:tt)*) => {
        let _msg = format!($($arg)*);

        {
            use egui_toast_fork::{Toast, ToastKind};
            use crate::logging::GLOBAL_TOASTS;
            match GLOBAL_TOASTS.lock() {
                Ok(mut toasts) => {
                    toasts.add(Toast {
                        text: _msg.into(),
                        kind: ToastKind::Info,
                        options: egui_toast_fork::ToastOptions::default()
                            .duration_in_seconds(1.0)
                            .show_progress(false),
                        style: egui_toast_fork::ToastStyle::default(),
                    });
                },
                Err(e) => {
                    log::error!("Unable to render toast: {e}")
                }
            }
        }
    };
}

/// Macro for logging a successful action without the console
/// 
/// This macro should be "success_toast", however in the case that I ever need to add some more functionality, 
/// this would be useful. 
/// 
/// Its feature-heavy counterpart would be [`crate::success!`].
/// 
/// It creates a toast under [`egui_toast_fork::ToastKind::Success`]. 
#[macro_export]
macro_rules! success_without_console {
    ($($arg:tt)*) => {
        let _msg = format!($($arg)*);

        {
            use egui_toast_fork::{Toast, ToastKind};
            use crate::logging::GLOBAL_TOASTS;
            match GLOBAL_TOASTS.lock() {
                Ok(mut toasts) => {
                    toasts.add(Toast {
                        text: _msg.into(),
                        kind: ToastKind::Success,
                        options: egui_toast_fork::ToastOptions::default()
                            .duration_in_seconds(3.0)
                            .show_progress(true),
                        style: egui_toast_fork::ToastStyle::default(),
                    });
                },
                Err(e) => {
                    log::error!("Unable to render toast: {e}")
                }
            }
        }
    };
}

/// Macro for logging a successful action without the console
/// 
/// This macro should be "success_toast", however in the case that I ever need to add some more functionality, 
/// this would be useful. 
/// 
/// Its feature-heavy counterpart would be [`crate::warn!`].
/// 
/// It creates a toast under [`egui_toast_fork::ToastKind::Warning`]. 
#[macro_export]
macro_rules! warn_without_console {
    ($($arg:tt)*) => {
        let _msg = format!($($arg)*);

        {
            use egui_toast_fork::{Toast, ToastKind};
            use crate::logging::GLOBAL_TOASTS;
            match GLOBAL_TOASTS.lock() {
                Ok(mut toasts) => {
                    toasts.add(Toast {
                        text: _msg.into(),
                        kind: ToastKind::Warning,
                        options: egui_toast_fork::ToastOptions::default()
                            .duration_in_seconds(3.0)
                            .show_progress(true),
                        style: egui_toast_fork::ToastStyle::default(),
                    });
                },
                Err(e) => {
                    log::error!("Unable to render toast: {e}")
                }
            }
        }
    };
}