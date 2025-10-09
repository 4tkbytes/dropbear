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

    // fuck you windows :(
    #[cfg(target_os = "windows")]
    {
        println!("cargo:rustc-link-arg=/FORCE:MULTIPLE");
        println!("cargo:rustc-link-arg=/NODEFAULTLIB:libcmt.lib");
    }

    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
