use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
use winit::{
    dpi::PhysicalPosition, event::MouseButton, event_loop::ActiveEventLoop, keyboard::KeyCode,
};

pub type KeyboardImpl = Rc<RefCell<dyn Keyboard>>;
pub type MouseImpl = Rc<RefCell<dyn Mouse>>;

pub trait Keyboard {
    fn key_down(&mut self, key: KeyCode, event_loop: &ActiveEventLoop);
    fn key_up(&mut self, key: KeyCode, event_loop: &ActiveEventLoop);
}

pub trait Mouse {
    fn mouse_move(&mut self, position: PhysicalPosition<f64>);
    fn mouse_down(&mut self, button: MouseButton);
    fn mouse_up(&mut self, button: MouseButton);
}

pub struct Manager {
    pressed_keys: HashSet<KeyCode>,
    just_pressed_keys: HashSet<KeyCode>,
    just_released_keys: HashSet<KeyCode>,

    pressed_mouse_buttons: HashSet<MouseButton>,
    just_pressed_mouse_buttons: HashSet<MouseButton>,
    just_released_mouse_buttons: HashSet<MouseButton>,
    mouse_position: PhysicalPosition<f64>,

    input_handlers: HashMap<String, KeyboardImpl>,
    mouse_handlers: HashMap<String, MouseImpl>,
}

impl Manager {
    pub fn new() -> Self {
        Self {
            pressed_keys: HashSet::new(),
            just_pressed_keys: HashSet::new(),
            just_released_keys: HashSet::new(),
            pressed_mouse_buttons: HashSet::new(),
            just_pressed_mouse_buttons: HashSet::new(),
            just_released_mouse_buttons: HashSet::new(),
            mouse_position: PhysicalPosition::new(0.0, 0.0),
            input_handlers: HashMap::new(),
            mouse_handlers: HashMap::new(),
        }
    }

    pub fn add_keyboard(&mut self, name: &str, handler: KeyboardImpl) {
        self.input_handlers.insert(name.to_string(), handler);
    }

    pub fn add_mouse(&mut self, name: &str, handler: MouseImpl) {
        self.mouse_handlers.insert(name.to_string(), handler);
    }

    pub fn handle_key_input(&mut self, key: KeyCode, pressed: bool, event_loop: &ActiveEventLoop) {
        if pressed {
            if !self.pressed_keys.contains(&key) {
                self.just_pressed_keys.insert(key);
                for handler in self.input_handlers.values_mut() {
                    handler.borrow_mut().key_down(key, event_loop);
                }
            }
            self.pressed_keys.insert(key);
        } else {
            if self.pressed_keys.contains(&key) {
                self.just_released_keys.insert(key);
                for handler in self.input_handlers.values_mut() {
                    handler.borrow_mut().key_up(key, event_loop);
                }
            }
            self.pressed_keys.remove(&key);
        }
    }

    pub fn handle_mouse_input(&mut self, button: MouseButton, pressed: bool) {
        if pressed {
            if !self.pressed_mouse_buttons.contains(&button) {
                self.just_pressed_mouse_buttons.insert(button);
                for handler in self.mouse_handlers.values_mut() {
                    handler.borrow_mut().mouse_down(button);
                }
            }
            self.pressed_mouse_buttons.insert(button);
        } else {
            if self.pressed_mouse_buttons.contains(&button) {
                self.just_released_mouse_buttons.insert(button);
                for handler in self.mouse_handlers.values_mut() {
                    handler.borrow_mut().mouse_up(button);
                }
            }
            self.pressed_mouse_buttons.remove(&button);
        }
    }

    pub fn handle_mouse_movement(&mut self, position: PhysicalPosition<f64>) {
        self.mouse_position = position;
        for handler in self.mouse_handlers.values_mut() {
            handler.borrow_mut().mouse_move(position);
        }
    }

    // Keyboard query methods
    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed_keys.contains(&key)
    }

    pub fn is_key_just_released(&self, key: KeyCode) -> bool {
        self.just_released_keys.contains(&key)
    }

    // Mouse query methods
    pub fn is_mouse_button_pressed(&self, button: MouseButton) -> bool {
        self.pressed_mouse_buttons.contains(&button)
    }

    pub fn is_mouse_button_just_pressed(&self, button: MouseButton) -> bool {
        self.just_pressed_mouse_buttons.contains(&button)
    }

    pub fn is_mouse_button_just_released(&self, button: MouseButton) -> bool {
        self.just_released_mouse_buttons.contains(&button)
    }

    pub fn get_mouse_position(&self) -> PhysicalPosition<f64> {
        self.mouse_position
    }

    pub fn update(&mut self) {
        // Clear just pressed/released keys and mouse buttons at the end of each frame
        self.just_pressed_keys.clear();
        self.just_released_keys.clear();
        self.just_pressed_mouse_buttons.clear();
        self.just_released_mouse_buttons.clear();
    }
}
