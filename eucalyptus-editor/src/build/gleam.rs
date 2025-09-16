use app_dirs2::{AppDataType, AppInfo, app_dir};
use futures_util::StreamExt;
use std::path::PathBuf;
use std::process::Command;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc::UnboundedSender;

const GLEAM_VERSION: &'static str = "1.12.0";
const BUN_VERSION: &'static str = "1.2.22";
const JAVY_VERSION: &'static str = "6.0.0";

pub const APP_INFO: AppInfo = AppInfo {
    name: "Eucalyptus",
    author: "4tkbytes",
};

pub enum InstallStatus {
    NotStarted,
    InProgress {
        tool: String,
        step: String,
        progress: f32,
    },
    Success,
    Failed(String),
}

/// Compiles a gleam project into WASM through a pipeline.
pub struct GleamScriptCompiler {
    #[allow(dead_code)]
    project_location: PathBuf,
}

impl GleamScriptCompiler {
    pub fn new(project_location: &PathBuf) -> Self {
        GleamScriptCompiler {
            project_location: project_location.clone(),
        }
    }

    pub async fn evaluate(&self) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn build(
        &self,
        sender: Option<UnboundedSender<InstallStatus>>,
    ) -> anyhow::Result<()> {
        Self::ensure_dependencies(sender).await?;
        Ok(())
    }

    pub async fn ensure_dependencies(
        sender: Option<UnboundedSender<InstallStatus>>,
    ) -> anyhow::Result<()> {
        println!("Checking dependencies...");

        if let Some(ref s) = sender {
            let _ = s.send(InstallStatus::InProgress {
                tool: "None".to_string(),
                step: "Checking existing tools...".to_string(),
                progress: 0.05,
            });
        }

        let gleam_available = Self::check_tool_in_path("gleam").await;
        let bun_available = Self::check_tool_in_path("bun").await;
        let javy_available = Self::check_tool_in_path("javy").await;

        if gleam_available && bun_available && javy_available {
            println!("All dependencies found in PATH");
            if let Some(ref s) = sender {
                let _ = s.send(InstallStatus::Success);
            }
            return Ok(());
        }

        if !(cfg!(target_os = "windows") || cfg!(target_os = "linux") || cfg!(target_os = "macos"))
        {
            anyhow::bail!("The operating system is not supported for building the Gleam project")
        }

        let app_dir = app_dir(AppDataType::UserData, &APP_INFO, "")
            .map_err(|e| anyhow::anyhow!("Failed to get app directory: {}", e))?;

        let tools_to_install: Vec<(&str, bool)> = vec![
            ("Gleam", gleam_available),
            ("Bun", bun_available),
            ("Javy", javy_available),
        ];
        let total_to_install = tools_to_install.iter().filter(|(_, avail)| !avail).count();
        let mut installed_count = 0;

        if !gleam_available {
            if let Some(ref s) = sender.clone() {
                let _ = s.send(InstallStatus::InProgress {
                    tool: "Gleam".to_string(),
                    step: "Downloading Gleam...".to_string(),
                    progress: 0.2 + (installed_count as f32 / total_to_install as f32) * 0.6,
                });
            }
            Self::download_gleam(&app_dir, sender.clone()).await?;
            installed_count += 1;
        } else {
            if let Some(ref s) = sender.clone() {
                let _ = s.send(InstallStatus::InProgress {
                    tool: "Gleam".to_string(),
                    step: "Done!".to_string(),
                    progress: 1.0,
                });
            }
            installed_count += 1;
        }

        if !bun_available {
            if let Some(ref s) = sender.clone() {
                let _ = s.send(InstallStatus::InProgress {
                    tool: "Bun".to_string(),
                    step: "Downloading Bun...".to_string(),
                    progress: 0.2 + (installed_count as f32 / total_to_install as f32) * 0.6,
                });
            }
            Self::download_bun(&app_dir, sender.clone()).await?;
            installed_count += 1;
        } else {
            if let Some(ref s) = sender.clone() {
                let _ = s.send(InstallStatus::InProgress {
                    tool: "Bun".to_string(),
                    step: "Done!".to_string(),
                    progress: 1.0,
                });
            }
            installed_count += 1;
        }

        if !javy_available {
            if let Some(ref s) = sender.clone() {
                let _ = s.send(InstallStatus::InProgress {
                    tool: "Javy".to_string(),
                    step: "Downloading Javy...".to_string(),
                    progress: 0.2 + (installed_count as f32 / total_to_install as f32) * 0.6,
                });
            }
            Self::download_javy(&app_dir, sender.clone()).await?;
            installed_count += 1;
        } else {
            if let Some(ref s) = sender.clone() {
                let _ = s.send(InstallStatus::InProgress {
                    tool: "Javy".to_string(),
                    step: "Done!".to_string(),
                    progress: 1.0,
                });
            }
            installed_count += 1;
        }

        if let Some(ref s) = sender.clone() {
            let _ = s.send(InstallStatus::Success);
        }

        println!(
            "All {} dependencies installed successfully",
            installed_count
        );
        Ok(())
    }

