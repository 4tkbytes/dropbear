use std::{collections::{HashMap, HashSet}, time::{Duration, Instant}};

use rustyscript::{serde_json, Runtime};
use serde::{Deserialize, Serialize};
use winit::{event::MouseButton, keyboard::KeyCode};

#[derive(Clone)]
pub struct InputState {
    #[allow(dead_code)]
    pub last_key_press_times: HashMap<KeyCode, Instant>,
    #[allow(dead_code)]
    pub double_press_threshold: Duration,
    pub mouse_pos: (f64, f64),
    pub mouse_button: HashSet<MouseButton>,    
    pub pressed_keys: HashSet<KeyCode>,
    pub mouse_delta: Option<(f64, f64)>,
    pub is_cursor_locked: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SerializableInputState {
    pub mouse_pos: (f64, f64),
    pub pressed_keys: Vec<String>,
    pub mouse_delta: Option<(f64, f64)>,
    pub is_cursor_locked: bool,
}

impl From<&InputState> for SerializableInputState {
    fn from(input_state: &InputState) -> Self {
        Self {
            mouse_pos: input_state.mouse_pos,
            pressed_keys: input_state.pressed_keys.iter()
                .filter_map(|key| keycode_to_string(*key))
                .collect(),
            mouse_delta: input_state.mouse_delta,
            is_cursor_locked: input_state.is_cursor_locked,
        }
    }
}

impl Default for InputState {
    fn default() -> Self {
        Self::new()
    }
}

impl InputState {
    pub fn new() -> Self {
        Self {
            mouse_pos: Default::default(),
            mouse_button: Default::default(),
            pressed_keys: HashSet::new(),
            last_key_press_times: HashMap::new(),
            double_press_threshold: Duration::from_millis(300),
            mouse_delta: None,
            is_cursor_locked: false,
        }
    }

    pub fn lock_cursor(&mut self, toggle: bool) {
        self.is_cursor_locked = toggle;
    }

    pub fn register_input_modules(runtime: &mut Runtime) -> anyhow::Result<()> {
        runtime.register_function("isKeyPressed", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 2 {
                return Err(rustyscript::Error::Runtime("isKeyPressed requires 2 arguments (inputState, keyCode)".to_string()));
            }
            
            let input_state: SerializableInputState = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid input state: {}", e)))?;
            
            let key_code_str = args[1].as_str()
                .ok_or_else(|| rustyscript::Error::Runtime("Key code must be a string".to_string()))?;
            
            let is_pressed = input_state.pressed_keys.contains(&key_code_str.to_string());
            Ok(serde_json::Value::Bool(is_pressed))
        })?;

        runtime.register_function("getMouseX", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 1 {
                return Err(rustyscript::Error::Runtime("getMouseX requires 1 argument (inputState)".to_string()));
            }
            
            let input_state: SerializableInputState = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid input state: {}", e)))?;
            
            Ok(serde_json::Value::Number(serde_json::Number::from_f64(input_state.mouse_pos.0).unwrap()))
        })?;

        runtime.register_function("getMouseY", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 1 {
                return Err(rustyscript::Error::Runtime("getMouseY requires 1 argument (inputState)".to_string()));
            }
            
            let input_state: SerializableInputState = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid input state: {}", e)))?;
            
            Ok(serde_json::Value::Number(serde_json::Number::from_f64(input_state.mouse_pos.1).unwrap()))
        })?;

        runtime.register_function("getMouseDeltaX", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 1 {
                return Err(rustyscript::Error::Runtime("getMouseDeltaX requires 1 argument (inputState)".to_string()));
            }
            
            let input_state: SerializableInputState = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid input state: {}", e)))?;
            
            let delta_x = input_state.mouse_delta.map(|d| d.0).unwrap_or(0.0);
            Ok(serde_json::Value::Number(serde_json::Number::from_f64(delta_x).unwrap()))
        })?;

        runtime.register_function("getMouseDeltaY", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 1 {
                return Err(rustyscript::Error::Runtime("getMouseDeltaY requires 1 argument (inputState)".to_string()));
            }
            
            let input_state: SerializableInputState = serde_json::from_value(args[0].clone())
                .map_err(|e| rustyscript::Error::Runtime(format!("Invalid input state: {}", e)))?;
            
            let delta_y = input_state.mouse_delta.map(|d| d.1).unwrap_or(0.0);
            Ok(serde_json::Value::Number(serde_json::Number::from_f64(delta_y).unwrap()))
        })?;

        runtime.register_function("lockCursor", |args: &[serde_json::Value]| -> Result<serde_json::Value, rustyscript::Error> {
            if args.len() != 1 {
                return Err(rustyscript::Error::Runtime("lockCursor requires 1 argument (locked)".to_string()));
            }
            
            let _locked = args[0].as_bool()
                .ok_or_else(|| rustyscript::Error::Runtime("Locked parameter must be a boolean".to_string()))?;
            
            // This function would need to communicate back to the engine to actually lock the cursor
            // For now, just return success
            Ok(serde_json::Value::Bool(true))
        })?;

        log::info!("[Script] Initialised input module");
        Ok(())
    }
}

