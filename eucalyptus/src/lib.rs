#[cfg(feature = "editor")]
pub mod editor;
#[cfg(feature = "editor")]
pub mod menu;

#[cfg(feature = "editor")]
pub mod build;

pub mod camera;
pub mod logging;
pub mod scripting;
pub mod states;
pub mod utils;

#[cfg(feature = "editor")]
pub const APP_INFO: app_dirs2::AppInfo = app_dirs2::AppInfo {
    name: "Eucalyptus",
    author: "4tkbytes",
};