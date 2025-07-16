use anyhow::Result;
use std::env;

use fs_extra::{copy_items, dir::CopyOptions};

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=resources/**/*");

    let out_dir = env::var("OUT_DIR")?;
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;
    let mut paths_to_copy = Vec::new();
    paths_to_copy.push("resources/");
    copy_items(&paths_to_copy, out_dir, &copy_options)?;

    Ok(())
}