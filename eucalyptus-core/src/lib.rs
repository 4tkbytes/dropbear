pub mod camera;
pub mod hierarchy;
pub mod input;
pub mod logging;
pub mod ptr;
pub mod result;
pub mod scripting;
pub mod spawn;
pub mod states;
pub mod utils;
pub mod window;
pub use dropbear_derive as derive;
pub use dropbear_traits as traits;

pub use egui;
pub use typetag;

/// The appdata directory for storing any information.
///
/// By default, most of its items are located in [`app_dirs2::AppDataType::UserData`].
pub const APP_INFO: app_dirs2::AppInfo = app_dirs2::AppInfo {
    name: "Eucalyptus",
    author: "4tkbytes",
};
