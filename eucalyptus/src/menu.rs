use std::{fs, path::PathBuf};

use chrono::Utc;
use dropbear_engine::{
    async_trait::async_trait,
    egui::{self, FontId, Frame, RichText},
    gilrs,
    input::{Controller, Keyboard, Mouse},
    log::{self, debug},
    scene::{Scene, SceneCommand},
};
use git2::Repository;
use serde::{Deserialize, Serialize};

#[derive(Default)]
pub struct MainMenu {
    scene_command: SceneCommand,
    show_new_project: bool,
    project_name: String,
    project_path: Option<std::path::PathBuf>,
    project_created: bool,
    project_error: Option<Vec<String>>,

    show_progress: bool,
    progress: f32,
    progress_message: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ProjectConfig {
    project_name: String,
    project_path: String,
    date_created: String,
}

impl ProjectConfig {
    pub fn from(project_name: String, project_path: &PathBuf) -> Self {
        let date_created = format!("{}", Utc::now().format("%Y-%m-%d %H:%M:%S"));
        let project_path = project_path.to_str().unwrap().to_string();
        Self {
            project_name,
            project_path,
            date_created,
        }
    }
}

impl MainMenu {
    pub fn new() -> Self {
        Self {
            show_progress: false,
            ..Default::default()
        }
    }

    fn start_project_creation(&mut self) {
        self.show_progress = true;
        self.progress = 0.0;
        self.progress_message = "Starting project creation...".to_string();
        self.project_created = false;
        self.project_error = None;
        log::debug!("Starting project creation");
    }

    fn create_new_project(&mut self) {
        let mut errors = Vec::new();
        let folders = [
            ("git", 0.1, "Creating a git folder..."),
            ("src", 0.2, "Creating src folder..."),
            ("resources/models", 0.4, "Creating models folder..."),
            ("resources/shaders", 0.6, "Creating shaders folder..."),
            ("resources/textures", 0.8, "Creating textures folder..."),
            ("src2", 0.9, "Creating project config file..."),
        ];

        if let Some(path) = &self.project_path {
            for (folder, progress, message) in folders {
                self.progress_message = message.to_string();
                self.progress = progress;
                let full_path = path.join(folder);
                let result: Result<(), String> = if folder == "src" {
                    fs::create_dir(&full_path).map_err(|e| e.to_string())
                } else if folder == "git" {
                    Repository::init(path)
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                } else if folder == "src2" {
                    if let Some(path) = &self.project_path {
                        let config = ProjectConfig::from(self.project_name.clone(), path);
                        match ron::ser::to_string(&config) {
                            Ok(ron_str) => {
                                let config_path =
                                    path.join(format!("{}.euc", self.project_name.clone()));
                                fs::write(&config_path, ron_str)
                                    .map(|_| ())
                                    .map_err(|e| e.to_string())
                            }
                            Err(e) => Err(format!("RON serialization error: {}", e)),
                        }
                    } else {
                        Err("Project path not found".to_string())
                    }
                } else {
                    fs::create_dir_all(&full_path).map_err(|e| e.to_string())
                };
                if let Err(e) = result {
                    errors.push(e);
                }
            }
            self.progress = 1.0;
            self.progress_message = "Project creation complete!".to_string();
            if errors.is_empty() {
                self.project_created = true;
            } else {
                self.project_created = false;
                self.project_error = Some(errors);
            }
        }
        self.show_progress = true;
        if self.project_created {
            self.scene_command = SceneCommand::SwitchScene("editor".to_string());
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

        let mut show_new_project = self.show_new_project;
        egui::Window::new("Create new project")
            .open(&mut show_new_project)
            .resizable(true)
            .collapsible(false)
            .fixed_size(screen_size)
            .show(egui_ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Project Name:");
                    ui.add_space(5.0);

                    ui.text_edit_singleline(&mut self.project_name);
                    ui.add_space(10.0);

                    ui.heading("Project Location: ");
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

                    let can_create = self.project_path.is_some() && !self.project_name.is_empty();
                    if ui
                        .add_enabled(can_create, egui::Button::new("Create Project"))
                        .clicked()
                    {
                        log::info!("Creating new project at {:?}", self.project_path);
                        self.start_project_creation();
                        self.create_new_project();
                        ui.ctx().request_repaint();
                    }
                });
            });
        self.show_new_project = show_new_project;

        if self.show_progress {
            egui::Window::new("Creating Project...")
                .collapsible(false)
                .resizable(false)
                .fixed_size([400.0, 120.0])
                .show(egui_ctx, |ui| {
                    ui.label(&self.progress_message);
                    ui.add_space(10.0);

                    ui.add(egui::ProgressBar::new(self.progress).show_percentage());
                    if let Some(errors) = &self.project_error {
                        ui.colored_label(egui::Color32::RED, "Errors:");
                        for err in errors {
                            ui.label(err);
                        }
                    }
                });
        }
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
