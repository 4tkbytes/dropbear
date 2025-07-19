use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};
use gilrs::{Axis, EventType, Gilrs};
use winit::{
    dpi::PhysicalPosition, event::MouseButton, event_loop::ActiveEventLoop, keyboard::KeyCode,
};

pub type KeyboardImpl = Rc<RefCell<dyn Keyboard>>;
pub type MouseImpl = Rc<RefCell<dyn Mouse>>;
pub type ControllerImpl = Rc<RefCell<dyn Controller>>;

pub trait Keyboard {
    fn key_down(&mut self, key: KeyCode, event_loop: &ActiveEventLoop);
    fn key_up(&mut self, key: KeyCode, event_loop: &ActiveEventLoop);
}

pub trait Mouse {
    fn mouse_move(&mut self, position: PhysicalPosition<f64>);
    fn mouse_down(&mut self, button: MouseButton);
    fn mouse_up(&mut self, button: MouseButton);
}

pub trait Controller {
    fn button_down(&mut self, button: gilrs::Button, id: gilrs::GamepadId);
    fn button_up(&mut self, button: gilrs::Button, id: gilrs::GamepadId);
    fn left_stick_changed(&mut self, x: f32, y: f32, id: gilrs::GamepadId);
    fn right_stick_changed(&mut self, x: f32, y: f32, id: gilrs::GamepadId);
    fn on_connect(&mut self, id: gilrs::GamepadId);
    fn on_disconnect(&mut self, id: gilrs::GamepadId);
}

pub struct Manager {
    // keyboard
    pressed_keys: HashSet<KeyCode>,
    just_pressed_keys: HashSet<KeyCode>,
    just_released_keys: HashSet<KeyCode>,

    // mouse
    pressed_mouse_buttons: HashSet<MouseButton>,
    just_pressed_mouse_buttons: HashSet<MouseButton>,
    just_released_mouse_buttons: HashSet<MouseButton>,
    mouse_position: PhysicalPosition<f64>,

    keyboard_handlers: HashMap<String, KeyboardImpl>,
    mouse_handlers: HashMap<String, MouseImpl>,
    controller_handlers: HashMap<String, ControllerImpl>,
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
            keyboard_handlers: HashMap::new(),
            mouse_handlers: HashMap::new(),
            controller_handlers: HashMap::new(),
        }
    }

    pub fn add_keyboard(&mut self, name: &str, handler: KeyboardImpl) {
        self.keyboard_handlers.insert(name.to_string(), handler);
    }

    pub fn add_mouse(&mut self, name: &str, handler: MouseImpl) {
        self.mouse_handlers.insert(name.to_string(), handler);
    }

    pub fn handle_key_input(&mut self, key: KeyCode, pressed: bool, event_loop: &ActiveEventLoop) {
        if pressed {
            if !self.pressed_keys.contains(&key) {
                self.just_pressed_keys.insert(key);
                for handler in self.keyboard_handlers.values_mut() {
                    handler.borrow_mut().key_down(key, event_loop);
                }
            }
            self.pressed_keys.insert(key);
        } else {
            if self.pressed_keys.contains(&key) {
                self.just_released_keys.insert(key);
                for handler in self.keyboard_handlers.values_mut() {
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

    pub fn is_key_pressed(&self, key: KeyCode) -> bool {
        self.pressed_keys.contains(&key)
    }

    pub fn is_key_just_pressed(&self, key: KeyCode) -> bool {
        self.just_pressed_keys.contains(&key)
    }

    pub fn is_key_just_released(&self, key: KeyCode) -> bool {
        self.just_released_keys.contains(&key)
    }

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

    pub fn update(&mut self, gilrs: &mut Gilrs) {
        self.just_pressed_keys.clear();
        self.just_released_keys.clear();
        self.just_pressed_mouse_buttons.clear();
        self.just_released_mouse_buttons.clear();
        self.poll_controllers(gilrs);
    }

    pub fn add_controller(&mut self, name: &str, handler: ControllerImpl) {
        self.controller_handlers.insert(name.to_string(), handler);
    }

    pub fn handle_controller_event(&mut self, event: gilrs::Event) {
        for handler in self.controller_handlers.values_mut() {
            match event.event {
                EventType::ButtonPressed(button, _) => {
                    handler.borrow_mut().button_down(button, event.id);
                }
                EventType::ButtonReleased(button, _) => {
                    handler.borrow_mut().button_up(button, event.id);
                }
                EventType::AxisChanged(Axis::LeftStickX, x, _) => {
                    // You may want to cache Y and call only when both are updated
                    handler.borrow_mut().left_stick_changed(x, 0.0, event.id);
                }
                EventType::AxisChanged(Axis::LeftStickY, y, _) => {
                    handler.borrow_mut().left_stick_changed(0.0, y, event.id);
                }
                EventType::AxisChanged(Axis::RightStickX, x, _) => {
                    handler.borrow_mut().right_stick_changed(x, 0.0, event.id);
                }
                EventType::AxisChanged(Axis::RightStickY, y, _) => {
                    handler.borrow_mut().right_stick_changed(0.0, y, event.id);
                }
                EventType::Connected => {
                    handler.borrow_mut().on_connect(event.id);
                }
                EventType::Disconnected => {
                    handler.borrow_mut().on_disconnect(event.id);
                }
                _ => {}
            }
        }
    }

    pub fn poll_controllers(&mut self, gilrs: &mut gilrs::Gilrs) {
        while let Some(event) = gilrs.next_event() {
            self.handle_controller_event(event);
        }
    }
}
