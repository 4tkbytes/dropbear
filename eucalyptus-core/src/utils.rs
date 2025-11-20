use crate::states::Node;
use dropbear_engine::utils::{ResourceReference, ResourceReferenceType, relative_path_from_euca};
use std::path::PathBuf;
use winit::keyboard::KeyCode;

pub const PROTO_TEXTURE: &[u8] = include_bytes!("../../resources/textures/proto.png");

pub fn search_nodes_recursively<'a, F>(nodes: &'a [Node], matcher: &F, results: &mut Vec<&'a Node>)
where
    F: Fn(&Node) -> bool,
{
    for node in nodes {
        if matcher(node) {
            results.push(node);
        }
        match node {
            Node::File(_) => {}
            Node::Folder(folder) => {
                search_nodes_recursively(&folder.nodes, matcher, results);
            }
        }
    }
}

/// Progress events for project creation
pub enum ProjectProgress {
    Step {
        _progress: f32,
        _message: String,
    },
    #[allow(dead_code)] // idk why its giving me this warning :(
    Error(String),
    Done,
}

#[derive(Clone)]
pub enum ViewportMode {
    None,
    CameraMove,
    Gizmo,
}

pub fn keycode_from_ordinal(ordinal: i32) -> Option<KeyCode> {
    match ordinal {
        0 => Some(KeyCode::Backquote),
        1 => Some(KeyCode::Backslash),
        2 => Some(KeyCode::BracketLeft),
        3 => Some(KeyCode::BracketRight),
        4 => Some(KeyCode::Comma),
        5 => Some(KeyCode::Digit0),
        6 => Some(KeyCode::Digit1),
        7 => Some(KeyCode::Digit2),
        8 => Some(KeyCode::Digit3),
        9 => Some(KeyCode::Digit4),
        10 => Some(KeyCode::Digit5),
        11 => Some(KeyCode::Digit6),
        12 => Some(KeyCode::Digit7),
        13 => Some(KeyCode::Digit8),
        14 => Some(KeyCode::Digit9),
        15 => Some(KeyCode::Equal),
        16 => Some(KeyCode::IntlBackslash),
        17 => Some(KeyCode::IntlRo),
        18 => Some(KeyCode::IntlYen),
        19 => Some(KeyCode::KeyA),
        20 => Some(KeyCode::KeyB),
        21 => Some(KeyCode::KeyC),
        22 => Some(KeyCode::KeyD),
        23 => Some(KeyCode::KeyE),
        24 => Some(KeyCode::KeyF),
        25 => Some(KeyCode::KeyG),
        26 => Some(KeyCode::KeyH),
        27 => Some(KeyCode::KeyI),
        28 => Some(KeyCode::KeyJ),
        29 => Some(KeyCode::KeyK),
        30 => Some(KeyCode::KeyL),
        31 => Some(KeyCode::KeyM),
        32 => Some(KeyCode::KeyN),
        33 => Some(KeyCode::KeyO),
        34 => Some(KeyCode::KeyP),
        35 => Some(KeyCode::KeyQ),
        36 => Some(KeyCode::KeyR),
        37 => Some(KeyCode::KeyS),
        38 => Some(KeyCode::KeyT),
        39 => Some(KeyCode::KeyU),
        40 => Some(KeyCode::KeyV),
        41 => Some(KeyCode::KeyW),
        42 => Some(KeyCode::KeyX),
        43 => Some(KeyCode::KeyY),
        44 => Some(KeyCode::KeyZ),
        45 => Some(KeyCode::Minus),
        46 => Some(KeyCode::Period),
        47 => Some(KeyCode::Quote),
        48 => Some(KeyCode::Semicolon),
        49 => Some(KeyCode::Slash),
        50 => Some(KeyCode::AltLeft),
        51 => Some(KeyCode::AltRight),
        52 => Some(KeyCode::Backspace),
        53 => Some(KeyCode::CapsLock),
        54 => Some(KeyCode::ContextMenu),
        55 => Some(KeyCode::ControlLeft),
        56 => Some(KeyCode::ControlRight),
        57 => Some(KeyCode::Enter),
        58 => Some(KeyCode::SuperLeft),
        59 => Some(KeyCode::SuperRight),
        60 => Some(KeyCode::ShiftLeft),
        61 => Some(KeyCode::ShiftRight),
        62 => Some(KeyCode::Space),
        63 => Some(KeyCode::Tab),
        64 => Some(KeyCode::Convert),
        65 => Some(KeyCode::KanaMode),
        66 => Some(KeyCode::Lang1),
        67 => Some(KeyCode::Lang2),
        68 => Some(KeyCode::Lang3),
        69 => Some(KeyCode::Lang4),
        70 => Some(KeyCode::Lang5),
        71 => Some(KeyCode::NonConvert),
        72 => Some(KeyCode::Delete),
        73 => Some(KeyCode::End),
        74 => Some(KeyCode::Help),
        75 => Some(KeyCode::Home),
        76 => Some(KeyCode::Insert),
        77 => Some(KeyCode::PageDown),
        78 => Some(KeyCode::PageUp),
        79 => Some(KeyCode::ArrowDown),
        80 => Some(KeyCode::ArrowLeft),
        81 => Some(KeyCode::ArrowRight),
        82 => Some(KeyCode::ArrowUp),
        83 => Some(KeyCode::NumLock),
        84 => Some(KeyCode::Numpad0),
        85 => Some(KeyCode::Numpad1),
        86 => Some(KeyCode::Numpad2),
        87 => Some(KeyCode::Numpad3),
        88 => Some(KeyCode::Numpad4),
        89 => Some(KeyCode::Numpad5),
        90 => Some(KeyCode::Numpad6),
        91 => Some(KeyCode::Numpad7),
        92 => Some(KeyCode::Numpad8),
        93 => Some(KeyCode::Numpad9),
        94 => Some(KeyCode::NumpadAdd),
        95 => Some(KeyCode::NumpadBackspace),
        96 => Some(KeyCode::NumpadClear),
        97 => Some(KeyCode::NumpadClearEntry),
        98 => Some(KeyCode::NumpadComma),
        99 => Some(KeyCode::NumpadDecimal),
        100 => Some(KeyCode::NumpadDivide),
        101 => Some(KeyCode::NumpadEnter),
        102 => Some(KeyCode::NumpadEqual),
        103 => Some(KeyCode::NumpadHash),
        104 => Some(KeyCode::NumpadMemoryAdd),
        105 => Some(KeyCode::NumpadMemoryClear),
        106 => Some(KeyCode::NumpadMemoryRecall),
        107 => Some(KeyCode::NumpadMemoryStore),
        108 => Some(KeyCode::NumpadMemorySubtract),
        109 => Some(KeyCode::NumpadMultiply),
        110 => Some(KeyCode::NumpadParenLeft),
        111 => Some(KeyCode::NumpadParenRight),
        112 => Some(KeyCode::NumpadStar),
        113 => Some(KeyCode::NumpadSubtract),
        114 => Some(KeyCode::Escape),
        115 => Some(KeyCode::Fn),
        116 => Some(KeyCode::FnLock),
        117 => Some(KeyCode::PrintScreen),
        118 => Some(KeyCode::ScrollLock),
        119 => Some(KeyCode::Pause),
        120 => Some(KeyCode::BrowserBack),
        121 => Some(KeyCode::BrowserFavorites),
        122 => Some(KeyCode::BrowserForward),
        123 => Some(KeyCode::BrowserHome),
        124 => Some(KeyCode::BrowserRefresh),
        125 => Some(KeyCode::BrowserSearch),
        126 => Some(KeyCode::BrowserStop),
        127 => Some(KeyCode::Eject),
        128 => Some(KeyCode::LaunchApp1),
        129 => Some(KeyCode::LaunchApp2),
        130 => Some(KeyCode::LaunchMail),
        131 => Some(KeyCode::MediaPlayPause),
        132 => Some(KeyCode::MediaSelect),
        133 => Some(KeyCode::MediaStop),
        134 => Some(KeyCode::MediaTrackNext),
        135 => Some(KeyCode::MediaTrackPrevious),
        136 => Some(KeyCode::Power),
        137 => Some(KeyCode::Sleep),
        138 => Some(KeyCode::AudioVolumeDown),
        139 => Some(KeyCode::AudioVolumeMute),
        140 => Some(KeyCode::AudioVolumeUp),
        141 => Some(KeyCode::WakeUp),
        142 => Some(KeyCode::Meta),
        143 => Some(KeyCode::Hyper),
        144 => Some(KeyCode::Turbo),
        145 => Some(KeyCode::Abort),
        146 => Some(KeyCode::Resume),
        147 => Some(KeyCode::Suspend),
        148 => Some(KeyCode::Again),
        149 => Some(KeyCode::Copy),
        150 => Some(KeyCode::Cut),
        151 => Some(KeyCode::Find),
        152 => Some(KeyCode::Open),
        153 => Some(KeyCode::Paste),
        154 => Some(KeyCode::Props),
        155 => Some(KeyCode::Select),
        156 => Some(KeyCode::Undo),
        157 => Some(KeyCode::Hiragana),
        158 => Some(KeyCode::Katakana),
        159 => Some(KeyCode::F1),
        160 => Some(KeyCode::F2),
        161 => Some(KeyCode::F3),
        162 => Some(KeyCode::F4),
        163 => Some(KeyCode::F5),
        164 => Some(KeyCode::F6),
        165 => Some(KeyCode::F7),
        166 => Some(KeyCode::F8),
        167 => Some(KeyCode::F9),
        168 => Some(KeyCode::F10),
        169 => Some(KeyCode::F11),
        170 => Some(KeyCode::F12),
        171 => Some(KeyCode::F13),
        172 => Some(KeyCode::F14),
        173 => Some(KeyCode::F15),
        174 => Some(KeyCode::F16),
        175 => Some(KeyCode::F17),
        176 => Some(KeyCode::F18),
        177 => Some(KeyCode::F19),
        178 => Some(KeyCode::F20),
        179 => Some(KeyCode::F21),
        180 => Some(KeyCode::F22),
        181 => Some(KeyCode::F23),
        182 => Some(KeyCode::F24),
        183 => Some(KeyCode::F25),
        184 => Some(KeyCode::F26),
        185 => Some(KeyCode::F27),
        186 => Some(KeyCode::F28),
        187 => Some(KeyCode::F29),
        188 => Some(KeyCode::F30),
        189 => Some(KeyCode::F31),
        190 => Some(KeyCode::F32),
        191 => Some(KeyCode::F33),
        192 => Some(KeyCode::F34),
        193 => Some(KeyCode::F35),
        _ => None,
    }
}

