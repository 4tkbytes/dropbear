use clap::ArgMatches;
use std::path::Path;
use std::{collections::HashMap, fs, path::PathBuf, process::Command};
use zip::write::SimpleFileOptions;
use eucalyptus_core::config::ProjectConfig;
use eucalyptus_core::states::{RuntimeData, SCENES, SOURCE};

pub fn package(project_path: PathBuf, _sub_matches: &ArgMatches) -> anyhow::Result<()> {
    if !project_path.exists() {
        return Err(anyhow::anyhow!("Unable to locate project config file"));
    }

    let build_dir = project_path
        .parent()
        .ok_or(anyhow::anyhow!("Unable to get parent"))?
        .join("build");

    // check health
    println!("Checking health (checking if commands exist)");
    health()?;
    println!("Health check completed!");

    let clone_dir = build_dir.join("redback-runtime");

    if clone_dir.exists() {
        println!("Repository directory exists, checking for updates...");
        if should_update_repository(&clone_dir)? {
            println!("Repository has changes or is outdated, removing and re-cloning...");
            std::fs::remove_dir_all(&clone_dir)?;
            clone_repository(&build_dir)?;
        } else {
            println!("Repository is up to date, skipping clone");
        }
    } else {
        println!("Cloning repository");
        clone_repository(&build_dir)?;
    }

    let project_config = ProjectConfig::read_from(&project_path)?;
    let project_name = project_config.project_name.clone();

    // cd into redback-runtime folder and compile redback-runtime using cargo
    let runtime_dir = build_dir.join("redback-runtime");
    if !runtime_dir.exists() {
        return Err(anyhow::anyhow!(
            "redback-runtime directory not found after cloning"
        ));
    }

    let cargo_toml_path = runtime_dir.join("Cargo.toml");
    if cargo_toml_path.exists() {
        let cargo_toml_content = std::fs::read_to_string(&cargo_toml_path)?;
        let modified_content = cargo_toml_content.replace(
            r#"name = "redback-runtime""#,
            &format!(r#"name = "{}""#, project_name),
        );
        std::fs::write(&cargo_toml_path, modified_content)?;
        println!("Updated Cargo.toml with project name: {}", project_name);
    }

    println!("Building {} for release", project_name);
    let mut cargo_build = Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&runtime_dir)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    let exit_status = cargo_build.wait()?;

    if !exit_status.success() {
        return Err(anyhow::anyhow!("Failed to build {}", project_name));
    }
    println!("{} built successfully!", project_name);

    let target_dir = runtime_dir.join("target").join("release");
    let exe_name = if cfg!(target_os = "windows") {
        format!("{}.exe", project_name)
    } else {
        project_name.clone()
    };

    let built_exe = target_dir.join(&exe_name);
    if !built_exe.exists() {
        return Err(anyhow::anyhow!(
            "Built executable not found at: {}",
            built_exe.display()
        ));
    }

    println!("Building project data (.eupak file)");
    build(project_path.clone())?;

    let output_dir = project_path
        .parent()
        .ok_or(anyhow::anyhow!("Unable to get parent"))?
        .join("build")
        .join("package");
    std::fs::create_dir_all(&output_dir)?;

    let output_exe = output_dir.join(&exe_name);

    println!("Copying executable to: {}", output_exe.display());
    std::fs::copy(&built_exe, &output_exe)?;

    let eupak_source = project_path
        .parent()
        .ok_or(anyhow::anyhow!("Unable to get parent"))?
        .join("build")
        .join("output")
        .join(format!("{}.eupak", project_name));
    let eupak_dest = output_dir.join(format!("{}.eupak", project_name));

    if !eupak_source.exists() {
        return Err(anyhow::anyhow!(
            "Expected .eupak file not found at: {}",
            eupak_source.display()
        ));
    }

    println!("Copying .eupak file to: {}", eupak_dest.display());
    std::fs::copy(&eupak_source, &eupak_dest)?;

    let project_resources = project_path
        .parent()
        .ok_or(anyhow::anyhow!("Unable to get parent"))?
        .join("resources");
    if project_resources.exists() {
        println!("Copying resources folder...");
        let output_resources = output_dir.join("resources");
        copy_resources_folder(&project_resources, &output_resources)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&output_exe)?.permissions();
        perms.set_mode(0o755); // rwxr-xr-x
        std::fs::set_permissions(&output_exe, perms)?;
    }

    copy_system_libraries(&output_dir)?;

    println!("Creating zip package...");
    let zip_path = project_path
        .parent()
        .ok_or(anyhow::anyhow!("Unable to get parent"))?
        .join("build")
        .join(format!("{}.zip", project_name));
    create_zip_package(&output_dir, &zip_path)?;

    println!("Cleaning up temporary files");
    if build_dir.join("redback-runtime").exists() {
        std::fs::remove_dir_all(build_dir.join("redback-runtime"))?;
    }
    if build_dir.join("output").exists() {
        std::fs::remove_dir_all(build_dir.join("output"))?;
    }

    println!("\n✓ Package completed successfully!");
    println!("Output directory: {}", output_dir.display());
    println!("Zip package: {}", zip_path.display());
    println!("Executable: {}", exe_name);
    println!("Data file: {}.eupak", project_name);

    Ok(())
}

