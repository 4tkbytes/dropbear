use gleek::build::{GleamBindingsConfig, generate_gleam_bindings};

fn main() {
    let config = GleamBindingsConfig::default();
    
    if let Err(e) = generate_gleam_bindings(config) {
        eprintln!("Failed to generate Gleam bindings: {}", e);
        std::process::exit(1);
    }
}