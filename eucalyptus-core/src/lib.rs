pub mod camera;
pub mod dropbear;
pub mod input;
pub mod logging;
pub mod ptr;
pub mod result;
pub mod scripting;
pub mod spawn;
pub mod states;
pub mod utils;
pub mod window;

pub use egui;

pub const APP_INFO: app_dirs2::AppInfo = app_dirs2::AppInfo {
    name: "Eucalyptus",
    author: "4tkbytes",
};
