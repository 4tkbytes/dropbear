// use std::path::PathBuf;
// use std::sync::LazyLock;

// pub static RESOURCES_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
//     std::path::Path::new(env!("OUT_DIR")).join("resources")
// });

// pub async fn load_binary(file_name: &str) -> anyhow::Result<(PathBuf, Vec<u8>)> {
//     let path = std::path::Path::new(env!("OUT_DIR"))
//             .join("resources")
//             .join(file_name);
//     let data = {
//         log::info!("Loading binary file from: {:?}", path);
//         std::fs::read(&path)?
//     };

//     Ok((path, data))
// }

// pub async fn load_string(file_name: &str) -> anyhow::Result<(PathBuf, String)> {
//     let path = std::path::Path::new(env!("OUT_DIR"))
//             .join("resources")
//             .join(file_name);
//     let txt = {
//         log::info!("Loading string file from: {:?}", path);
//         std::fs::read_to_string(&path)?
//     };

//     Ok((path, txt))
// }
