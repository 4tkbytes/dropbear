use dropbear_engine::{
    async_trait::async_trait,
    graphics::Graphics,
    input::{Controller, Keyboard, Mouse},
    scene::{Scene, SceneCommand},
    winit::event_loop::ActiveEventLoop,
};

#[derive(Default)]
pub struct Editor {
    scene_command: SceneCommand,
}

impl Editor {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Scene for Editor {
    async fn load(&mut self, graphics: &mut Graphics) {}
    async fn update(&mut self, dt: f32, graphics: &mut Graphics) {}
    async fn render(&mut self, graphics: &mut Graphics) {}
    async fn exit(&mut self, event_loop: &ActiveEventLoop) {}

    fn run_command(&mut self) -> SceneCommand {
        std::mem::replace(&mut self.scene_command, SceneCommand::None)
    }
}

impl Keyboard for Editor {
    fn key_down(
        &mut self,
        key: dropbear_engine::winit::keyboard::KeyCode,
        event_loop: &dropbear_engine::winit::event_loop::ActiveEventLoop,
    ) {
    }

    fn key_up(
        &mut self,
        key: dropbear_engine::winit::keyboard::KeyCode,
        event_loop: &dropbear_engine::winit::event_loop::ActiveEventLoop,
    ) {
    }
}

impl Mouse for Editor {
    fn mouse_move(&mut self, position: dropbear_engine::winit::dpi::PhysicalPosition<f64>) {}

    fn mouse_down(&mut self, button: dropbear_engine::winit::event::MouseButton) {}

    fn mouse_up(&mut self, button: dropbear_engine::winit::event::MouseButton) {}
}

impl Controller for Editor {
    fn button_down(
        &mut self,
        button: dropbear_engine::gilrs::Button,
        id: dropbear_engine::gilrs::GamepadId,
    ) {
    }

    fn button_up(
        &mut self,
        button: dropbear_engine::gilrs::Button,
        id: dropbear_engine::gilrs::GamepadId,
    ) {
    }

    fn left_stick_changed(&mut self, x: f32, y: f32, id: dropbear_engine::gilrs::GamepadId) {}

    fn right_stick_changed(&mut self, x: f32, y: f32, id: dropbear_engine::gilrs::GamepadId) {}

    fn on_connect(&mut self, id: dropbear_engine::gilrs::GamepadId) {}

    fn on_disconnect(&mut self, id: dropbear_engine::gilrs::GamepadId) {}
}
