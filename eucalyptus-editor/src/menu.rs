use std::{
    fs,
    path::PathBuf,
};
use std::sync::Arc;
use anyhow::anyhow;
use dropbear_engine::{
    graphics::RenderContext,
    input::{Controller, Keyboard, Mouse},
    scene::{Scene, SceneCommand},
    future::{FutureHandle, FutureQueue},
};
use egui::{self, FontId, Frame, RichText};
use egui_toast_fork::{ToastOptions, Toasts};
use git2::Repository;
use log::{self, debug};
use rfd::FileDialog; // ← Sync version — no async needed
use winit::{
    dpi::PhysicalPosition, event::MouseButton, event_loop::ActiveEventLoop, keyboard::KeyCode,
};
use eucalyptus_core::states::{PROJECT, ProjectConfig};
use tokio::sync::watch;

#[derive(Debug, Clone)]
pub enum ProjectProgress {
    Step {
        progress: f32,
        message: String,
    },
    Error(String),
    Done,
}

#[derive(Default)]
pub struct MainMenu {
    scene_command: SceneCommand,
    show_new_project: bool,
    project_name: String,
    project_path: Option<PathBuf>,
    project_error: Option<Vec<String>>,

    project_progress_rx: Option<watch::Receiver<ProjectProgress>>,
    show_progress: bool,
    progress: f32,
    progress_message: String,

    // ❌ REMOVED: file_dialog: Option<FutureHandle>,
    project_creation_handle: Option<FutureHandle>, // ← Keep — project creation is async

    toast: Toasts,
    is_in_file_dialogue: bool,
}

impl MainMenu {
    pub fn new() -> Self {
        Self {
            show_progress: false,
            toast: Toasts::new()
                .anchor(egui::Align2::RIGHT_BOTTOM, (-10.0, -10.0))
                .direction(egui::Direction::BottomUp),
            ..Default::default()
        }
    }

    fn start_project_creation(&mut self, queue: Arc<FutureQueue>) {
        let project_name = self.project_name.clone();
        let project_path = self.project_path.clone();

        let (progress_tx, progress_rx) = watch::channel(ProjectProgress::Step {
            progress: 0.0,
            message: "Starting project creation...".to_string(),
        });

        self.project_progress_rx = Some(progress_rx);
        self.show_progress = true;
        self.progress = 0.0;

        let handle = queue.push(async move {
            let mut errors = Vec::new();
            let folders = [
                ("git", 0.1, "Initializing git repository..."),
                ("src", 0.2, "Creating src folder..."),
                ("resources/models", 0.3, "Creating models folder..."),
                ("resources/shaders", 0.4, "Creating shaders folder..."),
                ("resources/textures", 0.5, "Creating textures folder..."),
                ("src2", 0.6, "Generating project config..."),
                ("scenes", 0.7, "Creating scenes folder..."),
            ];

            if let Some(path) = &project_path {
                for (folder, progress, message) in folders {
                    let _ = progress_tx.send(ProjectProgress::Step {
                        progress,
                        message: message.to_string(),
                    });

                    let full_path = path.join(folder);
                    let result: anyhow::Result<()> = match folder {
                        "git" => {
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
                        }
                        "src2" => {
                            let mut config = ProjectConfig::new(project_name.clone(), path);
                            let _ = config.write_to_all();
                            let mut global = PROJECT.write();
                            *global = config;
                            Ok(())
                        }
                        _ => {
                            if !full_path.exists() {
                                fs::create_dir_all(&full_path)
                                    .map_err(|e| anyhow::anyhow!(e))
                                    .map(|_| ())
                            } else {
                                log::warn!("{:?} already exists", full_path);
                                Ok(())
                            }
                        }
                    };

                    if let Err(e) = result {
                        let _ = progress_tx.send(ProjectProgress::Error(e.to_string()));
                        errors.push(e);
                    }
                }

                let _ = progress_tx.send(ProjectProgress::Step {
                    progress: 1.0,
                    message: "Finalizing project...".to_string(),
                });

                if errors.is_empty() {
                    let _ = progress_tx.send(ProjectProgress::Done);
                    Ok(()) // Success
                } else {
                    Err(anyhow!("Project creation failed with {} errors", errors.len()))
                }
            } else {
                let _ = progress_tx.send(ProjectProgress::Error("Project path not set".to_string()));
                Err(anyhow!("Project path not set"))
            }
        });

        self.project_creation_handle = Some(handle);
        queue.poll();
        debug!("Starting project creation");
    }
}

