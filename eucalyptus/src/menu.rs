use dropbear_engine::{async_trait::async_trait, egui, gilrs, input::{Controller, Keyboard, Mouse}, log::debug, scene::{Scene, SceneCommand}};

#[derive(Default)]
pub struct MainMenu {
    scene_command: SceneCommand,
}

impl MainMenu {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }
}

#[async_trait]
impl Scene for MainMenu {
    async fn load(&mut self, _graphics: &mut dropbear_engine::graphics::Graphics) {

    }

    async fn update(&mut self, _dt: f32, _graphics: &mut dropbear_engine::graphics::Graphics) {
        
    }

    async fn render(&mut self, graphics: &mut dropbear_engine::graphics::Graphics) {
        egui::CentralPanel::default().show(graphics.get_egui_context(), |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(100.0);
                ui.heading("Welcome to Eucalyptus!");
                ui.add_space(40.0);

                let button_size = egui::vec2(300.0, 60.0); // width, height

                if ui.add_sized(button_size, egui::Button::new("Start")).clicked() {
                    self.scene_command = SceneCommand::SwitchScene("testing_scene_1".to_string());
                }
                ui.add_space(20.0);
                if ui.add_sized(button_size, egui::Button::new("Quit")).clicked() {
                    self.scene_command = SceneCommand::Quit
                }
            });
        });
    }
    
    async fn exit(&mut self, _event_loop: &dropbear_engine::winit::event_loop::ActiveEventLoop) {}

    fn run_command(&mut self) -> SceneCommand {
        std::mem::replace(&mut self.scene_command, SceneCommand::None)
    }
}

impl Keyboard for MainMenu {
    fn key_down(&mut self, key: dropbear_engine::winit::keyboard::KeyCode, event_loop: &dropbear_engine::winit::event_loop::ActiveEventLoop) {
        if key == dropbear_engine::winit::keyboard::KeyCode::Escape {
            event_loop.exit();
        }
    }

    fn key_up(&mut self, _key: dropbear_engine::winit::keyboard::KeyCode, _event_loop: &dropbear_engine::winit::event_loop::ActiveEventLoop) {

    }
}

impl Mouse for MainMenu {
    fn mouse_move(&mut self, _position: dropbear_engine::winit::dpi::PhysicalPosition<f64>) {

    }

    fn mouse_down(&mut self, _button: dropbear_engine::winit::event::MouseButton) {

    }

    fn mouse_up(&mut self, _button: dropbear_engine::winit::event::MouseButton) {

    }
}

impl Controller for MainMenu {
    fn button_down(&mut self, button: gilrs::Button, id: gilrs::GamepadId) {
        debug!("Controller button {:?} pressed! [{}]", button, id);
    }

    fn button_up(&mut self, button: gilrs::Button, id: gilrs::GamepadId) {
        debug!("Controller button {:?} released! [{}]", button, id);
    }

    fn left_stick_changed(&mut self, x: f32, y: f32, id: gilrs::GamepadId) {
        debug!("Left stick changed: x = {} | y = {} | id = {}", x, y, id);
    }

    fn right_stick_changed(&mut self, x: f32, y: f32, id: gilrs::GamepadId) {
        debug!("Right stick changed: x = {} | y = {} | id = {}", x, y, id);
    }

    fn on_connect(&mut self, id: gilrs::GamepadId) {
        debug!("Controller connected [{}]", id);
    }

    fn on_disconnect(&mut self, id: gilrs::GamepadId) {
        debug!("Controller disconnected [{}]", id);
    }
}