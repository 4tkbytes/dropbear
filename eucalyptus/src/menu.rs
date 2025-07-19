use dropbear_engine::{async_trait::async_trait, egui, input::{Keyboard, Mouse}, scene::Scene};

#[derive(Default)]
pub struct MainMenu {
    switch_to: Option<String>,
    _send_exit_sig: bool,
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
                    self.switch_to = Some("testing_scene_1".to_string());
                }
                ui.add_space(20.0);
                if ui.add_sized(button_size, egui::Button::new("Quit")).clicked() {
                    // fix this up
                }
            });
        });
    }
    
    async fn exit(&mut self, _event_loop: &dropbear_engine::winit::event_loop::ActiveEventLoop) {}

    fn requested_switch(&mut self) -> Option<String> {
        self.switch_to.take()
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