use std::{path::PathBuf, process::Command};

use dropbear_engine::log;

use crate::states::Node;

pub fn search_nodes_recursively<'a, F>(nodes: &'a [Node], matcher: &F, results: &mut Vec<&'a Node>)
where
    F: Fn(&Node) -> bool,
{
    for node in nodes {
        if matcher(node) {
            results.push(node);
        }
        match node {
            Node::File(_) => {}
            Node::Folder(folder) => {
                search_nodes_recursively(&folder.nodes, matcher, results);
            }
        }
    }
}

pub fn convert_model_to_image(project_path: &PathBuf, path: &PathBuf) {
    let path = path.clone();
    let project_path = project_path.clone();
    std::thread::spawn(move || {
        let script_path = "src/scripts/convert_model_to_image.py";

        match Command::new("poetry")
            .current_dir(&project_path)
            .arg("lock")
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    log::error!(
                        "Poetry lock failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                    return;
                } else {
                    log::info!(
                        "Poetry lock succeeded: {}",
                        String::from_utf8_lossy(&output.stdout)
                    );
                }
            }
            Err(e) => {
                log::error!("Failed to run poetry lock: {}", e);
                return;
            }
        }

        match Command::new("poetry")
            .current_dir(&project_path)
            .arg("install")
            .output()
        {
            Ok(output) => {
                if !output.status.success() {
                    log::error!(
                        "Poetry install failed: {}",
                        String::from_utf8_lossy(&output.stderr)
                    );
                    return;
                } else {
                    log::info!(
                        "Poetry install succeeded: {}",
                        String::from_utf8_lossy(&output.stdout)
                    );
                }
            }
            Err(e) => {
                log::error!("Failed to run poetry install: {}", e);
                return;
            }
        }

        let poetry_cmd = Command::new("poetry")
            .current_dir(&project_path)
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
