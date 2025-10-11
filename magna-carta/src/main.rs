use clap::{Parser, ValueEnum};
use magna_carta::{KotlinProcessor, ScriptManifest};
use magna_carta::generator::{Generator, jvm::KotlinJVMGenerator, native::KotlinNativeGenerator};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "magna-carta-cli")]
#[command(about = "Generate script manifests for Kotlin projects")]
struct Cli {
    #[arg(short, long, help = "Input directory containing Kotlin source files")]
    input: PathBuf,

    #[arg(short, long, help = "Output directory for generated files")]
    output: PathBuf,

    #[arg(short, long, help = "Target platform")]
    target: Target,
}

#[derive(ValueEnum, Clone)]
enum Target {
    Jvm,
    Native,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut processor = KotlinProcessor::new()?;
    let mut manifest = ScriptManifest::new();

    if !cli.input.exists() {
        return Err(anyhow::anyhow!("Input directory does not exist: {:?}", cli.input));
    }

    visit_kotlin_files(&cli.input, &mut processor, &mut manifest)?;

    fs::create_dir_all(&cli.output)?;

    match cli.target {
        Target::Jvm => {
            let generator = KotlinJVMGenerator;
            let output_path = cli.output.join("RunnableRegistry.kt");
            generator.write_to_file(&manifest, &output_path)?;
            println!("Generated JVM manifest at: {}", output_path.display());
        }
        Target::Native => {
            let generator = KotlinNativeGenerator;
            let output_path = cli.output.join("ScriptManifest.kt");
            generator.write_to_file(&manifest, &output_path)?;
            println!("Generated Native manifest at: {}", output_path.display());
        }
    }

    println!("Found {} script classes", manifest.items().len());
    Ok(())
}

fn visit_kotlin_files(
    dir: &PathBuf,
    processor: &mut KotlinProcessor,
    manifest: &mut ScriptManifest,
) -> anyhow::Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            if path.is_dir() {
                visit_kotlin_files(&path, processor, manifest)?;
            } else if path.extension() == Some(std::ffi::OsStr::new("kt")) {
                let source_code = fs::read_to_string(&path)?;
                
                if let Some(item) = processor.process_file(&source_code, path.clone())? {
                    manifest.add_item(item);
                }
            }
        }
    }
    
    Ok(())
}