impl Scene for MainMenu {
    fn load(&mut self, _graphics: &mut RenderContext) {
        log::info!("Loaded main menu scene");
    }

    fn update(&mut self, _dt: f32, _graphics: &mut RenderContext) {}

    fn render(&mut self, graphics: &mut RenderContext) {
        #[allow(clippy::collapsible_if)]
        if let Some(handle) = self.project_creation_handle.as_ref() {
            if let Some(result) = graphics.shared.future_queue.exchange_owned_as::<anyhow::Result<()>>(handle) {
                self.project_creation_handle = None;

                if result.is_ok() {
                    log::info!("Project created successfully!");
                    self.show_new_project = false;
                    self.show_progress = false;
                    self.scene_command = SceneCommand::SwitchScene("editor".to_string());
                } else {
                    log::error!("Project creation failed");
                }
            }
        }
        
        #[allow(clippy::collapsible_if)]
        if let Some(rx) = self.project_progress_rx.as_ref() {
            if let Ok(true) = rx.has_changed() {
                let progress = rx.borrow().clone();
                match progress {
                    ProjectProgress::Step { progress, message } => {
                        self.progress = progress;
                        self.progress_message = message;
                    }
                    ProjectProgress::Error(err) => {
                        self.project_error.get_or_insert_with(Vec::new).push(err);
                    }
                    ProjectProgress::Done => {}
                }
            }
        }

        let screen_size: (f32, f32) = (
            graphics.shared.window.inner_size().width as f32 - 100.0,
            graphics.shared.window.inner_size().height as f32 - 100.0,
        );
        let egui_ctx = graphics.shared.get_egui_context();
        let mut local_open_project = false;
        let mut local_select_project = false;

        egui::CentralPanel::default()
            .frame(Frame::new())
            .show(&egui_ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(64.0);
                    ui.label(RichText::new("Eucalyptus").font(FontId::proportional(32.0)));
                    ui.add_space(40.0);

                    let button_size = egui::vec2(300.0, 60.0);
                    let is_busy = self.is_in_file_dialogue || self.project_creation_handle.is_some();

                    if ui
                        .add_enabled(!is_busy, egui::Button::new("New Project").min_size(button_size))
                        .clicked()
                    {
                        log::debug!("Creating new project");
                        self.show_new_project = true;
                    }
                    ui.add_space(20.0);

                    if ui
                        .add_enabled(!is_busy, egui::Button::new("Open Project").min_size(button_size))
                        .clicked()
                    {
                        local_open_project = true;
                    }
                    ui.add_space(20.0);

                    if ui
                        .add_enabled(!is_busy, egui::Button::new("Settings").min_size(button_size))
                        .clicked()
                    {
                        log::debug!("Settings (not implemented)");
                    }
                    ui.add_space(20.0);

                    if ui
                        .add_enabled(!is_busy, egui::Button::new("Quit").min_size(button_size))
                        .clicked()
                    {
                        self.scene_command = SceneCommand::Quit;
                    }
                    ui.add_space(20.0);
                });
            });

        if local_open_project {
            debug!("Opening project dialog");
            self.is_in_file_dialogue = true;

            if let Some(path) = FileDialog::new()
                .add_filter("Eucalyptus Configuration Files", &["eucp"])
                .pick_file()
            {
                match ProjectConfig::read_from(&path) {
                    Ok(config) => {
                        log::info!("Loaded project: {:?}", path);
                        let mut global = PROJECT.write();
                        *global = config;
                        self.scene_command = SceneCommand::SwitchScene("editor".to_string());
                    }
                    Err(e) => {
                        let error_msg = if e.to_string().contains("missing field") {
                            "Project version is outdated. Please update your .eucp file."
                        } else {
                            &e.to_string()
                        };

                        self.toast.add(egui_toast_fork::Toast {
                            kind: egui_toast_fork::ToastKind::Error,
                            text: error_msg.to_string().into(),
                            options: ToastOptions::default()
                                .duration_in_seconds(8.0)
                                .show_progress(true),
                            ..Default::default()
                        });
                        log::error!("Failed to load project: {}", e);
                    }
                }
            } else {
                log::info!("User cancelled file dialog");
            }

            self.is_in_file_dialogue = false;
        }

        let mut show_new_project = self.show_new_project;
        egui::Window::new("Create New Project")
            .open(&mut show_new_project)
            .resizable(true)
            .collapsible(false)
            .fixed_size(screen_size)
            .show(&egui_ctx, |ui| {
                ui.vertical(|ui| {
                    ui.heading("Project Name:");
                    ui.add_space(5.0);
                    ui.text_edit_singleline(&mut self.project_name);
                    ui.add_space(10.0);

                    ui.heading("Project Location:");
                    ui.add_space(5.0);

                    if let Some(ref path) = self.project_path {
                        ui.label(format!("Chosen location: {}", path.display()));
                        ui.add_space(5.0);
                    }

                    if ui.button("Choose Location").clicked() {
                        local_select_project = true;
                    }
                    ui.add_space(10.0);

                    let can_create = self.project_path.is_some() && !self.project_name.is_empty();
                    if ui
                        .add_enabled(can_create && !self.project_creation_handle.is_some(),
                                     egui::Button::new("Create Project"))
                        .clicked()
                    {
                        log::info!("Creating new project at {:?}", self.project_path);
                        self.start_project_creation(graphics.shared.future_queue.clone());
                    }
                });
            });
        self.show_new_project = show_new_project;

        if local_select_project {
            log::debug!("Opening folder picker");
            self.is_in_file_dialogue = true;

            let name = self.project_name.clone();
            if let Some(path) = FileDialog::new()
                .set_title("Select Project Folder")
                .set_file_name(&name)
                .pick_folder()
            {
                self.project_path = Some(path.clone());
                log::debug!("Selected project location: {:?}", path);
            }

            self.is_in_file_dialogue = false;
        }

        if self.show_progress {
            egui::Window::new("Creating Project...")
                .collapsible(false)
                .resizable(false)
                .fixed_size([400.0, 150.0])
                .show(&egui_ctx, |ui| {
                    ui.label(&self.progress_message);
                    ui.add_space(10.0);
                    ui.add(egui::ProgressBar::new(self.progress).show_percentage());

                    if let Some(errors) = &self.project_error {
                        ui.add_space(10.0);
                        ui.colored_label(egui::Color32::RED, "Errors encountered:");
                        for err in errors {
                            ui.label(err);
                        }
                    }
                });
        }

        self.toast.show(&egui_ctx);
    }

    fn exit(&mut self, _event_loop: &ActiveEventLoop) {
        log::info!("Exiting main menu scene");
    }

    fn run_command(&mut self) -> SceneCommand {
        std::mem::replace(&mut self.scene_command, SceneCommand::None)
    }
}

impl Keyboard for MainMenu {
    fn key_down(&mut self, key: KeyCode, event_loop: &ActiveEventLoop) {
        if key == KeyCode::Escape
            && !self.show_new_project && !self.is_in_file_dialogue {
            event_loop.exit();
        }
    }

    fn key_up(&mut self, _key: KeyCode, _event_loop: &ActiveEventLoop) {}
}

impl Mouse for MainMenu {
    fn mouse_move(&mut self, _position: PhysicalPosition<f64>) {}
    fn mouse_down(&mut self, _button: MouseButton) {}
    fn mouse_up(&mut self, _button: MouseButton) {}
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