    async fn check_tool_in_path(tool: &str) -> bool {
        let cmd = if cfg!(target_os = "windows") {
            Command::new("where").arg(tool).output()
        } else {
            Command::new("which").arg(tool).output()
        };

        match cmd {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    pub async fn download_gleam(
        app_dir: &PathBuf,
        sender: Option<UnboundedSender<InstallStatus>>,
    ) -> anyhow::Result<()> {
        let gleam_dir = app_dir
            .join("dependencies")
            .join("gleam")
            .join(GLEAM_VERSION);

        if gleam_dir.exists() {
            println!(
                "Gleam v{} already cached at {}",
                GLEAM_VERSION,
                app_dir.display()
            );
            return Ok(());
        }

        println!("Downloading Gleam v{}...", GLEAM_VERSION);

        let gleam_link = Self::get_gleam_download_url()?;
        Self::download_and_extract(&gleam_link, &gleam_dir, "gleam", sender).await?;

        println!("Gleam v{} downloaded successfully", GLEAM_VERSION);
        Ok(())
    }

    pub async fn download_bun(
        app_dir: &PathBuf,
        sender: Option<UnboundedSender<InstallStatus>>,
    ) -> anyhow::Result<()> {
        let bun_dir = app_dir.join("dependencies").join("bun").join(BUN_VERSION);

        if bun_dir.exists() {
            println!(
                "Bun v{} already cached at {}",
                BUN_VERSION,
                app_dir.display()
            );
            return Ok(());
        }

        println!("Downloading Bun v{}...", BUN_VERSION);

        let bun_link = Self::get_bun_download_url()?;
        Self::download_and_extract(&bun_link, &bun_dir, "bun", sender).await?;

        println!("Bun v{} downloaded successfully", BUN_VERSION);
        Ok(())
    }

    pub async fn download_javy(
        app_dir: &PathBuf,
        sender: Option<UnboundedSender<InstallStatus>>,
    ) -> anyhow::Result<()> {
        let javy_dir = app_dir.join("dependencies").join("javy").join(JAVY_VERSION);

        if javy_dir.exists() {
            println!(
                "Javy v{} already cached at {}",
                JAVY_VERSION,
                app_dir.display()
            );
            return Ok(());
        }

        println!("Downloading Javy v{}...", JAVY_VERSION);

        let javy_link = Self::get_javy_download_url()?;
        Self::download_and_extract(&javy_link, &javy_dir, "javy", sender).await?;

        println!("Javy v{} downloaded successfully", JAVY_VERSION);
        Ok(())
    }

    async fn download_and_extract(
        url: &str,
        target_dir: &PathBuf,
        tool_name: &str,
        sender: Option<UnboundedSender<InstallStatus>>,
    ) -> anyhow::Result<()> {
        if let Some(ref s) = sender {
            let _ = s.send(InstallStatus::InProgress {
                step: format!("Creating directories for {}...", tool_name),
                progress: 0.0,
                tool: tool_name.to_string(),
            });
        }

        fs::create_dir_all(target_dir).await?;

        if let Some(ref s) = sender {
            let _ = s.send(InstallStatus::InProgress {
                step: format!("Downloading {}...", tool_name),
                progress: 0.3,
                tool: tool_name.to_string(),
            });
        }

        let response = reqwest::get(url).await?;
        let total_size = response.content_length().unwrap_or(0);
        let mut downloaded = 0;
        let mut stream = response.bytes_stream();

        let temp_file = target_dir.join(format!("{}_download", tool_name));
        let mut file = fs::File::create(&temp_file).await?;

        while let Some(item) = stream.next().await {
            let bytes = item?;
            file.write_all(&bytes).await?;
            downloaded += bytes.len() as u64;

            if let Some(ref s) = sender {
                let progress = 0.3 + (downloaded as f32 / total_size as f32) * 0.4;
                let _ = s.send(InstallStatus::InProgress {
                    step: format!("Downloading {}...", tool_name),
                    progress,
                    tool: tool_name.to_string(),
                });
            }
        }

        file.sync_all().await?;
        drop(file);

        if let Some(ref s) = sender {
            let _ = s.send(InstallStatus::InProgress {
                step: format!("Extracting {}...", tool_name),
                progress: 0.7,
                tool: tool_name.to_string(),
            });
        }

        if url.ends_with(".zip") {
            Self::extract_zip(&temp_file, target_dir).await?;
        } else if url.ends_with(".tar.gz") {
            Self::extract_tar_gz(&temp_file, target_dir).await?;
        } else if url.ends_with(".gz") {
            Self::extract_gz(&temp_file, target_dir, tool_name).await?;
        }

        fs::remove_file(&temp_file).await?;

        if let Some(ref s) = sender {
            let _ = s.send(InstallStatus::InProgress {
                step: format!("{} installation complete", tool_name),
                progress: 0.9,
                tool: tool_name.to_string(),
            });
        }

        Ok(())
    }

    async fn extract_zip(archive: &PathBuf, target_dir: &PathBuf) -> anyhow::Result<()> {
        let file = std::fs::File::open(archive)?;
        let mut archive = zip::ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = target_dir.join(file.name());

            if file.is_dir() {
                fs::create_dir_all(&outpath).await?;
            } else {
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p).await?;
                }
                let mut outfile = fs::File::create(&outpath).await?;
                let mut buffer = Vec::new();
                std::io::Read::read_to_end(&mut file, &mut buffer)?;
                outfile.write_all(&buffer).await?;
            }
        }
        Ok(())
    }

    async fn extract_tar_gz(archive: &PathBuf, target_dir: &PathBuf) -> anyhow::Result<()> {
        let file = std::fs::File::open(archive)?;
        let tar = flate2::read::GzDecoder::new(file);
        let mut archive = tar::Archive::new(tar);

        for entry in archive.entries()? {
            let mut entry = entry?;
            let path = target_dir.join(entry.path()?);
            entry.unpack(path)?;
        }
        Ok(())
    }

    async fn extract_gz(
        archive: &PathBuf,
        target_dir: &PathBuf,
        tool_name: &str,
    ) -> anyhow::Result<()> {
        let file = std::fs::File::open(archive)?;
        let mut decoder = flate2::read::GzDecoder::new(file);
        let mut buffer = Vec::new();
        std::io::Read::read_to_end(&mut decoder, &mut buffer)?;

        let exe_name = if cfg!(target_os = "windows") {
            format!("{}.exe", tool_name)
        } else {
            tool_name.to_string()
        };

        let output_path = target_dir.join(exe_name);
        let mut output_file = fs::File::create(&output_path).await?;
        output_file.write_all(&buffer).await?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = output_file.metadata().await?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&output_path, perms).await?;
        }

        Ok(())
    }

    fn get_gleam_download_url() -> anyhow::Result<String> {
        let gleam_link = {
            #[cfg(target_os = "windows")]
            {
                let arch = {
                    if cfg!(target_arch = "aarch64") {
                        "aarch64"
                    } else if cfg!(target_arch = "x86_64") {
                        "x86_64"
                    } else {
                        anyhow::bail!(
                            "This architecture is not supported for building the gleam project"
                        );
                    }
                };
                format!(
                    "https://github.com/gleam-lang/gleam/releases/download/v{}/gleam-v{}-{}-pc-windows-msvc.zip",
                    GLEAM_VERSION, GLEAM_VERSION, arch,
                )
            }

            #[cfg(target_os = "linux")]
            {
                let arch = {
                    if cfg!(target_arch = "aarch64") {
                        "aarch64"
                    } else if cfg!(target_arch = "x86_64") {
                        "x86_64"
                    } else {
                        anyhow::bail!(
                            "This architecture is not supported for building the gleam project"
                        );
                    }
                };
                format!(
                    "https://github.com/gleam-lang/gleam/releases/download/v{}/gleam-v{}-{}-unknown-linux-musl.tar.gz",
                    GLEAM_VERSION, GLEAM_VERSION, arch,
                )
            }

            #[cfg(target_os = "macos")]
            {
                let arch = {
                    if cfg!(target_arch = "aarch64") {
                        "aarch64"
                    } else if cfg!(target_arch = "x86_64") {
                        "x86_64"
                    } else {
                        anyhow::bail!(
                            "This architecture is not supported for building the gleam project"
                        );
                    }
                };
                format!(
                    "https://github.com/gleam-lang/gleam/releases/download/v{}/gleam-v{}-{}-apple-darwin.tar.gz",
                    GLEAM_VERSION, GLEAM_VERSION, arch,
                )
            }
        };
        Ok(gleam_link)
    }

    fn get_bun_download_url() -> anyhow::Result<String> {
        let bun_link = {
            #[cfg(target_os = "windows")]
            {
                let arch = {
                    if cfg!(target_arch = "aarch64") {
                        "aarch64"
                    } else if cfg!(target_arch = "x86_64") {
                        "x64"
                    } else {
                        anyhow::bail!(
                            "This architecture is not supported for building the gleam project"
                        );
                    }
                };
                format!(
                    "https://github.com/oven-sh/bun/releases/download/bun-v{}/bun-windows-{}.zip",
                    BUN_VERSION, arch,
                )
            }

            #[cfg(target_os = "linux")]
            {
                let arch = {
                    if cfg!(target_arch = "aarch64") {
                        "aarch64"
                    } else if cfg!(target_arch = "x86_64") {
                        "x64"
                    } else {
                        anyhow::bail!(
                            "This architecture is not supported for building the gleam project"
                        );
                    }
                };
                format!(
                    "https://github.com/oven-sh/bun/releases/download/bun-v{}/bun-linux-{}.zip",
                    BUN_VERSION, arch,
                )
            }

            #[cfg(target_os = "macos")]
            {
                let arch = {
                    if cfg!(target_arch = "aarch64") {
                        "aarch64"
                    } else if cfg!(target_arch = "x86_64") {
                        "x64"
                    } else {
                        anyhow::bail!(
                            "This architecture is not supported for building the gleam project"
                        );
                    }
                };
                format!(
                    "https://github.com/oven-sh/bun/releases/download/bun-v{}/bun-darwin-{}.zip",
                    BUN_VERSION, arch,
                )
            }
        };
        Ok(bun_link)
    }

    fn get_javy_download_url() -> anyhow::Result<String> {
        let javy_link = {
            #[cfg(target_os = "windows")]
            {
                let arch = {
                    if cfg!(target_arch = "aarch64") {
                        anyhow::bail!(
                            "This arch is not available for prebuilt download. Please build this from source"
                        );
                    } else if cfg!(target_arch = "x86_64") {
                        "x86_64"
                    } else {
                        anyhow::bail!(
                            "This architecture is not supported for building the gleam project"
                        );
                    }
                };
                format!(
                    "https://github.com/bytecodealliance/javy/releases/download/v{}/javy-{}-windows-v{}.gz",
                    JAVY_VERSION, arch, JAVY_VERSION
                )
            }

            #[cfg(target_os = "linux")]
            {
                let arch = {
                    if cfg!(target_arch = "aarch64") {
                        "arm"
                    } else if cfg!(target_arch = "x86_64") {
                        "x86_64"
                    } else {
                        anyhow::bail!(
                            "This architecture is not supported for building the gleam project"
                        );
                    }
                };
                format!(
                    "https://github.com/bytecodealliance/javy/releases/download/v{}/javy-{}-linux-v{}.gz",
                    JAVY_VERSION, arch, JAVY_VERSION
                )
            }

            #[cfg(target_os = "macos")]
            {
                let arch = {
                    if cfg!(target_arch = "aarch64") {
                        "arm"
                    } else if cfg!(target_arch = "x86_64") {
                        "x86_64"
                    } else {
                        anyhow::bail!(
                            "This architecture is not supported for building the gleam project"
                        );
                    }
                };
                format!(
                    "https://github.com/bytecodealliance/javy/releases/download/v{}/javy-{}-macos-v{}.gz",
                    JAVY_VERSION, arch, JAVY_VERSION
                )
            }
        };
        Ok(javy_link)
    }
}

#[tokio::test]
async fn check_if_dependencies_install() {
    GleamScriptCompiler::ensure_dependencies(None)
        .await
        .unwrap();
}
