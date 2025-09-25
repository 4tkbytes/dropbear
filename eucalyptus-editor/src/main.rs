#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod build;
mod camera;
mod debug;
mod editor;
mod menu;
mod utils;
mod spawn;
mod signal;

use clap::{Arg, Command};
use dropbear_engine::{scene, WindowConfiguration};
use std::{fs, path::PathBuf, rc::Rc};
use std::sync::Arc;
use parking_lot::RwLock;
use dropbear_engine::future::FutureQueue;

pub const APP_INFO: app_dirs2::AppInfo = app_dirs2::AppInfo {
    name: "Eucalyptus",
    author: "4tkbytes",
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "android")]
    compile_error!(
        "The `editor` feature is not supported on Android. If you are attempting\
 to use the Eucalyptus editor on Android, please don't. Instead, use the `data-only` feature\
 to use with dependencies or create your own game on Desktop. Sorry :("
    );
    let matches = Command::new("eucalyptus-editor")
        .about("A visual game editor")
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand_required(false)
        .arg_required_else_help(false)
        .subcommand(
            Command::new("build")
                .about("Build a eucalyptus project, but only the .eupak file and its resources")
                .arg(
                    Arg::new("project")
                        .help("Path to the .eucp project file")
                        .value_name("PROJECT_FILE")
                        .required(true),
                ),
        )
        .subcommand(
            Command::new("package")
                .about("Package a eucalyptus project, which compiles the runtime and the resource .eupak file")
                .arg(
                    Arg::new("project")
                        .help("Path to the .eucp project file")
                        .value_name("PROJECT_FILE")
                        .required(true),
                ),
        )
        .subcommand(Command::new("read")
            .about("Reads and displays the contents of a .eupak file for debugging")
            .arg(
                    Arg::new("eupak_file")
                        .help("Path to the .eupak file")
                        .value_name("RESOURCE_FILE")
                        .required(true),
                ),
        )
        .subcommand(Command::new("health").about("Check the health of the eucalyptus installation"))
        .subcommand(Command::new("compile")
            .about("Compiles a project's script into WebAssembly, primarily used for testing")
            .arg(
                Arg::new("project")
                    .help("Path to the .eucp project file")
                    .value_name("PROJECT_FILE")
                    .required(true),
            )
        )
        .get_matches();

    match matches.subcommand() {
        Some(("build", sub_matches)) => {
            let project_path = match sub_matches.get_one::<String>("project") {
                Some(path) => PathBuf::from(path),
                None => match find_eucp_file() {
                    Ok(path) => path,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                },
            };

            crate::build::build(project_path)?;
        }
        Some(("package", sub_matches)) => {
            let project_path = match sub_matches.get_one::<String>("project") {
                Some(path) => PathBuf::from(path),
                None => match find_eucp_file() {
                    Ok(path) => path,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                },
            };

            crate::build::package(project_path, sub_matches)?;
        }
        Some(("health", _)) => {
            build::health()?;
        }
        Some(("read", sub_matches)) => {
            let project_path = match sub_matches.get_one::<String>("eupak_file") {
                Some(path) => PathBuf::from(path),
                None => match find_eucp_file() {
                    Ok(path) => path,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                },
            };

            crate::build::read_from_eupak(project_path)?;
        }
        Some(("compile", sub_matches)) => {
            let _project_path = match sub_matches.get_one::<String>("project") {
                Some(path) => PathBuf::from(path),
                None => match find_eucp_file() {
                    Ok(path) => path,
                    Err(e) => {
                        eprintln!("Error: {}", e);
                        std::process::exit(1);
                    }
                },
            };
            
            println!("\"Compile\" command not implemented yet");
            // crate::build::compile(project_path).await?;
        }
        None => {
            let config = WindowConfiguration {
                title: "Eucalyptus, built with dropbear".into(),
                windowed_mode: dropbear_engine::WindowedModes::Maximised,
                max_fps: dropbear_engine::App::NO_FPS_CAP,
                app_info: APP_INFO,
            };
            
            let future_queue = Arc::new(FutureQueue::new());

            let main_menu = Rc::new(RwLock::new(menu::MainMenu::new()));
            let editor = Rc::new(RwLock::new(editor::Editor::new()));

            dropbear_engine::run_app!(config, Some(future_queue), |mut scene_manager, mut input_manager| {
                scene::add_scene_with_input(
                    &mut scene_manager,
                    &mut input_manager,
                    main_menu,
                    "main_menu",
                );
                scene::add_scene_with_input(
                    &mut scene_manager,
                    &mut input_manager,
                    editor,
                    "editor",
                );

                scene_manager.switch("main_menu");

                (scene_manager, input_manager)
            })
            .await?;
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn find_eucp_file() -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir().map_err(|_| "Failed to get current directory")?;

    let entries = fs::read_dir(&current_dir).map_err(|_| "Failed to read current directory")?;

    let mut eucp_files = Vec::new();

    for entry in entries {
        if let Ok(entry) = entry
            && let Some(file_name) = entry.file_name().to_str()
                && file_name.ends_with(".eucp") {
                    eucp_files.push(entry.path());
                }
    }

    match eucp_files.len() {
        0 => Err("No .eucp files found in current directory".to_string()),
        1 => Ok(eucp_files[0].clone()),
        _ => Err(format!(
            "Multiple .eucp files found: {:#?}. Please specify which one to use.",
            eucp_files
        )),
    }
}
