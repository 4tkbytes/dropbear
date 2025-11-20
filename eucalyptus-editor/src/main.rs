// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// note to self: when it becomes release, remember to readd this back

use clap::{Arg, Command};
use dropbear_engine::future::FutureQueue;
use dropbear_engine::{MutableWindowConfiguration, WindowConfiguration, scene};
use eucalyptus_core::APP_INFO;
use eucalyptus_editor::{build, editor, menu};
use parking_lot::RwLock;
use std::sync::Arc;
use std::{fs, path::PathBuf, rc::Rc};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    #[cfg(not(target_os = "android"))]
    {
        use colored::Colorize;
        use env_logger::Builder;
        use log::LevelFilter;
        use std::fs::OpenOptions;

        let log_dir =
            app_dirs2::app_root(app_dirs2::AppDataType::UserData, &eucalyptus_core::APP_INFO)
                .expect("Failed to get app data directory")
                .join("logs");
        fs::create_dir_all(&log_dir).expect("Failed to create log dir");

        let datetime_str = chrono::offset::Local::now().format("%Y-%m-%d_%H-%M-%S");
        let log_filename = format!("{}.{}.log", "eucalyptus-editor", datetime_str);
        let log_path = log_dir.join(log_filename);

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)
            .expect("Failed to open log file");
        let file = parking_lot::Mutex::new(file);

        let app_target = "eucalyptus-editor".replace('-', "_");
        let log_config = format!("dropbear_engine=trace,{}=debug,warn", app_target);
        unsafe { std::env::set_var("RUST_LOG", log_config) };

        Builder::new()
            .format(move |buf, record| {
                use std::io::Write;

                let ts = chrono::offset::Local::now().format("%Y-%m-%dT%H:%M:%S");

                let colored_level = match record.level() {
                    log::Level::Error => record.level().to_string().red().bold(),
                    log::Level::Warn => record.level().to_string().yellow().bold(),
                    log::Level::Info => record.level().to_string().green().bold(),
                    log::Level::Debug => record.level().to_string().blue().bold(),
                    log::Level::Trace => record.level().to_string().cyan().bold(),
                };

                let colored_timestamp = ts.to_string().bright_black();

                let file_info = format!(
                    "{}:{}",
                    record.file().unwrap_or("unknown"),
                    record.line().unwrap_or(0)
                )
                .bright_black();

                let console_line = format!(
                    "{} {} [{}] - {}\n",
                    file_info,
                    colored_timestamp,
                    colored_level,
                    record.args()
                );

                let file_line = format!(
                    "{}:{} {} [{}] - {}\n",
                    record.file().unwrap_or("unknown"),
                    record.line().unwrap_or(0),
                    ts,
                    record.level(),
                    record.args()
                );

                write!(buf, "{}", console_line)?;

                let mut fh = file.lock();
                let _ = fh.write_all(file_line.as_bytes());

                Ok(())
            })
            .filter(Some("dropbear_engine"), LevelFilter::Trace)
            .filter(
                Some("eucalyptus-editor".replace('-', "_").as_str()),
                LevelFilter::Debug,
            )
            .filter(Some("eucalyptus_core"), LevelFilter::Debug)
            .init();
        log::info!("Initialised logger");
    }

    dropbear_engine::panic::set_hook();
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

            build::build(project_path)?;
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

            build::package(project_path, sub_matches)?;
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
                title: format!(
                    "Eucalyptus, built with dropbear | Version {} on commit {}",
                    env!("CARGO_PKG_VERSION"),
                    env!("GIT_HASH")
                ),
                window_config: MutableWindowConfiguration {
                    windowed_mode: dropbear_engine::WindowedModes::Maximised,
                    max_fps: dropbear_engine::App::NO_FPS_CAP,
                },
                app_info: APP_INFO,
            };

            let future_queue = Arc::new(FutureQueue::new());

            let main_menu = Rc::new(RwLock::new(menu::MainMenu::new()));
            let editor =
                Rc::new(RwLock::new(editor::Editor::new().unwrap_or_else(|e| {
                    panic!("Unable to initialise Eucalyptus Editor: {}", e)
                })));

            dropbear_engine::run_app!(
                config,
                Some(future_queue),
                |mut scene_manager, mut input_manager| {
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
                }
            )
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
            && file_name.ends_with(".eucp")
        {
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