fn clone_repository(build_dir: impl AsRef<Path>) -> anyhow::Result<()> {
    git2::build::RepoBuilder::new().clone(
        "https://github.com/4tkbytes/redback-runtime",
        &build_dir.as_ref().join("redback-runtime"),
    )?;
    println!("Repository cloned successfully!");
    Ok(())
}

fn should_update_repository(repo_dir: impl AsRef<Path>) -> anyhow::Result<bool> {
    let repo = match git2::Repository::open(repo_dir) {
        Ok(repo) => repo,
        Err(_) => {
            return Ok(true);
        }
    };

    let statuses = repo.statuses(None)?;
    if !statuses.is_empty() {
        println!("Found local changes in repository");
        return Ok(true);
    }

    let mut remote = repo.find_remote("origin")?;
    remote.fetch(&["refs/heads/*:refs/remotes/origin/*"], None, None)?;

    let head = repo
        .head()?
        .target()
        .ok_or(anyhow::anyhow!("No HEAD commit"))?;

    let remote_ref = if let Ok(remote_main) = repo.find_reference("refs/remotes/origin/main") {
        remote_main
    } else if let Ok(remote_master) = repo.find_reference("refs/remotes/origin/master") {
        remote_master
    } else {
        return Ok(true);
    };

    let remote_commit = remote_ref
        .target()
        .ok_or(anyhow::anyhow!("No remote commit"))?;

    Ok(head != remote_commit)
}

fn copy_resources_folder(src: impl AsRef<Path>, dest: impl AsRef<Path>) -> anyhow::Result<()> {
    fs::create_dir_all(dest.as_ref())?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let file_name = entry.file_name();

        if file_name == "resources.eucc" {
            continue;
        }

        let dest_path = dest.as_ref().join(&file_name);

        if src_path.is_dir() {
            copy_resources_folder(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }

    Ok(())
}

fn copy_system_libraries(output_dir: impl AsRef<Path>) -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        let dll_paths = vec![
            "C:\\vcpkg\\installed\\x64-windows\\bin\\assimp-vc143-mt.dll",
            "C:\\vcpkg\\installed\\x64-windows\\bin\\assimp.dll",
            "C:\\Program Files\\Assimp\\bin\\assimp.dll",
            "C:\\Program Files (x86)\\Assimp\\bin\\assimp.dll",
        ];

        for dll_path in dll_paths {
            if std::path::Path::new(dll_path).exists() {
                let dll_name = std::path::Path::new(dll_path).file_name().unwrap();
                let dest = output_dir.as_ref().join(dll_name);
                std::fs::copy(dll_path, dest)?;
                println!("Copied system library: {}", dll_name.to_string_lossy());
                break;
            }
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let lib_dir = output_dir.as_ref().join("lib");
        std::fs::create_dir_all(&lib_dir)?;

        let lib_paths = vec![
            "/usr/lib/libassimp.so",
            "/usr/lib/x86_64-linux-gnu/libassimp.so",
            "/usr/lib64/libassimp.so",
            "/usr/local/lib/libassimp.so",
            "/opt/homebrew/lib/libassimp.dylib",
            "/usr/local/lib/libassimp.dylib",
        ];

        for lib_path in lib_paths {
            if std::path::Path::new(lib_path).exists() {
                let lib_name = std::path::Path::new(lib_path).file_name().unwrap();
                let dest = lib_dir.join(lib_name);
                std::fs::copy(lib_path, dest)?;
                println!("Copied system library: {}", lib_name.to_string_lossy());
                break;
            }
        }
    }

    Ok(())
}

