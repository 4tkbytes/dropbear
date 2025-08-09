mod editor;
mod menu;

pub(crate) mod build;
pub(crate) mod camera;
pub(crate) mod logging;
pub(crate) mod scripting;
pub(crate) mod states;
pub(crate) mod utils;

use std::{cell::RefCell, fs, path::PathBuf, rc::Rc};

use clap::{Arg, Command};
use dropbear_engine::{WindowConfiguration, scene};

pub const APP_INFO: app_dirs2::AppInfo = app_dirs2::AppInfo {
    name: "Eucalyptus",
    author: "4tkbytes",
};

#[tokio::main]
async fn main() {
    let matches = Command::new("eucalyptus")
        .about("A visual game editor")
        .version("1.0.0")
        .subcommand_required(false)
        .arg_required_else_help(false)
        .subcommand(
            Command::new("build")
                .about("Build a eucalyptus project")
                .arg(
                    Arg::new("project")
                        .help("Path to the .eucp project file")
                        .value_name("PROJECT_FILE")
                        .required(false),
                ),
        )
        .subcommand(
            Command::new("package")
                .about("Package a eucalyptus project")
                .arg(
                    Arg::new("project")
                        .help("Path to the .eucp project file")
                        .value_name("PROJECT_FILE")
                        .required(false),
                ),
        )
        .subcommand(Command::new("health").about("Check the health of the eucalyptus installation"))
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

            crate::build::build(project_path, sub_matches);
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

            crate::build::package(project_path, sub_matches);
        }
        Some(("health", _)) => {
            crate::build::health();
        }
        None => {
            let config = WindowConfiguration {
                title: "Eucalyptus, built with dropbear",
                windowed_mode: dropbear_engine::WindowedModes::Maximised,
                max_fps: dropbear_engine::App::NO_FPS_CAP,
            };

            let _app = dropbear_engine::run_app!(config, |mut scene_manager, mut input_manager| {
                let main_menu = Rc::new(RefCell::new(menu::MainMenu::new()));
                let editor = Rc::new(RefCell::new(editor::Editor::new()));

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
            .unwrap();
        }
        _ => unreachable!(),
    }
}

fn find_eucp_file() -> Result<PathBuf, String> {
    let current_dir = std::env::current_dir().map_err(|_| "Failed to get current directory")?;

    let entries = fs::read_dir(&current_dir).map_err(|_| "Failed to read current directory")?;

    let mut eucp_files = Vec::new();

    for entry in entries {
        if let Ok(entry) = entry {
            if let Some(file_name) = entry.file_name().to_str() {
                if file_name.ends_with(".eucp") {
                    eucp_files.push(entry.path());
                }
            }
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
