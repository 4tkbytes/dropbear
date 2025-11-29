use crate::scene::{SceneConfig};
use crate::states::{PROJECT, SCENES};

/// The settings of a project in its runtime. 
/// 
/// This is different to [`SceneSettings`], which contains settings for ONLY
/// that specific scene. This is for any configurations of the project during its runtime, 
/// such as initial scene and stuff like that. 
#[derive(
    bincode::Decode,
    bincode::Encode,
    serde::Serialize,
    serde::Deserialize,
    Debug,
    Clone,
)]
pub struct RuntimeSettings {}

impl RuntimeSettings {
    /// Creates a new [`RuntimeSettings`] config. 
    pub fn new() -> Self {
        Self { }
    }
}

impl Default for RuntimeSettings {
    fn default() -> Self {
        Self::new()
    }
}

/// The configuration of a packaged eucalyptus project. 
/// 
/// Often stored as a single .eupak file, it contains all the scenes and the references of different
/// resources. 
#[derive(bincode::Decode, bincode::Encode, serde::Serialize, serde::Deserialize, Debug)]
pub struct RuntimeProjectConfig {
    #[bincode(with_serde)]
    pub project_name: String,

    // authoring stuff needs to be added, maybe later

    // versioning stuff too

    #[bincode(with_serde)]
    pub runtime_settings: RuntimeSettings,
    
    #[bincode(with_serde)]
    pub scenes: Vec<SceneConfig>,
}

impl RuntimeProjectConfig {
    /// Creates a [RuntimeProjectConfig] from a loaded [PROJECT] and [SCENES] states. 
    pub fn from_memory() -> Self {
        let project = PROJECT.read();
        let scenes = SCENES.read();
        
        Self {
            project_name: project.project_name.clone(),
            runtime_settings: project.runtime_settings.clone(),
            scenes: scenes.to_vec(),
        }
    }
}