fn create_zip_package(
    source_dir: impl AsRef<Path>,
    zip_path: impl AsRef<Path>,
) -> anyhow::Result<()> {
    let file = fs::File::create(zip_path.as_ref())?;
    let mut zip = zip::ZipWriter::new(file);

    let walkdir = walkdir::WalkDir::new(source_dir.as_ref());
    for entry in walkdir {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            let relative_path = path.strip_prefix(source_dir.as_ref())?;
            let name = relative_path.to_string_lossy();

            let options: SimpleFileOptions = Default::default();
            zip.start_file(name, options)?;
            let mut file = std::fs::File::open(path)?;
            std::io::copy(&mut file, &mut zip)?;
        }
    }

    zip.finish()?;
    println!("Created zip package: {}", zip_path.as_ref().display());
    Ok(())
}

pub fn read_from_eupak(eupak_path: PathBuf) -> anyhow::Result<()> {
    let bytes = std::fs::read(&eupak_path)?;
    let (content, _): (RuntimeData, usize) =
        bincode::decode_from_slice(&bytes, bincode::config::standard())?;
    println!("{} contents: {:#?}", eupak_path.display(), content);
    Ok(())
}

pub fn build(
    project_path: PathBuf,
    // _sub_matches: &ArgMatches
) -> anyhow::Result<PathBuf> {
    println!(" > Starting build");
    if !project_path.exists() {
        return Err(anyhow::anyhow!(format!(
            "Unable to locate project config file: [{}]",
            project_path.display()
        )));
    }
    // ProjectConfig::read_from(&project_path)?.load_config_to_memory()?;

    let mut project_config = ProjectConfig::read_from(&project_path)?;
    log::info!(" > Reading from project config");
    project_config.load_config_to_memory()?;
    log::info!(" > Loading config to memory");

    let scene_data = {
        let scenes_guard = SCENES.read();
        scenes_guard.clone()
    };
    log::info!(" > Copied scene data");

    let source_config = {
        let source_guard = SOURCE.read();
        source_guard.clone()
    };
    log::info!(" > Captured source config");

    let build_dir = project_path.parent().unwrap().join("build").join("output");
    fs::create_dir_all(&build_dir)?;
    log::info!(" > Created build dir");

    let project_name = project_config.project_name.clone();

    let mut scripts = HashMap::new();
    let script_dir = project_path.parent().unwrap().join("src");
    if script_dir.exists() {
        for entry in fs::read_dir(&script_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension()
                && ext == "rhai"
            {
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                let contents = fs::read_to_string(&path)?;
                println!(" > Copied script info from [{}]", name);
                scripts.insert(name, contents);
            }
        }
    }

    let runtime_data = RuntimeData {
        project_config,
        source_config,
        scene_data,
        scripts,
    };
    log::info!(" > Created runtime data structures");

    let runtime_file = build_dir.join(format!("{}.eupak", project_name));
    let serialized = bincode::serde::encode_to_vec(runtime_data, bincode::config::standard())?;
    std::fs::write(&runtime_file, serialized)?;
    log::info!(" > Written the file to build location");

    println!(
        "Build completed successfully. Output at {:?}",
        runtime_file.display()
    );
    Ok(runtime_file)
}

