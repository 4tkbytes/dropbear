use std::path::Path;
use std::sync::Arc;
use crossbeam_channel::Sender;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use dropbear_engine::future::{FutureHandle, FutureQueue};
use crate::scripting::{get_gradle_command};

pub enum HotReloadEvent {
    SuccessBuild,
    FailedBuild(String),
}

pub struct HotReloader {
    cancellation_token: CancellationToken,
    handle: Option<FutureHandle>,
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
        future_queue: Arc<FutureQueue>,
        sender: Sender<HotReloadEvent>,
    ) {
        let project_root = project_root.as_ref().to_path_buf();
        let token = self.cancellation_token.clone();
        let sender_clone = sender.clone();

        let handle = future_queue.push(async move {
            let gradle_cmd = get_gradle_command(project_root.clone());

            loop {
                if token.is_cancelled() {
                    log::info!("Hot reloader cancelled, stopping...");
                    break;
                }

                let mut child = match Command::new(&gradle_cmd)
                    .current_dir(&project_root)
                    .args(["--continuous", "--console=plain", "fatJar"])
                    .stdout(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
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

                // Spawn child tasks with cancellation
                let stdout_handle = tokio::spawn({
                    let sender = sender_clone.clone();
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
                        stdout_handle.abort();
                        stderr_handle.abort();
                        break;
                    }
                    _ = child.wait() => {
                        log::warn!("Gradle process exited unexpectedly, restarting...");
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

        self.handle = Some(handle.clone());
    }

    pub fn stop(&mut self, future_queue: Arc<FutureQueue>) {
        self.cancellation_token.cancel();

        if let Some(handle) = self.handle.take() {
            future_queue.cancel(&handle);
            log::info!("Hot reloader stopped");
        }
    }

    pub fn is_running(&self) -> bool {
        self.handle.is_some() && !self.cancellation_token.is_cancelled()
    }
}

impl Default for HotReloader {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for HotReloader {
    fn drop(&mut self) {
        self.cancellation_token.cancel();
    }
}