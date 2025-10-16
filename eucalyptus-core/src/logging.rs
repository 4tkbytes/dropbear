//! This crate will allow for logging to specific locations. Essentially, just removing boilerplate
//!
//! # Supported logging locations:
//! - Toasts (egui)
//! - Console
//! - File (to be implemented)

#[cfg(feature = "editor")]
use egui::Context;
use std::fmt::{Display, Formatter};

#[cfg(feature = "editor")]
use egui_toast_fork::Toasts;

#[cfg(feature = "editor")]
use once_cell::sync::Lazy;
#[cfg(feature = "editor")]
use parking_lot::Mutex;

pub static LOG_LEVEL: Lazy<Mutex<LogLevel>> = Lazy::new(|| {
    Mutex::new(LogLevel::default())
});

#[derive(Default)]
/// LogLevel as shown in LogLevel.kt in the dropbear engine jar library
pub enum LogLevel {
    TRACE,
    DEBUG,
    #[default]
    INFO,
    WARN,
    ERROR
}

impl Display for LogLevel {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::TRACE => write!(f, "TRACE"),
            LogLevel::DEBUG => write!(f, "DEBUG"),
            LogLevel::INFO => write!(f, "INFO"),
            LogLevel::WARN => write!(f, "WARN"),
            LogLevel::ERROR => write!(f, "ERROR"),
        }
    }
}

#[cfg(feature = "editor")]
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
#[cfg(feature = "editor")]
pub fn render(context: &Context) {
    let mut toasts = GLOBAL_TOASTS.lock();
    toasts.show(context);
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

        #[cfg(feature = "editor")]
        {
            use egui_toast_fork::{Toast, ToastKind};
            use $crate::logging::GLOBAL_TOASTS;
            let mut toasts = GLOBAL_TOASTS.lock();
            toasts.add(Toast {
                text: _msg.into(),
                kind: ToastKind::Error,
                options: egui_toast_fork::ToastOptions::default()
                    .duration_in_seconds(3.0)
                    .show_progress(true),
                style: egui_toast_fork::ToastStyle::default(),
            });
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
        log::debug!("{}", _msg);

        #[cfg(feature = "editor")]
        {
            use egui_toast_fork::{Toast, ToastKind};
            use $crate::logging::GLOBAL_TOASTS;
            let mut toasts = GLOBAL_TOASTS.lock();
            toasts.add(Toast {
                    text: _msg.into(),
                    kind: ToastKind::Success,
                    options: egui_toast_fork::ToastOptions::default()
                        .duration_in_seconds(3.0)
                        .show_progress(true),
                    style: egui_toast_fork::ToastStyle::default(),
                });
            };
        }
    };
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

        #[cfg(feature = "editor")]
        {
            use egui_toast_fork::{Toast, ToastKind};
            use $crate::logging::GLOBAL_TOASTS;
            let mut toasts = GLOBAL_TOASTS.lock();
            toasts.add(Toast {
                    text: _msg.into(),
                    kind: ToastKind::Warning,
                    options: egui_toast_fork::ToastOptions::default()
                        .duration_in_seconds(3.0)
                        .show_progress(true),
                    style: egui_toast_fork::ToastStyle::default(),
                });
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

        #[cfg(feature = "editor")]
        {
            use egui_toast_fork::{Toast, ToastKind};
            use $crate::logging::GLOBAL_TOASTS;
            let mut toasts = GLOBAL_TOASTS.lock();
            toasts.add(Toast {
                text: _msg.into(),
                kind: ToastKind::Info,
                options: egui_toast_fork::ToastOptions::default()
                    .duration_in_seconds(1.0)
                    .show_progress(false),
                style: egui_toast_fork::ToastStyle::default(),
            });
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

        #[cfg(feature = "editor")]
        {
            use egui_toast_fork::{Toast, ToastKind};
            use $crate::logging::GLOBAL_TOASTS;
            let mut toasts = GLOBAL_TOASTS.lock();
            toasts.add(Toast {
                text: _msg.into(),
                kind: ToastKind::Info,
                options: egui_toast_fork::ToastOptions::default()
                    .duration_in_seconds(1.0)
                    .show_progress(false),
                style: egui_toast_fork::ToastStyle::default(),
            });
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

        #[cfg(feature = "editor")]
        {
            use egui_toast_fork::{Toast, ToastKind};
            use $crate::logging::GLOBAL_TOASTS;
            let mut toasts = GLOBAL_TOASTS.lock();
            toasts.add(Toast {
                text: _msg.into(),
                kind: ToastKind::Success,
                options: egui_toast_fork::ToastOptions::default()
                    .duration_in_seconds(3.0)
                    .show_progress(true),
                style: egui_toast_fork::ToastStyle::default(),
            });
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

        #[cfg(feature = "editor")]
        {
            use egui_toast_fork::{Toast, ToastKind};
            use $crate::logging::GLOBAL_TOASTS;
            let mut toasts = GLOBAL_TOASTS.lock();
            toasts.add(Toast {
                text: _msg.into(),
                kind: ToastKind::Warning,
                options: egui_toast_fork::ToastOptions::default()
                    .duration_in_seconds(3.0)
                    .show_progress(true),
                style: egui_toast_fork::ToastStyle::default(),
            });
        }
    };
}