pub trait ResolveReference {
    /// This function attempts to resolve the [`ResourceReference`]
    /// (specifically the [`ResourceReferenceType::File`]) into
    /// a [`PathBuf`].
    ///
    /// It does this by checking if the app is the `eucalyptus-editor`
    /// through the `editor` flag, or the redback-runtime.
    ///
    /// It first resolves for the project config, and if that is not available
    /// it will resolve by comparing to the executable's directory.
    fn resolve(&self) -> anyhow::Result<PathBuf>;
}

impl ResolveReference for ResourceReference {
    fn resolve(&self) -> anyhow::Result<PathBuf> {
        match &self.ref_type {
            ResourceReferenceType::File(path) => {
                let relative = relative_path_from_euca(path)?;

                #[cfg(feature = "editor")]
                {
                    let project_config = {
                        use crate::states::PROJECT;

                        let cfg = PROJECT.read();
                        cfg.project_path.clone()
                    };

                    let path = project_config.join("resources").join(relative);
                    return Ok(path);
                }

                #[cfg(not(feature = "editor"))]
                {
                    let current_exe = std::env::current_exe()?;
                    let dir = current_exe
                        .parent()
                        .ok_or_else(|| anyhow::anyhow!("Unable to get path"))?;
                    return Ok(dir.join("resources").join(relative));
                }
            }
            _ => {
                anyhow::bail!("Cannot resolve any other ResourceReferenceType that is not File")
            }
        }
    }
}

