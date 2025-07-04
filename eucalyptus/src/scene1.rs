use dropbear_engine::{input::{Keyboard, Mouse}, log::{self, debug}, scene::Scene, winit::{event, event_loop::{self, ActiveEventLoop}, keyboard::KeyCode}};

pub struct TestingScene1;

impl TestingScene1 {
    pub fn new() -> Self {
        Self
    }
}

impl Scene for TestingScene1 {
    fn load(&mut self) {
        debug!("TestingScene1 loaded!");
    }

    fn update(&mut self, dt: f32) {
        // Scene update logic here
    }

    fn render(&mut self) {
        // Scene rendering logic here
    }

    fn exit(&mut self) {
        debug!("TestingScene1 exited!");
    }
}

impl Keyboard for TestingScene1 {
    fn key_down(&mut self, key: KeyCode, event_loop: &ActiveEventLoop) {
        debug!("Key pressed: {:?}", key);
        match key {
            KeyCode::Escape => event_loop.exit(),
            _ => {}
        }
    }

    fn key_up(&mut self, key: KeyCode, event_loop: &ActiveEventLoop) {
        debug!("Key released: {:?}", key);
    }
}

impl Mouse for TestingScene1 {

    fn mouse_down(&mut self, button: dropbear_engine::winit::event::MouseButton) {
        debug!("Mouse button pressed: {:?}", button)
    }

    fn mouse_up(&mut self, button: dropbear_engine::winit::event::MouseButton) {
        debug!("Mouse button released: {:?}", button);
    }
    
    fn mouse_move(&mut self, position: dropbear_engine::winit::dpi::PhysicalPosition<f64>) {
        debug!("Mouse position: {}, {}", position.x, position.y)
    }
}