use dropbear_engine::{
    async_trait::async_trait,
    egui::{self, FontId, Frame, RichText},
    gilrs,
    input::{Controller, Keyboard, Mouse},
    log::{self, debug},
    scene::{Scene, SceneCommand},
};

#[derive(Default)]
pub struct MainMenu {
    scene_command: SceneCommand,
    show_new_project: bool,
    project_name: String,
    project_path: Option<std::path::PathBuf>,
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
    async fn load(&mut self, _graphics: &mut dropbear_engine::graphics::Graphics) {}

    async fn update(&mut self, _dt: f32, _graphics: &mut dropbear_engine::graphics::Graphics) {}

    async fn render(&mut self, graphics: &mut dropbear_engine::graphics::Graphics) {
        let screen_size: (f32, f32) = (
            graphics.state.window.inner_size().width as f32 - 100.0,
            graphics.state.window.inner_size().height as f32 - 100.0,
        );
        let egui_ctx = graphics.get_egui_context();

        egui::CentralPanel::default()
            .frame(Frame::new())
            .show(egui_ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(64.0);
                    ui.label(RichText::new("Eucalyptus").font(FontId::proportional(32.0)));
                    ui.add_space(40.0);

                    let button_size = egui::vec2(300.0, 60.0);

                    if ui
                        .add_sized(button_size, egui::Button::new("New Project"))
                        .clicked()
                    {
                        log::debug!("Creating new project");
                        self.show_new_project = true;
                    }
                    ui.add_space(20.0);

                    if ui
                        .add_sized(button_size, egui::Button::new("Open Project"))
                        .clicked()
                    {
                        log::debug!("Opening project (not implemented)");
                    }
                    ui.add_space(20.0);

                    if ui
                        .add_sized(button_size, egui::Button::new("Settings"))
                        .clicked()
                    {
                        log::debug!("Settings");
                    }
                    ui.add_space(20.0);

                    if ui
                        .add_sized(button_size, egui::Button::new("Quit"))
                        .clicked()
                    {
                        self.scene_command = SceneCommand::Quit
                    }
                    ui.add_space(20.0);
                });
            });

        egui::Window::new("Create new project")
            .open(&mut self.show_new_project)
            .resizable(true)
            .collapsible(false)
            .fixed_size(screen_size)
            .show(egui_ctx, |ui| {
                ui.vertical(|ui| {
                    ui.label("Project Name:");
                    ui.add_space(5.0);
                    ui.text_edit_singleline(&mut self.project_name);
                    ui.add_space(10.0);
                    ui.label(format!("\"{}\" Project Location", self.project_name));
                    ui.add_space(5.0);

                    if let Some(ref path) = self.project_path {
                        ui.label(format!("Chosen location: {}", path.display()));
                        ui.add_space(5.0);
                    }

                    ui.add_space(5.0);
                    if ui.button("Choose Location").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_title("Save Project")
                            .set_file_name(&self.project_name)
                            .pick_folder()
                        {
                            self.project_path = Some(path);
                            log::debug!("Project will be saved at: {:?}", self.project_path);
                        }
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
    fn key_down(
        &mut self,
        key: dropbear_engine::winit::keyboard::KeyCode,
        event_loop: &dropbear_engine::winit::event_loop::ActiveEventLoop,
    ) {
        if key == dropbear_engine::winit::keyboard::KeyCode::Escape {
            event_loop.exit();
        }
    }

    fn key_up(
        &mut self,
        _key: dropbear_engine::winit::keyboard::KeyCode,
        _event_loop: &dropbear_engine::winit::event_loop::ActiveEventLoop,
    ) {
    }
}

impl Mouse for MainMenu {
    fn mouse_move(&mut self, _position: dropbear_engine::winit::dpi::PhysicalPosition<f64>) {}

    fn mouse_down(&mut self, _button: dropbear_engine::winit::event::MouseButton) {}

    fn mouse_up(&mut self, _button: dropbear_engine::winit::event::MouseButton) {}
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
