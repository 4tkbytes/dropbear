use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use winit::{event::MouseButton, keyboard::KeyCode, platform::scancode::PhysicalKeyExtScancode};

use crate::scripting::{DropbearScriptingAPIContext};

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
    pub last_mouse_pos: Option<(f64, f64)>,
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
        }
    }

    pub fn lock_cursor(&mut self, toggle: bool) {
        self.is_cursor_locked = toggle;
    }

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }
}