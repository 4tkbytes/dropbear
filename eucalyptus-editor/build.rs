use std::fs::{self, File};
use std::io::Cursor;
use std::process::Command;

fn main() -> anyhow::Result<()> {
    let output = Command::new("git")
        .args(["rev-parse", "--short=6", "HEAD"])
        .output()
        .expect("Failed to execute git");

    let git_hash = String::from_utf8(output.stdout).expect("Invalid UTF-8 in git output");
    let git_hash = git_hash.trim();

    println!("cargo:rustc-env=GIT_HASH={}", git_hash);

    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs/heads");

    // todo: move this into the "setup" process
    let repo_zip_url = "https://github.com/4tkbytes/dropbear/archive/refs/heads/main.zip";
    let response = reqwest::blocking::get(repo_zip_url)
        .map_err(|e| anyhow::anyhow!("Failed to download repo zip: {}", e))?
        .bytes()
        .map_err(|e| anyhow::anyhow!("Failed to read zip bytes: {}", e))?;

    let reader = Cursor::new(response);
    let mut zip = zip::ZipArchive::new(reader)
        .map_err(|e| anyhow::anyhow!("Failed to read zip archive: {}", e))?;

    let app_info = app_dirs2::AppInfo {
        name: "Eucalyptus",
        author: "4tkbytes",
    };
    let app_data_dir = app_dirs2::app_root(app_dirs2::AppDataType::UserData, &app_info)
        .map_err(|e| anyhow::anyhow!("Could not determine app data directory: {}", e))?;

    fs::create_dir_all(&app_data_dir)
        .map_err(|e| anyhow::anyhow!("Failed to create app data directory: {}", e))?;

    let resource_prefix = "dropbear-main/resources/";
    let mut found_resource = false;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i).unwrap();
        let name = file.name();

        if name.starts_with(resource_prefix) && !name.ends_with('/') {
            found_resource = true;
            let rel_path = &name[resource_prefix.len()..];
            let rel_path = rel_path.strip_prefix('/').unwrap_or(rel_path);
            let dest_path = app_data_dir.join(rel_path);

            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| anyhow::anyhow!("Failed to create parent directory: {}", e))?;
            }

            println!("Copying {} to {:?}", name, dest_path);

            let mut outfile = File::create(&dest_path)
                .map_err(|e| anyhow::anyhow!("Failed to create file: {}", e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| anyhow::anyhow!("Failed to copy file: {}", e))?;
        }
    }

    if !found_resource {
        return Err(anyhow::anyhow!(
            "No resources folder found in the github repository [4tkbytes/dropbear] :("
        ));
    }

    // fuck you windows :(
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-arg=/FORCE:MULTIPLE");
        println!("cargo:rustc-link-arg=/NODEFAULTLIB:libcmt.lib");
    }

    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