/// Helper function to convert string to KeyCode
#[allow(dead_code)]
fn string_to_keycode(key_str: &str) -> Option<KeyCode> {
    match key_str {
        "KeyA" => Some(KeyCode::KeyA),
        "KeyB" => Some(KeyCode::KeyB),
        "KeyC" => Some(KeyCode::KeyC),
        "KeyD" => Some(KeyCode::KeyD),
        "KeyE" => Some(KeyCode::KeyE),
        "KeyF" => Some(KeyCode::KeyF),
        "KeyG" => Some(KeyCode::KeyG),
        "KeyH" => Some(KeyCode::KeyH),
        "KeyI" => Some(KeyCode::KeyI),
        "KeyJ" => Some(KeyCode::KeyJ),
        "KeyK" => Some(KeyCode::KeyK),
        "KeyL" => Some(KeyCode::KeyL),
        "KeyM" => Some(KeyCode::KeyM),
        "KeyN" => Some(KeyCode::KeyN),
        "KeyO" => Some(KeyCode::KeyO),
        "KeyP" => Some(KeyCode::KeyP),
        "KeyQ" => Some(KeyCode::KeyQ),
        "KeyR" => Some(KeyCode::KeyR),
        "KeyS" => Some(KeyCode::KeyS),
        "KeyT" => Some(KeyCode::KeyT),
        "KeyU" => Some(KeyCode::KeyU),
        "KeyV" => Some(KeyCode::KeyV),
        "KeyW" => Some(KeyCode::KeyW),
        "KeyX" => Some(KeyCode::KeyX),
        "KeyY" => Some(KeyCode::KeyY),
        "KeyZ" => Some(KeyCode::KeyZ),
        "Space" => Some(KeyCode::Space),
        "ShiftLeft" => Some(KeyCode::ShiftLeft),
        "ShiftRight" => Some(KeyCode::ShiftRight),
        "ControlLeft" => Some(KeyCode::ControlLeft),
        "ControlRight" => Some(KeyCode::ControlRight),
        "AltLeft" => Some(KeyCode::AltLeft),
        "AltRight" => Some(KeyCode::AltRight),
        "Escape" => Some(KeyCode::Escape),
        "Enter" => Some(KeyCode::Enter),
        "Tab" => Some(KeyCode::Tab),
        "ArrowUp" => Some(KeyCode::ArrowUp),
        "ArrowDown" => Some(KeyCode::ArrowDown),
        "ArrowLeft" => Some(KeyCode::ArrowLeft),
        "ArrowRight" => Some(KeyCode::ArrowRight),
        "Digit0" => Some(KeyCode::Digit0),
        "Digit1" => Some(KeyCode::Digit1),
        "Digit2" => Some(KeyCode::Digit2),
        "Digit3" => Some(KeyCode::Digit3),
        "Digit4" => Some(KeyCode::Digit4),
        "Digit5" => Some(KeyCode::Digit5),
        "Digit6" => Some(KeyCode::Digit6),
        "Digit7" => Some(KeyCode::Digit7),
        "Digit8" => Some(KeyCode::Digit8),
        "Digit9" => Some(KeyCode::Digit9),
        "F1" => Some(KeyCode::F1),
        "F2" => Some(KeyCode::F2),
        "F3" => Some(KeyCode::F3),
        "F4" => Some(KeyCode::F4),
        "F5" => Some(KeyCode::F5),
        "F6" => Some(KeyCode::F6),
        "F7" => Some(KeyCode::F7),
        "F8" => Some(KeyCode::F8),
        "F9" => Some(KeyCode::F9),
        "F10" => Some(KeyCode::F10),
        "F11" => Some(KeyCode::F11),
        "F12" => Some(KeyCode::F12),
        _ => None,
    }
}

