use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};
use winit::{event::MouseButton, keyboard::KeyCode};
use dropbear_engine::gilrs::{Button, GamepadId};

#[derive(Clone, Debug)]
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
    pub last_mouse_pos: Option<(f64, f64)>,

    pub connected_gamepads: HashSet<GamepadId>,
    pub pressed_buttons: HashMap<GamepadId, HashSet<Button>>,
    pub left_stick_position: HashMap<GamepadId, (f32, f32)>,
    pub right_stick_position: HashMap<GamepadId, (f32, f32)>,
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
            last_mouse_pos: Default::default(),
            connected_gamepads: Default::default(),
            pressed_buttons: Default::default(),
            left_stick_position: Default::default(),
            right_stick_position: Default::default(),
        }
    }

    pub fn lock_cursor(&mut self, toggle: bool) {
        self.is_cursor_locked = toggle;
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    #[allow(clippy::unnecessary_map_or)]
    pub fn is_button_pressed(&self, gamepad_id: GamepadId, button: Button) -> bool {
        self.pressed_buttons
            .get(&gamepad_id)
            .map_or(false, |buttons| buttons.contains(&button))
    }

    pub fn get_left_stick(&self, gamepad_id: GamepadId) -> (f32, f32) {
        self.left_stick_position
            .get(&gamepad_id)
            .copied()
            .unwrap_or((0.0, 0.0))
    }

    pub fn get_right_stick(&self, gamepad_id: GamepadId) -> (f32, f32) {
        self.right_stick_position
            .get(&gamepad_id)
            .copied()
            .unwrap_or((0.0, 0.0))
    }

    pub fn is_gamepad_connected(&self, gamepad_id: GamepadId) -> bool {
        self.connected_gamepads.contains(&gamepad_id)
    }
}
