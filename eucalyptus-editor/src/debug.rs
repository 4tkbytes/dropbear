//! Used to aid with debugging any issues with the editor.
use crate::editor::Signal;
use egui::{Ui, Window, ProgressBar};
use eucalyptus_core::scripting::build::GleamScriptCompiler;
use tokio::sync::mpsc;

#[derive(Debug, Clone)]
pub enum DependencyProgress {
    Starting,
    CheckingPath(String),
    Downloading(String),
    Extracting(String),
    Completed(String),
    Error(String),
    Finished,
}

pub struct DependencyInstaller {
    pub progress_receiver: Option<mpsc::UnboundedReceiver<DependencyProgress>>,
    pub current_progress: Vec<String>,
    pub is_installing: bool,
    pub progress_value: f32,
}

impl Default for DependencyInstaller {
    fn default() -> Self {
        Self {
            progress_receiver: None,
            current_progress: Vec::new(),
            is_installing: false,
            progress_value: 0.0,
        }
    }
}

/// Show a menu bar for debug. A new "Debug" menu button will show up on the editors menu bar.
pub(crate) fn show_menu_bar(ui: &mut Ui, signal: &mut Signal, dependency_installer: &mut DependencyInstaller) {
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
                start_dependency_installation(dependency_installer);
            }
        });

        if ui_debug.button("Evaluate script").clicked() {
            log::info!("Evaluating script");
        }
    });
}

pub(crate) fn show_dependency_progress_window(
    ctx: &egui::Context,
    dependency_installer: &mut DependencyInstaller,
) {
    if let Some(receiver) = &mut dependency_installer.progress_receiver {
        while let Ok(progress) = receiver.try_recv() {
            match &progress {
                DependencyProgress::Starting => {
                    dependency_installer.current_progress.clear();
                    dependency_installer.current_progress.push("Starting dependency check...".to_string());
                    dependency_installer.progress_value = 0.0;
                }
                DependencyProgress::CheckingPath(tool) => {
                    dependency_installer.current_progress.push(format!("Checking if {} is in PATH...", tool));
                    dependency_installer.progress_value = 0.1;
                }
                DependencyProgress::Downloading(tool) => {
                    dependency_installer.current_progress.push(format!("Downloading {}...", tool));
                    dependency_installer.progress_value += 0.25;
                }
                DependencyProgress::Extracting(tool) => {
                    dependency_installer.current_progress.push(format!("Extracting {}...", tool));
                    dependency_installer.progress_value += 0.1;
                }
                DependencyProgress::Completed(tool) => {
                    dependency_installer.current_progress.push(format!("✓ {} ready", tool));
                    dependency_installer.progress_value += 0.1;
                }
                DependencyProgress::Error(error) => {
                    dependency_installer.current_progress.push(format!("❌ Error: {}", error));
                }
                DependencyProgress::Finished => {
                    dependency_installer.current_progress.push("✓ All dependencies ready!".to_string());
                    dependency_installer.progress_value = 1.0;
                    dependency_installer.is_installing = false;
                    // dependency_installer.progress_receiver = None;
                }
            }
        }
    }

    if dependency_installer.is_installing || !dependency_installer.current_progress.is_empty() {
        Window::new("Dependency Installation")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.vertical(|ui| {
                    ui.add(
                        ProgressBar::new(dependency_installer.progress_value)
                            .show_percentage()
                            .animate(dependency_installer.is_installing)
                    );
                    
                    ui.separator();
                    
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            for message in &dependency_installer.current_progress {
                                ui.label(message);
                            }
                        });
                    
                    if !dependency_installer.is_installing && ui.button("Close").clicked() {
                        dependency_installer.current_progress.clear();
                    }
                });
            });
    }
}