pub fn health() -> anyhow::Result<()> {
    let mut all_healthy = true;

    match Command::new("cargo").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
                println!("Does cargo exist? ✓ YES - {}", version);

                match Command::new("rustc").arg("--version").output() {
                    Ok(rustc_output) => {
                        if rustc_output.status.success() {
                            let rustc_version = String::from_utf8_lossy(&rustc_output.stdout)
                                .trim()
                                .to_string();
                            println!("Does rustc compiler exist? ✓ YES - {}", rustc_version);
                        } else {
                            println!("Does rustc compiler exist? ✗ NO - rustc command failed");
                            all_healthy = false;
                        }
                    }
                    Err(_) => {
                        println!("Does rustc compiler exist? ✗ NO - rustc not found in PATH");
                        all_healthy = false;
                    }
                }
            } else {
                println!("Does cargo exist? ✗ NO - cargo command failed");
                println!("Does rustc compiler exist? ✗ NO - cargo failed, cannot check rustc");
                all_healthy = false;
            }
        }
        Err(_) => {
            println!("Does cargo exist? ✗ NO - cargo not found in PATH");
            println!("Does rustc compiler exist? ✗ NO - cargo not found, cannot check rustc");
            all_healthy = false;
        }
    }

    let assimp_found = check_assimp_availability();
    if assimp_found {
        println!("Does assimp lib exist? ✓ YES - Found assimp library");
    } else {
        println!(
            "Does assimp lib exist? ⚠ MAYBE - Could not definitively locate assimp, but it may be available through system package manager or vcpkg"
        );
    }

    if all_healthy {
        println!("\n✓ All core tools are available!");
    } else {
        println!("\n✗ Some required tools are missing. Please install Rust and Cargo.");
        return Err(anyhow::anyhow!(
            "Health check failed - missing required tools"
        ));
    }

    Ok(())
}

fn check_assimp_availability() -> bool {
    #[cfg(target_os = "windows")]
    {
        let common_paths = vec![
            "C:\\vcpkg\\installed\\x64-windows\\lib\\assimp-vc143-mt.lib",
            "C:\\vcpkg\\installed\\x64-windows\\lib\\assimp.lib",
            "C:\\vcpkg\\installed\\x86-windows\\lib\\assimp-vc143-mt.lib",
            "C:\\vcpkg\\installed\\x86-windows\\lib\\assimp.lib",
            "C:\\Program Files\\Assimp\\lib\\assimp.lib",
            "C:\\Program Files (x86)\\Assimp\\lib\\assimp.lib",
        ];

        for path in common_paths {
            if std::path::Path::new(path).exists() {
                return true;
            }
        }

        if let Ok(output) = Command::new("where").arg("assimp.dll").output()
            && output.status.success()
        {
            return true;
        }
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let common_paths = vec![
            "/usr/lib/libassimp.so",
            "/usr/lib/x86_64-linux-gnu/libassimp.so",
            "/usr/lib64/libassimp.so",
            "/usr/local/lib/libassimp.so",
            "/opt/homebrew/lib/libassimp.dylib",
            "/usr/local/lib/libassimp.dylib",
        ];

        for path in common_paths {
            if std::path::Path::new(path).exists() {
                return true;
            }
        }

        if let Ok(output) = Command::new("pkg-config")
            .args(["--exists", "assimp"])
            .output()
            && output.status.success()
        {
            return true;
        }

        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = Command::new("ldconfig").args(["-p"]).output()
                && output.status.success()
            {
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("libassimp") {
                    return true;
                }
            }
        }
    }

    false
}
