#[cfg(feature = "editor")]
use std::{cell::RefCell, rc::Rc, fs, path::PathBuf};
#[cfg(feature = "editor")]
use clap::{Arg, Command};

#[cfg(feature = "editor")]
use dropbear_engine::{WindowConfiguration, scene};

#[tokio::main]
#[cfg(feature = "editor")]
async fn main() -> anyhow::Result<()> {
    let matches = Command::new("eucalyptus")
        .about("A visual game editor")
        .version("1.0.0")
        .subcommand_required(false)
        .arg_required_else_help(false)
        .subcommand(
            Command::new("build")
                .about("Build a eucalyptus project, but only the .eupak file and its resources")
                .arg(
                    Arg::new("project")
                        .help("Path to the .eucp project file")
                        .value_name("PROJECT_FILE")
                        .required(false),
                ),
        )
        .subcommand(
            Command::new("package")
                .about("Package a eucalyptus project, which compiles the runtime and the resource .eupak file")
                .arg(
                    Arg::new("project")
                        .help("Path to the .eucp project file")
                        .value_name("PROJECT_FILE")
                        .required(false),
                ),
        )
        .subcommand(Command::new("read")
            .about("Reads and displays the contents of a .eupak file for debugging")
            .arg(
                    Arg::new("eupak_file")
                        .help("Path to the .eupak file")
                        .value_name("RESOURCE_FILE")
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

            eucalyptus::build::build(project_path, sub_matches)?;
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

            eucalyptus::build::package(project_path, sub_matches)?;
        }
        Some(("health", _)) => {
            eucalyptus::build::health()?;
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

            eucalyptus::build::read_from_eupak(project_path)?;
        }
        None => {
            let config = WindowConfiguration {
                title: "Eucalyptus, built with dropbear".into(),
                windowed_mode: dropbear_engine::WindowedModes::Maximised,
                max_fps: dropbear_engine::App::NO_FPS_CAP,
            };

            let _app = dropbear_engine::run_app!(config, |mut scene_manager, mut input_manager| {
                let main_menu = Rc::new(RefCell::new(eucalyptus::menu::MainMenu::new()));
                let editor = Rc::new(RefCell::new(eucalyptus::editor::Editor::new()));

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
    Ok(())
}

#[cfg(not(feature = "editor"))]
fn main() {
    panic!("You have not enabled the \"editor\" feature, therefore cannot use the eucalyptus editor. 
Ether import as a lib to use its structs and enums or enable the editor feature");
}

#[cfg(feature = "editor")]
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
