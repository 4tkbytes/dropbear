use std::fs;

use anyhow::anyhow;
use dropbear_engine::{
    async_trait::async_trait,
    egui::{self, FontId, Frame, RichText},
    gilrs,
    input::{Controller, Keyboard, Mouse},
    log::{self, debug},
    scene::{Scene, SceneCommand},
};
use egui_toast::{ToastOptions, Toasts};
use git2::Repository;

use crate::states::{PROJECT, ProjectConfig};

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
    toast: Toasts,
}

impl MainMenu {
    pub fn new() -> Self {
        Self {
            show_progress: false,
            toast: egui_toast::Toasts::new()
                .anchor(egui::Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(egui::Direction::BottomUp),
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
                let result: anyhow::Result<()> = if folder == "src" {
                    fs::create_dir(&full_path)
                        .map_err(|e| anyhow!(e))
                        .map(|_| ())
                } else if folder == "git" {
                    Repository::init(path).map_err(|e| anyhow!(e)).map(|_| ())
                } else if folder == "src2" {
                    if let Some(path) = &self.project_path {
                        let mut config = ProjectConfig::new(self.project_name.clone(), &path);
                        let _ = config.write_to(&path);
                        let mut global = PROJECT.write().unwrap();
                        *global = config;
                        Ok(())
                    } else {
                        Err(anyhow!("Project path not found"))
                    }
                } else {
                    fs::create_dir_all(&full_path)
                        .map_err(|e| anyhow!(e))
                        .map(|_| ())
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
                self.project_error = Some(errors.iter().map(|e| e.to_string()).collect::<Vec<_>>());
            }
        }
        self.show_progress = true;
        if self.project_created {
            self.scene_command = SceneCommand::SwitchScene("editor".to_string());
        } else {
            for error in self.project_error.as_ref().unwrap() {
                log::error!("Error: {}", error);
            }
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
                        log::debug!("Opening project");
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Eucalyptus Configuration Files", &["eucp"])
                            .pick_file()
                        {
                            match ProjectConfig::read_from(&path) {
                                Ok(config) => {
                                    log::info!("Loaded project!");
                                    let mut global = PROJECT.write().unwrap();
                                    *global = config;
                                    println!("Loaded config info: {:#?}", global);
                                    self.scene_command =
                                        SceneCommand::SwitchScene(String::from("editor"));
                                }
                                Err(e) => if e.to_string().contains("missing field") {
                                    self.toast.add(egui_toast::Toast {
                                        kind: egui_toast::ToastKind::Error,
                                        text: format!("Your project version is not up to date with the current project version. To fix this, // TODO: create a way to backup").into(),
                                        options: ToastOptions::default()
                                            .duration_in_seconds(5.0)
                                            .show_progress(true),
                                        ..Default::default()
                                    });
                                }
                            };
                        } else {
                            log::error!("File dialog returned \"None\"");
                        }
                    }
                    ui.add_space(20.0);

                    if ui
                        .add_sized(button_size, egui::Button::new("Settings"))
                        .clicked()
                    {
                        log::debug!("Settings (not implemented)");
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
                .collapsible(true)
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

        self.toast.show(graphics.get_egui_context());
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
