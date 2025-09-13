use std::{
    collections::{HashMap, HashSet},
    time::{Duration, Instant},
};

use wasmer::{Function, FunctionEnvMut};
use winit::{event::MouseButton, keyboard::KeyCode, platform::scancode::PhysicalKeyExtScancode};

use crate::scripting::{DropbearScriptingAPIContext, ScriptableModuleWithEnv};

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

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }
}

impl ScriptableModuleWithEnv for InputState {
    type T = DropbearScriptingAPIContext;

    fn register(env: &wasmer::FunctionEnv<Self::T>, imports: &mut wasmer::Imports, store: &mut wasmer::Store) -> anyhow::Result<()> {
        fn is_key_pressed_impl(env: FunctionEnvMut<DropbearScriptingAPIContext>, key_code: u32) -> u32 {
            if let Some(input) = env.data().get_input() {
                match KeyCode::from_scancode(key_code) {
                    winit::keyboard::PhysicalKey::Code(key_code) => {
                        if input.is_key_pressed(key_code) { 1 } else { 0 }
                    },
                    winit::keyboard::PhysicalKey::Unidentified(_) => 0,
                }
            } else {
                0
            }
        }
        let is_key_pressed = Function::new_typed_with_env(store, &env, is_key_pressed_impl);

        fn get_mouse_position_impl(env: FunctionEnvMut<DropbearScriptingAPIContext>) -> (f64, f64) {
            if let Some(input) = env.data().get_input() {
                input.mouse_pos
            } else {
                (0.0, 0.0)
            }
        }
        let get_mouse_position = Function::new_typed_with_env(store, &env, get_mouse_position_impl);

        imports.define(Self::module_name(), "getMousePosition", get_mouse_position);
        imports.define(Self::module_name(), "isKeyPressed", is_key_pressed);

        Ok(())
    }

    fn module_name() -> &'static str {
        "dropbear_input"
    }
}
