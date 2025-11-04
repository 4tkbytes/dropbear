use crate::states::SceneConfig;
use std::collections::HashMap;
// #[derive(bincode::Decode, bincode::Encode, serde::Serialize, serde::Deserialize, Debug)]
// pub struct RuntimeData {
//     #[bincode(with_serde)]
//     pub project_config: ProjectConfig,
//     #[bincode(with_serde)]
//     pub source_config: SourceConfig,
//     #[bincode(with_serde)]
//     pub scene_data: Vec<SceneConfig>,
//     #[bincode(with_serde)]
//     pub scripts: HashMap<String, String>, // name, script_content
// }

#[derive(bincode::Decode, bincode::Encode, serde::Serialize, serde::Deserialize, Debug)]
pub struct RuntimeProjectConfig {
    #[bincode(with_serde)]
    pub project_name: String,
    #[bincode(with_serde)]
    pub scene_map: RuntimeSceneIndex,
}

#[derive(bincode::Decode, bincode::Encode, serde::Serialize, serde::Deserialize, Debug)]
pub struct RuntimeSceneIndex {
    #[bincode(with_serde)]
    pub scene_index: HashMap<String, SceneConfig>,
}
