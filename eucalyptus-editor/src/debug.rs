//! Used to aid with debugging any issues with the editor.
use crate::build::gleam::GleamScriptCompiler;
use crate::build::gleam::InstallStatus;
use crate::editor::Signal;
use egui::ProgressBar;
use egui::Ui;
use egui::Window;
use tokio::sync::mpsc;

pub struct DependencyInstaller {
    pub progress_receiver: Option<mpsc::UnboundedReceiver<InstallStatus>>,
    pub is_installing: bool,

    pub gleam_progress: f32,
    pub bun_progress: f32,
    pub javy_progress: f32,

    pub gleam_status: String,
    pub bun_status: String,
    pub javy_status: String,
}

impl Default for DependencyInstaller {
    fn default() -> Self {
        Self {
            progress_receiver: None,
            is_installing: false,
            gleam_progress: 0.0,
            bun_progress: 0.0,
            javy_progress: 0.0,
            gleam_status: "Not started".to_string(),
            bun_status: "Not started".to_string(),
            javy_status: "Not started".to_string(),
        }
    }
}

impl DependencyInstaller {
    pub fn update_progress(&mut self) {
        let mut local_prog_rec = false;
        let mut update_tool_status: (bool, String, f32, String) = Default::default();
        if let Some(receiver) = &mut self.progress_receiver {
            while let Ok(status) = receiver.try_recv() {
                match status {
                    InstallStatus::NotStarted => {
                        self.is_installing = false;
                    }
                    InstallStatus::InProgress {
                        tool,
                        step,
                        progress,
                    } => {
                        self.is_installing = true;
                        update_tool_status =
                            (true, String::from(&tool), progress, String::from(&step));
                    }
                    InstallStatus::Success => {
                        self.is_installing = false;
                        local_prog_rec = true;
                        self.gleam_status = "Complete".to_string();
                        self.gleam_progress = 1.0;
                        self.bun_status = "Complete".to_string();
                        self.bun_progress = 1.0;
                        self.javy_status = "Complete".to_string();
                        self.javy_progress = 1.0;
                    }
                    InstallStatus::Failed(msg) => {
                        self.is_installing = false;
                        log::error!("Installation error: {}", msg);
                        self.gleam_status = format!("Error: {}", msg);
                        self.bun_status = format!("Error: {}", msg);
                        self.javy_status = format!("Error: {}", msg);
                    }
                }
            }
        }
        if local_prog_rec {
            self.progress_receiver = None;
        }
        if update_tool_status.0 {
            self.update_tool_status(
                update_tool_status.1.as_str(),
                update_tool_status.2,
                update_tool_status.3.as_str(),
            );
        }
    }

    fn update_tool_status(&mut self, tool: &str, progress: f32, status: &str) {
        match tool.to_lowercase().as_str() {
            "gleam" => {
                self.gleam_progress = progress;
                self.gleam_status = status.to_string();
            }
            "bun" => {
                self.bun_progress = progress;
                self.bun_status = status.to_string();
            }
            "javy" => {
                self.javy_progress = progress;
                self.javy_status = status.to_string();
            }
            _ => {}
        }
    }

    pub fn show_installation_window(&mut self, ctx: &egui::Context) {
        Window::new("Installing Dependencies")
            .resizable(true)
            .collapsible(false)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.heading("Installing Dependencies");

                ui.separator();

                ui.label("Gleam:");
                ui.label(&self.gleam_status);
                ui.add(ProgressBar::new(self.gleam_progress).show_percentage());
                ui.separator();

                ui.label("Bun:");
                ui.label(&self.bun_status);
                ui.add(ProgressBar::new(self.bun_progress).show_percentage());
                ui.separator();

                ui.label("Javy:");
                ui.label(&self.javy_status);
                ui.add(ProgressBar::new(self.javy_progress).show_percentage());
                ui.separator();

                let overall_progress =
                    (self.gleam_progress + self.bun_progress + self.javy_progress) / 3.0;
                ui.label("Overall Progress:");
                ui.add(ProgressBar::new(overall_progress).show_percentage());

                ui.separator();

                if ui.button("Cancel").clicked() {
                    self.is_installing = false;
                    self.progress_receiver = None;
                }
            });
    }
}

pub(crate) fn show_menu_bar(
    ui: &mut Ui,
    signal: &mut Signal,
    dependency_installer: &mut DependencyInstaller,
) {
    ui.menu_button("Debug", |ui_debug| {
        if ui_debug.button("Panic").clicked() {
            log::warn!("Panic caused on purpose from Menu Button Click");
            panic!("Testing out panicking with new panic module, this is a test")
        }

        if ui_debug.button("Show Entities Loaded").clicked() {
            log::info!("Show Entities Loaded under Debug Menu is clicked");
            *signal = Signal::LogEntities;
        }

        ui_debug.add_enabled_ui(!dependency_installer.is_installing, |ui| {
            if ui.button("Ensure dependencies").clicked() {
                log::info!("Clicked ensure dependencies from debug menu");

                let (sender, receiver) = mpsc::unbounded_channel();
                dependency_installer.progress_receiver = Some(receiver);
                dependency_installer.is_installing = true;

                dependency_installer.gleam_progress = 0.0;
                dependency_installer.bun_progress = 0.0;
                dependency_installer.javy_progress = 0.0;
                dependency_installer.gleam_status = "Starting...".to_string();
                dependency_installer.bun_status = "Starting...".to_string();
                dependency_installer.javy_status = "Starting...".to_string();

                tokio::task::spawn(async move {
                    match GleamScriptCompiler::ensure_dependencies(Some(sender.clone())).await {
                        Ok(_) => {
                            let _ = sender.send(InstallStatus::Success);
                        }
                        Err(e) => {
                            let _ = sender.send(InstallStatus::Failed(e.to_string()));
                        }
                    }
                });
            }
        });

        if ui_debug.button("Evaluate script").clicked() {
            log::info!("Evaluating script");
        }
    });
}
