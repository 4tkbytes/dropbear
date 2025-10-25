use clap::{Parser, ValueEnum};
use magna_carta::generator::{Generator, jvm::KotlinJVMGenerator, native::KotlinNativeGenerator};
use magna_carta::{KotlinProcessor, ScriptManifest};
use std::fs;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "magna-carta-cli")]
#[command(about = "Generate script manifests for Kotlin projects")]
struct Cli {
    #[arg(short, long, help = "Input directory containing Kotlin source files")]
    input: PathBuf,

    #[arg(
        short,
        long,
        help = "Output directory for generated files (ignored if --stdout is used)"
    )]
    output: Option<PathBuf>,

    #[arg(short, long, help = "Target platform")]
    target: Target,

    #[arg(
        long,
        help = "Print generated manifest to stdout instead of writing to file"
    )]
    stdout: bool,

    #[arg(long, help = "Print manifest raw")]
    raw: bool,
}

#[derive(ValueEnum, Clone, Debug)]
enum Target {
    Jvm,
    Native,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if !cli.raw || (cli.stdout && cli.output.is_some()) {
        return Err(anyhow::anyhow!(
            "No output given. --stdout, --output <target> or --raw must be used."
        ));
    }

    let mut processor = KotlinProcessor::new()?;
    let mut manifest = ScriptManifest::new();

    if !cli.input.exists() {
        return Err(anyhow::anyhow!(
            "Input directory does not exist: {:?}",
            cli.input
        ));
    }

    visit_kotlin_files(&cli.input, &mut processor, &mut manifest)?;

    let generated_content = match cli.target {
        Target::Jvm => {
            let generator = KotlinJVMGenerator;
            generator.generate(&manifest)?
        }
        Target::Native => {
            let generator = KotlinNativeGenerator;
            generator.generate(&manifest)?
        }
    };

    if cli.raw {
        println!("{:#?}", manifest);
    }

    if cli.stdout {
        print!("{}", generated_content);
    } else if cli.raw && !(cli.stdout || cli.output.is_some()) {
        return Ok(());
    } else {
        let output_dir = cli.output.unwrap();
        fs::create_dir_all(&output_dir)?;

        let filename = match cli.target {
            Target::Jvm => "RunnableRegistry.kt",
            Target::Native => "ScriptManifest.kt",
        };
        let output_path = output_dir.join(filename);
        fs::write(&output_path, generated_content)?;
        println!(
            "Generated {:?} manifest at: {}",
            cli.target,
            output_path.display()
        );
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
