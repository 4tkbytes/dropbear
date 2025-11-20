pub mod camera;
pub mod hierarchy;
pub mod input;
pub mod logging;
pub mod ptr;
pub mod result;
pub mod runtime;
pub mod scripting;
pub mod spawn;
pub mod states;
pub mod utils;
pub mod window;
pub mod scene;
pub mod component;
pub mod config;

pub use dropbear_traits as traits;
pub use dropbear_macro as macros;

pub use egui;

/// The appdata directory for storing any information.
///
/// By default, most of its items are located in [`app_dirs2::AppDataType::UserData`].
pub const APP_INFO: app_dirs2::AppInfo = app_dirs2::AppInfo {
    name: "Eucalyptus",
    author: "4tkbytes",
};
