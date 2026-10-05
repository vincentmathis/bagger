use clap::Parser;
use crossterm::style::Stylize;
use std::path::PathBuf;

use crate::Result;

/// Create a new Scoop bucket or manifest template
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The name of the bucket to create
    name: String,

    /// Create a manifest template instead of a bucket
    #[arg(short = 'm', long, action = clap::ArgAction::SetTrue)]
    manifest: bool,

    /// Directory where to create the bucket (default: scoop root)
    #[arg(short = 'd', long)]
    directory: Option<String>,
}

pub fn execute(args: Args, session: &scoop_rs::Session) -> Result<()> {
    let config = session.config();
    let root_path = PathBuf::from(config.root_path());

    if args.manifest {
        create_manifest_template(&args.name, &root_path)?;
    } else {
        let dir = args
            .directory
            .map(PathBuf::from)
            .unwrap_or_else(|| root_path.join("buckets"));
        create_bucket(&args.name, &dir)?;
    }

    Ok(())
}

fn create_bucket(name: &str, dir: &std::path::Path) -> Result<()> {
    let bucket_dir = dir.join(name);

    if bucket_dir.exists() {
        eprintln!("Bucket directory already exists: {}", bucket_dir.display());
        return Ok(());
    }

    std::fs::create_dir_all(&bucket_dir)?;

    // Create a minimal bucket structure
    let gitignore = bucket_dir.join(".gitignore");
    std::fs::write(&gitignore, "*.json\n")?;

    let readme = bucket_dir.join("README.md");
    std::fs::write(&readme, format!("# {}\n\n", name))?;

    let manifest = bucket_dir.join(format!("{}.json", name));
    std::fs::write(&manifest, SAMPLE_MANIFEST)?;

    println!(
        "Created bucket '{}' at {}",
        name.green(),
        bucket_dir.display()
    );
    println!("\nTo add this bucket, run: bagger bucket add {}", name);

    Ok(())
}

fn create_manifest_template(name: &str, dir: &std::path::Path) -> Result<()> {
    let manifest_path = dir.join(format!("{}.json", name));

    if manifest_path.exists() {
        eprintln!("Manifest file already exists: {}", manifest_path.display());
        return Ok(());
    }

    std::fs::write(&manifest_path, SAMPLE_MANIFEST)?;

    println!(
        "Created manifest template at {}",
        manifest_path.display().to_string().green()
    );

    Ok(())
}

const SAMPLE_MANIFEST: &str = r#"{
  "version": "1.0.0",
  "description": "Example application",
  "homepage": "https://example.com",
  "license": "MIT",
  "url": "https://example.com/app-1.0.0.zip",
  "hash": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
  "bin": "app.exe",
  "shortcuts": [
    [
      "app.exe",
      "App"
    ]
  ]
}"#;
