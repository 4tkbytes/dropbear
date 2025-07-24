use std::{
    fs, io,
    path::Path,
    process::Command,
    sync::mpsc::{self, Receiver},
};

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
    project_error: Option<Vec<String>>,

    progress_rx: Option<Receiver<ProjectProgress>>,

    show_progress: bool,
    progress: f32,
    progress_message: String,
    toast: Toasts,
}

pub enum ProjectProgress {
    Step {
        progress: f32,
        message: String,
    },
    #[allow(dead_code)]
    Error(String),
    Done,
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

    fn setup_poetry_project(project_path: &Path) -> anyhow::Result<()> {
        if !Command::new("poetry").args(["--version"]).output().is_ok() {
            return Err(anyhow!(
                "Poetry is not installed. Please install it using pipx or the official poetry website"
            ));
        }

        let status = Command::new("poetry")
            .args(&["new", "scripts"])
            .current_dir(project_path)
            .status()
            .expect("Failed to run poetry new");
        if !status.success() {
            return Err(anyhow!(io::Error::new(
                io::ErrorKind::Other,
                "Poetry project creation failed",
            )));
        }

        let scripts_path = project_path.join("scripts");
        if scripts_path.exists() && scripts_path.is_dir() {
            for entry in fs::read_dir(&scripts_path)? {
                let entry = entry?;
                let path = entry.path();
                let file_name = path.file_name().unwrap();
                let dest = project_path.join(file_name);
                fs::rename(&path, &dest)?;
            }
        }

        if scripts_path.exists() {
            fs::remove_dir_all(&scripts_path)?;
        }

        let tests_path = project_path.join("tests");
        if tests_path.exists() {
            fs::remove_dir_all(&tests_path)?;
        }

        log::debug!("Running poetry install in {:?}", project_path);
        let status = Command::new("poetry")
            .arg("install")
            .current_dir(project_path)
            .status()
            .expect("Failed to run poetry install");
        if !status.success() {
            return Err(anyhow!(io::Error::new(
                io::ErrorKind::Other,
                "Poetry install failed",
            )));
        }

        Ok(())
    }

    fn start_project_creation(&mut self) {
        let (tx, rx) = mpsc::channel();
        let project_name = self.project_name.clone();
        let project_path = self.project_path.clone();

        self.progress_rx = Some(rx);
        self.show_progress = true;
        self.progress = 0.0;
        self.progress_message = "Starting project creation...".to_string();

        std::thread::spawn(move || {
            let mut errors = Vec::new();
            let folders = [
                ("git", 0.1, "Creating a git folder..."),
                ("src", 0.2, "Creating src folder..."),
                ("resources/models", 0.4, "Creating models folder..."),
                ("scripts", 0.5, "Initialising Python environment"),
                (
                    "deps",
                    0.55,
                    "Installing python dependencies into environment",
                ),
                ("resources/shaders", 0.6, "Creating shader folder..."),
                ("resources/textures", 0.8, "Creating textures folder..."),
                ("src2", 0.9, "Creating project config file..."),
            ];

            if let Some(path) = &project_path {
                for (folder, progress, message) in folders {
                    tx.send(ProjectProgress::Step {
                        progress,
                        message: message.to_string(),
                    })
                    .ok();

                    let full_path = path.join(folder);
                    let result: anyhow::Result<()> = if folder == "src" {
                        if !full_path.exists() {
                            fs::create_dir(&full_path)
                                .map_err(|e| anyhow::anyhow!(e))
                                .map(|_| ())
                        } else {
                            Ok(())
                        }
                    } else if folder == "git" {
                        match Repository::init(path) {
                            Ok(_) => Ok(()),
                            Err(e) => {
                                if matches!(e.code(), git2::ErrorCode::Exists) {
                                    log::warn!("Git repository already exists");
                                    Ok(())
                                } else {
                                    Err(anyhow!(e))
                                }
                            }
                        }
                    } else if folder == "src2" {
                        if let Some(path) = &project_path {
                            let mut config = ProjectConfig::new(project_name.clone(), &path);
                            let _ = config.write_to(&path);
                            let mut global = PROJECT.write().unwrap();
                            *global = config;
                            Ok(())
                        } else {
                            Err(anyhow!("Project path not found"))
                        }
                    } else if folder == "scripts" || folder == "deps" {
                        if folder == "scripts" {
                            Self::setup_poetry_project(path)
                        } else {
                            let status = Command::new("poetry")
                                .args(["add", "3d-to-image"])
                                .current_dir(path)
                                .status();
                            match status {
                                Ok(_) => Ok(()),
                                Err(e) => Err(anyhow!(e)),
                            }
                        }
                    } else {
                        if !full_path.exists() {
                            fs::create_dir_all(&full_path)
                                .map_err(|e| anyhow!(e))
                                .map(|_| ())
                        } else {
                            log::warn!("{:?} already exists", full_path);
                            Ok(())
                        }
                    };
                    if let Err(e) = result {
                        errors.push(e);
                    }
                }
                tx.send(ProjectProgress::Step {
                    progress: 1.0,
                    message: "Project creation complete!".to_string(),
                })
                .ok();

                tx.send(ProjectProgress::Done).ok();
            }
        });

        log::debug!("Starting project creation");
    }
}

#[async_trait]
impl Scene for MainMenu {
    fn load(&mut self, _graphics: &mut dropbear_engine::graphics::Graphics) {}

    fn update(&mut self, _dt: f32, _graphics: &mut dropbear_engine::graphics::Graphics) {}

    fn render(&mut self, graphics: &mut dropbear_engine::graphics::Graphics) {
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
                        ui.ctx().request_repaint();
                    }
                });
            });
        self.show_new_project = show_new_project;

        if let Some(rx) = self.progress_rx.as_mut() {
            while let Ok(progress) = rx.try_recv() {
                match progress {
                    ProjectProgress::Step { progress, message } => {
                        self.progress = progress;
                        self.progress_message = message;
                    }
                    ProjectProgress::Error(err) => {
                        self.project_error.get_or_insert_with(Vec::new).push(err);
                    }
                    ProjectProgress::Done if self.project_error.is_none() => {
                        self.scene_command = SceneCommand::SwitchScene("editor".to_string());
                    }
                    ProjectProgress::Done => {}
                }
            }
        }

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

    fn exit(&mut self, _event_loop: &dropbear_engine::winit::event_loop::ActiveEventLoop) {}

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
