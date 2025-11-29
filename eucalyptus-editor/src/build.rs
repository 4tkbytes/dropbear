use eucalyptus_core::config::ProjectConfig;
use eucalyptus_core::runtime::RuntimeProjectConfig;
use eucalyptus_core::scene::SceneConfig;
use std::fs;
use std::path::{Path, PathBuf};

/// Builds a eucalyptus project into a single bundle. 
/// 
/// Returns the path of the build directory
pub fn build(project_config: PathBuf) -> anyhow::Result<PathBuf> {
    log::info!("Started project building");
    // create a build directory
    let project_root = project_config
        .parent()
        .ok_or(anyhow::anyhow!("Unable to locate parent folder of config"))?
        .to_path_buf();
    let build_dir = project_root.join("build/output");

    if build_dir.exists() {
        fs::remove_dir_all(&build_dir)?;
    }
    fs::create_dir_all(&build_dir)?;
    log::debug!("Readied build directory");

    // load the project config manually to avoid overwriting global state
    let ron_str = fs::read_to_string(&project_config)?;
    let mut config: ProjectConfig = ron::de::from_str(&ron_str)?;
    config.project_path = project_root.clone();
    log::debug!("Loaded project config");

    // load scenes
    let mut scenes = Vec::new();
    let scene_folder = project_root.join("scenes");
    if scene_folder.exists() {
        for entry in fs::read_dir(scene_folder)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("eucs") {
                match SceneConfig::read_from(&path) {
                    Ok(scene) => {
                        scenes.push(scene);
                    }
                    Err(e) => {
                        log::warn!("Failed to load scene {:?} during build: {}", path, e);
                    }
                }
            }
        }
    }

    // convert to runtime project config
    let runtime_config = RuntimeProjectConfig {
        project_name: config.project_name.clone(),
        runtime_settings: config.runtime_settings.clone(),
        scenes,
    };
    log::debug!("Converted to runtime project config");

    // export to .eupak
    let eupak_path = build_dir.join("data.eupak");
    let config_bytes = bincode::encode_to_vec(&runtime_config, bincode::config::standard())?;
    fs::write(&eupak_path, config_bytes)?;
    log::debug!("Exported scene config to {:?}", eupak_path);
    
    // copy resources
    let resources_src = project_root.join("resources");
    let resources_dst = build_dir.join("resources");
    if resources_src.exists() {
        copy_dir_recursive(&resources_src, &resources_dst)?;
        log::debug!("Copied resources to {:?}", resources_dst);
    }

    log::info!("Done!");

    Ok(build_dir)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> anyhow::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_recursive(&entry.path(), &dst.join(entry.file_name()))?;
        } else {
            fs::copy(entry.path(), dst.join(entry.file_name()))?;
        }
    }
    Ok(())
}

/// Reads the contents of a data.eupak file into a pretty print format. 
/// 
/// Returns the contents of the project config. 
pub fn read(eupak: PathBuf) -> anyhow::Result<RuntimeProjectConfig> {
    let bytes = std::fs::read(&eupak)?;
    let (content, _): (RuntimeProjectConfig, usize) =
        bincode::decode_from_slice(&bytes, bincode::config::standard())?;
    println!("{} contents: {:#?}", eupak.display(), content);
    Ok(content)
}