fn start_dependency_installation(dependency_installer: &mut DependencyInstaller) {
    let (sender, receiver) = mpsc::unbounded_channel();
    dependency_installer.progress_receiver = Some(receiver);
    dependency_installer.is_installing = true;
    dependency_installer.progress_value = 0.0;
    
    tokio::spawn(async move {
        let _ = sender.send(DependencyProgress::Starting);
        
        match ensure_dependencies_with_progress(sender.clone()).await {
            Ok(_) => {
                let _ = sender.send(DependencyProgress::Finished);
            }
            Err(e) => {
                let _ = sender.send(DependencyProgress::Error(e.to_string()));
                let _ = sender.send(DependencyProgress::Finished);
            }
        }
    });
}

async fn ensure_dependencies_with_progress(
    progress_sender: mpsc::UnboundedSender<DependencyProgress>,
) -> anyhow::Result<()> {
    let tools = vec![
        ("gleam", "Gleam"),
        ("bun", "Bun"), 
        ("javy", "Javy"),
    ];
    
    let mut tools_to_download = Vec::new();
    
    for (tool_cmd, tool_name) in &tools {
        let _ = progress_sender.send(DependencyProgress::CheckingPath(tool_name.to_string()));
        
        let available = check_tool_in_path(tool_cmd).await;
        if !available {
            tools_to_download.push((*tool_cmd, *tool_name));
        } else {
            let _ = progress_sender.send(DependencyProgress::Completed(format!("{} (found in PATH)", tool_name)));
        }
    }
    
    if tools_to_download.is_empty() {
        return Ok(());
    }
    
    let app_dir = app_dirs2::app_dir(app_dirs2::AppDataType::UserData, &eucalyptus_core::scripting::build::APP_INFO, "")
        .map_err(|e| anyhow::anyhow!("Failed to get app directory: {}", e))?;
    
    for (tool_cmd, tool_name) in tools_to_download {
        let _ = progress_sender.send(DependencyProgress::Downloading(tool_name.to_string()));
        
        match tool_cmd {
            "gleam" => {
                if let Err(e) = download_gleam_with_progress(&app_dir, progress_sender.clone()).await {
                    let _ = progress_sender.send(DependencyProgress::Error(format!("Failed to download Gleam: {}", e)));
                    return Err(e);
                }
            }
            "bun" => {
                if let Err(e) = download_bun_with_progress(&app_dir, progress_sender.clone()).await {
                    let _ = progress_sender.send(DependencyProgress::Error(format!("Failed to download Bun: {}", e)));
                    return Err(e);
                }
            }
            "javy" => {
                if let Err(e) = download_javy_with_progress(&app_dir, progress_sender.clone()).await {
                    let _ = progress_sender.send(DependencyProgress::Error(format!("Failed to download Javy: {}", e)));
                    return Err(e);
                }
            }
            _ => {}
        }
        
        let _ = progress_sender.send(DependencyProgress::Completed(tool_name.to_string()));
    }
    
    Ok(())
}

async fn check_tool_in_path(tool: &str) -> bool {
    let cmd = if cfg!(target_os = "windows") {
        std::process::Command::new("where").arg(tool).output()
    } else {
        std::process::Command::new("which").arg(tool).output()
    };

    match cmd {
        Ok(output) => output.status.success(),
        Err(_) => false,
    }
}

async fn download_gleam_with_progress(
    app_dir: &std::path::PathBuf,
    progress_sender: mpsc::UnboundedSender<DependencyProgress>,
) -> anyhow::Result<()> {
    let _ = progress_sender.send(DependencyProgress::Extracting("Gleam".to_string()));
    GleamScriptCompiler::download_gleam(app_dir).await
}

async fn download_bun_with_progress(
    app_dir: &std::path::PathBuf,
    progress_sender: mpsc::UnboundedSender<DependencyProgress>,
) -> anyhow::Result<()> {
    let _ = progress_sender.send(DependencyProgress::Extracting("Bun".to_string()));
    GleamScriptCompiler::download_bun(app_dir).await
}

async fn download_javy_with_progress(
    app_dir: &std::path::PathBuf,
    progress_sender: mpsc::UnboundedSender<DependencyProgress>,
) -> anyhow::Result<()> {
    let _ = progress_sender.send(DependencyProgress::Extracting("Javy".to_string()));
    GleamScriptCompiler::download_javy(app_dir).await
}