/// Helper function to convert KeyCode to string
fn keycode_to_string(key_code: KeyCode) -> Option<String> {
    match key_code {
        KeyCode::KeyA => Some("KeyA".to_string()),
        KeyCode::KeyB => Some("KeyB".to_string()),
        KeyCode::KeyC => Some("KeyC".to_string()),
        KeyCode::KeyD => Some("KeyD".to_string()),
        KeyCode::KeyE => Some("KeyE".to_string()),
        KeyCode::KeyF => Some("KeyF".to_string()),
        KeyCode::KeyG => Some("KeyG".to_string()),
        KeyCode::KeyH => Some("KeyH".to_string()),
        KeyCode::KeyI => Some("KeyI".to_string()),
        KeyCode::KeyJ => Some("KeyJ".to_string()),
        KeyCode::KeyK => Some("KeyK".to_string()),
        KeyCode::KeyL => Some("KeyL".to_string()),
        KeyCode::KeyM => Some("KeyM".to_string()),
        KeyCode::KeyN => Some("KeyN".to_string()),
        KeyCode::KeyO => Some("KeyO".to_string()),
        KeyCode::KeyP => Some("KeyP".to_string()),
        KeyCode::KeyQ => Some("KeyQ".to_string()),
        KeyCode::KeyR => Some("KeyR".to_string()),
        KeyCode::KeyS => Some("KeyS".to_string()),
        KeyCode::KeyT => Some("KeyT".to_string()),
        KeyCode::KeyU => Some("KeyU".to_string()),
        KeyCode::KeyV => Some("KeyV".to_string()),
        KeyCode::KeyW => Some("KeyW".to_string()),
        KeyCode::KeyX => Some("KeyX".to_string()),
        KeyCode::KeyY => Some("KeyY".to_string()),
        KeyCode::KeyZ => Some("KeyZ".to_string()),
        KeyCode::Space => Some("Space".to_string()),
        KeyCode::ShiftLeft => Some("ShiftLeft".to_string()),
        KeyCode::ShiftRight => Some("ShiftRight".to_string()),
        KeyCode::ControlLeft => Some("ControlLeft".to_string()),
        KeyCode::ControlRight => Some("ControlRight".to_string()),
        KeyCode::AltLeft => Some("AltLeft".to_string()),
        KeyCode::AltRight => Some("AltRight".to_string()),
        KeyCode::Escape => Some("Escape".to_string()),
        KeyCode::Enter => Some("Enter".to_string()),
        KeyCode::Tab => Some("Tab".to_string()),
        KeyCode::ArrowUp => Some("ArrowUp".to_string()),
        KeyCode::ArrowDown => Some("ArrowDown".to_string()),
        KeyCode::ArrowLeft => Some("ArrowLeft".to_string()),
        KeyCode::ArrowRight => Some("ArrowRight".to_string()),
        KeyCode::Digit0 => Some("Digit0".to_string()),
        KeyCode::Digit1 => Some("Digit1".to_string()),
        KeyCode::Digit2 => Some("Digit2".to_string()),
        KeyCode::Digit3 => Some("Digit3".to_string()),
        KeyCode::Digit4 => Some("Digit4".to_string()),
        KeyCode::Digit5 => Some("Digit5".to_string()),
        KeyCode::Digit6 => Some("Digit6".to_string()),
        KeyCode::Digit7 => Some("Digit7".to_string()),
        KeyCode::Digit8 => Some("Digit8".to_string()),
        KeyCode::Digit9 => Some("Digit9".to_string()),
        KeyCode::F1 => Some("F1".to_string()),
        KeyCode::F2 => Some("F2".to_string()),
        KeyCode::F3 => Some("F3".to_string()),
        KeyCode::F4 => Some("F4".to_string()),
        KeyCode::F5 => Some("F5".to_string()),
        KeyCode::F6 => Some("F6".to_string()),
        KeyCode::F7 => Some("F7".to_string()),
        KeyCode::F8 => Some("F8".to_string()),
        KeyCode::F9 => Some("F9".to_string()),
        KeyCode::F10 => Some("F10".to_string()),
        KeyCode::F11 => Some("F11".to_string()),
        KeyCode::F12 => Some("F12".to_string()),
        _ => None,
    }
}

#[derive(Clone, Copy)]
pub struct Key(pub KeyCode);