/// Validates and converts a raw pointer to a reference.
/// Returns early if the pointer is null.
///
/// # Example
/// ```rust
/// use eucalyptus_core::{convert_ptr, ptr::AssetRegistryPtr};
/// use dropbear_engine::asset::AssetRegistry;
///
/// let asset_handle = 0x12345678; // pointer
///
/// let asset = convert_ptr!(asset_handle, AssetRegistryPtr => AssetRegistry);
/// ```
#[macro_export]
macro_rules! convert_ptr {
    ($ptr:expr, $ptr_ty:ty => $target_ty:ty) => {{
        let ptr = $ptr as $ptr_ty;
        if ptr.is_null() {
            let message = format!(
                "[{}] [ERROR] {} pointer is null",
                std::any::type_name::<$target_ty>(),
                stringify!($ptr)
            );
            println!("{}", message);
            return $crate::ffi_error_return!();
        }
        unsafe { &*(ptr as *const $target_ty) }
    }};

    ($ptr:expr => $target_ty:ty) => {{
        let ptr = $ptr as *const $target_ty;
        if ptr.is_null() {
            let message = format!(
                "[{}] [ERROR] {} pointer is null",
                std::any::type_name::<$target_ty>(),
                stringify!($ptr)
            );
            println!("{}", message);
            return $crate::ffi_error_return!();
        }
        unsafe { &*ptr }
    }};
}

