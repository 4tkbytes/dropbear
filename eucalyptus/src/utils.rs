use std::{path::PathBuf, process::Command};

use dropbear_engine::log;

pub fn convert_model_to_image(project_path: &PathBuf, path: &PathBuf) {
    let path = path.clone();
    let project_path = project_path.clone();
    std::thread::spawn(move || {
        let script_path = "src/scripts/convert_model_to_image.py";
        let poetry_cmd = Command::new("poetry")
            .current_dir(project_path)
            .arg("run")
            .arg("python")
            .arg(script_path)
            .arg(path.to_str().unwrap())
            .output();

        match poetry_cmd {
            Ok(output) => {
                if !output.status.success() {
                    log::error!(
                        "Thumbnail generation failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                } else {
                    log::info!(
                        "Thumbnail generation succeeded: {}",
                        String::from_utf8_lossy(&output.stdout)
                    );
                }
            }
            Err(e) => {
                log::error!("Failed to run thumbnail generator: {}", e);
            }
        }
    });
}