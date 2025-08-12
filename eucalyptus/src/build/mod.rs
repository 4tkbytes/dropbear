use std::{collections::HashMap, fs, path::PathBuf};

use bincode::{Decode, Encode};
use clap::ArgMatches;

use crate::states::{ProjectConfig, SceneConfig, SourceConfig, SCENES, SOURCE};

pub fn package(_project_path: PathBuf, _sub_matches: &ArgMatches) {
    todo!()
}

pub fn read_from_eupak(eupak_path: PathBuf) -> anyhow::Result<()> {
    let bytes = std::fs::read(&eupak_path)?;
    let (content, _): (RuntimeData, usize) = bincode::decode_from_slice(&bytes, bincode::config::standard())?;
    println!("{} contents: {:#?}", eupak_path.display(), content);
    Ok(())
}

pub fn build(project_path: PathBuf, _sub_matches: &ArgMatches) -> anyhow::Result<()> {
    if !project_path.exists() {
        return Err(anyhow::anyhow!("Unable to locate project config file"));
    }
    ProjectConfig::read_from(&project_path)?.load_config_to_memory()?;
    
    let mut project_config = ProjectConfig::read_from(&project_path)?;
    project_config.load_config_to_memory()?;

    let source_config = {
        let source_guard = SOURCE.read().map_err(|_| anyhow::anyhow!("Unable to lock SOURCE"))?;
        source_guard.clone()
    };

    let scene_data = {
        let scenes_guard = SCENES.read().map_err(|_| anyhow::anyhow!("Unable to lock SCENES"))?;
        scenes_guard.clone()
    };

    let build_dir = project_path.parent().unwrap().join("build").join("output");
    std::fs::create_dir_all(&build_dir)?;

    let project_name = project_config.project_name.clone();

    let mut scripts = HashMap::new();
    let script_dir = project_path.parent().unwrap().join("src");
    if script_dir.exists() {
        for entry in fs::read_dir(&script_dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(ext) = path.extension() {
                if ext == "rhai" {
                    let name = path.file_name().unwrap().to_string_lossy().to_string();
                    let contents = fs::read_to_string(&path)?;
                    scripts.insert(name, contents);
                }
            }
        }
    }

    let runtime_data = RuntimeData {
        project_config,
        source_config,
        scene_data,
        scripts
    };

    let runtime_file = build_dir.join(format!("{}.eupak", project_name));
    let serialized = bincode::serde::encode_to_vec(runtime_data, bincode::config::standard())?;
    std::fs::write(&runtime_file, serialized)?;

    println!("Build completed successfully. Output at {:?}", runtime_file.display());
    Ok(())
}

#[derive(Decode, Encode, serde::Serialize, serde::Deserialize, Debug)]
pub struct RuntimeData {
    #[bincode(with_serde)]
    project_config: ProjectConfig,
    #[bincode(with_serde)]
    source_config: SourceConfig,
    #[bincode(with_serde)]
    scene_data: Vec<SceneConfig>,
    #[bincode(with_serde)]
    scripts: HashMap<String, String>,
}

pub fn health() {
    todo!()
}
