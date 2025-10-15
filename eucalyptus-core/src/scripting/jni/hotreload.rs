use std::path::Path;
use crossbeam_channel::Sender;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use crate::scripting::get_gradle_command;

pub enum HotReloadEvent {
    SuccessBuild,
    FailedBuild(String),
}

pub struct HotReloader {
    cancellation_token: CancellationToken,
    handle: Option<JoinHandle<()>>,
}

impl HotReloader {
    pub fn new() -> Self {
        Self {
            cancellation_token: CancellationToken::new(),
            handle: None,
        }
    }

    pub fn start(
        &mut self,
        project_root: impl AsRef<Path>,
        sender: Sender<HotReloadEvent>,
    ) {
        if self.is_running() {
            self.stop();
        }

        let project_root = project_root.as_ref().to_path_buf();
        let token = self.cancellation_token.clone();

        let handle = tokio::spawn(async move {
            let gradle_cmd = get_gradle_command(project_root.clone());

            loop {
                if token.is_cancelled() {
                    log::info!("Hot reloader cancelled, stopping...");
                    break;
                }

                let mut child = match Command::new(&gradle_cmd)
                    .current_dir(&project_root)
                    .args(["--continuous", "--console=plain", "jvmJar"])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .kill_on_drop(true)
                    .spawn()
                {
                    Ok(child) => child,
                    Err(e) => {
                        log::error!("Failed to start continuous build: {}", e);

                        tokio::select! {
                            _ = token.cancelled() => break,
                            _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => continue,
                        }
                    }
                };

                let stdout = child.stdout.take().expect("Stdout was piped");
                let stderr = child.stderr.take().expect("Stderr was piped");

                let stdout_handle = tokio::spawn({
                    let sender = sender.clone();
                    let token = token.clone();

                    async move {
                        let mut reader = BufReader::new(stdout).lines();

                        loop {
                            tokio::select! {
                                _ = token.cancelled() => {
                                    log::debug!("Stdout reader cancelled");
                                    break;
                                }
                                result = reader.next_line() => {
                                    match result {
                                        Ok(Some(line)) => {
                                            log::debug!("[Gradle] {}", line);

                                            if line.contains("BUILD SUCCESSFUL") {
                                                log::info!("Build completed, triggering hot-reload...");
                                                let _ = sender.send(HotReloadEvent::SuccessBuild);
                                            } else if line.contains("BUILD FAILED") {
                                                log::error!("Build failed, waiting for next change...");
                                                let _ = sender.send(HotReloadEvent::FailedBuild(line.clone()));
                                            }
                                        }
                                        Ok(None) => break, // EOF
                                        Err(e) => {
                                            log::error!("Error reading stdout: {}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                });

                let stderr_handle = tokio::spawn({
                    let token = token.clone();

                    async move {
                        let mut reader = BufReader::new(stderr).lines();

                        loop {
                            tokio::select! {
                                _ = token.cancelled() => {
                                    log::debug!("Stderr reader cancelled");
                                    break;
                                }
                                result = reader.next_line() => {
                                    match result {
                                        Ok(Some(line)) => {
                                            log::warn!("[Gradle Error] {}", line);
                                        }
                                        Ok(None) => break, // EOF
                                        Err(e) => {
                                            log::error!("Error reading stderr: {}", e);
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                });

                tokio::select! {
                    _ = token.cancelled() => {
                        log::info!("Hot reloader cancelled, cleaning up...");
                        let _ = child.kill().await;

                        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

                        stdout_handle.abort();
                        stderr_handle.abort();
                        let _ = tokio::join!(stdout_handle, stderr_handle);
                        break;
                    }
                    status = child.wait() => {
                        match status {
                            Ok(exit_status) => {
                                log::warn!("Gradle process exited with status: {}, restarting...", exit_status);
                            }
                            Err(e) => {
                                log::error!("Error waiting for gradle process: {}", e);
                            }
                        }
                        let _ = tokio::join!(stdout_handle, stderr_handle);
                    }
                }

                tokio::select! {
                    _ = token.cancelled() => break,
                    _ = tokio::time::sleep(tokio::time::Duration::from_secs(2)) => {}
                }
            }

            log::info!("Hot reloader task finished");
        });

        self.handle = Some(handle);
    }

    pub fn stop(&mut self) {
        log::info!("Stopping hot reloader...");

        self.cancellation_token.cancel();

        if let Some(handle) = self.handle.take() {
            handle.abort();
        }

        self.cancellation_token = CancellationToken::new();
    }

    pub fn is_running(&self) -> bool {
        self.handle.as_ref().map_or(false, |h| !h.is_finished())
            && !self.cancellation_token.is_cancelled()
    }
}

impl Default for HotReloader {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for HotReloader {
    fn drop(&mut self) {
        self.stop();
    }
}