/// Converts a JString to a Rust String with automatic error handling.
/// Automatically infers the appropriate error return value based on the function's return type.
///
/// # Usage
/// ```rust
/// convert_jstring!(env, jstring);
/// ```
#[macro_export]
macro_rules! convert_jstring {
    ($env:expr, $jstring:expr) => {{
        match $env.get_string(&$jstring) {
            Ok(java_string) => match java_string.to_str() {
                Ok(rust_str) => rust_str.to_string(),
                Err(e) => {
                    let message = format!(
                        "[{}] [ERROR] Failed to convert Java string to Rust string: {}",
                        stringify!($jstring),
                        e
                    );
                    println!("{}", message);
                    return $crate::ffi_error_return!();
                }
            },
            Err(e) => {
                let message = format!(
                    "[{}] [ERROR] Failed to get string from JNI: {}",
                    stringify!($jstring),
                    e
                );
                println!("{}", message);
                return $crate::ffi_error_return!();
            }
        }
    }};
}

#[macro_export]
macro_rules! ffi_error_return {
    () => {{
        trait ErrorValue {
            fn error_value() -> Self;
        }

        impl ErrorValue for () {
            fn error_value() -> Self {}
        }

        impl ErrorValue for i8 {
            fn error_value() -> Self {
                -1
            }
        }

        impl ErrorValue for i16 {
            fn error_value() -> Self {
                -1
            }
        }

        impl ErrorValue for i32 {
            fn error_value() -> Self {
                -1
            }
        }

        impl ErrorValue for i64 {
            fn error_value() -> Self {
                -1
            }
        }

        impl ErrorValue for isize {
            fn error_value() -> Self {
                -1
            }
        }

        impl ErrorValue for u8 {
            fn error_value() -> Self {
                0
            } // most likely a char or a jboolean
        }

        impl ErrorValue for u16 {
            fn error_value() -> Self {
                u16::MAX
            }
        }

        impl ErrorValue for u32 {
            fn error_value() -> Self {
                u32::MAX
            }
        }

        impl ErrorValue for u64 {
            fn error_value() -> Self {
                u64::MAX
            }
        }

        impl ErrorValue for usize {
            fn error_value() -> Self {
                usize::MAX
            }
        }

        impl<T> ErrorValue for *mut T {
            fn error_value() -> Self {
                std::ptr::null_mut()
            }
        }

        impl<T> ErrorValue for *const T {
            fn error_value() -> Self {
                std::ptr::null()
            }
        }

        impl<'local> ErrorValue for jni::objects::JObject<'local> {
            fn error_value() -> Self {
                jni::objects::JObject::null()
            }
        }

        impl ErrorValue for f64 {
            fn error_value() -> Self {
                f64::NAN
            }
        }

        // todo: implement other types

        ErrorValue::error_value()
    }};
}

/// Converts a jlong to a hecs::Entity with automatic error handling.
/// Returns from the function with an appropriate error value if the conversion fails.
///
/// # Usage
/// ```rust
/// let entity = convert_jlong_to_entity!(jlong_value);
/// ```
#[macro_export]
macro_rules! convert_jlong_to_entity {
    ($jlong:expr) => {{
        match hecs::Entity::from_bits($jlong as u64) {
            Some(entity) => entity,
            None => {
                let message = format!(
                    "[{}] [ERROR] Invalid bit pattern for entity provided: {}",
                    stringify!($jlong),
                    $jlong
                );
                println!("{}", message);
                return $crate::ffi_error_return!();
            }
        }
    }};
}
