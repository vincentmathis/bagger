use clap::Parser;
use crossterm::style::Stylize;
use scoop_rs::operation;
use std::path::PathBuf;

use crate::Result;

/// Scan downloaded app archives with VirusTotal (requires API key)
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package name to scan
    package: String,

    /// VirusTotal API key (or set VIRUSTOTAL_API_KEY env var)
    #[arg(short = 'k', long)]
    api_key: Option<String>,
}

pub fn execute(args: Args, session: &scoop_rs::Session) -> Result<()> {
    let api_key = args
        .api_key
        .or_else(|| std::env::var("VIRUSTOTAL_API_KEY").ok());

    let api_key = match api_key {
        Some(k) if !k.is_empty() => k,
        _ => {
            eprintln!(
                "VirusTotal API key required. Set VIRUSTOTAL_API_KEY env var or use --api-key."
            );
            return Ok(());
        }
    };

    let config = session.config();
    let cache_dir = PathBuf::from(config.root_path()).join("cache");
    drop(config);

    if !cache_dir.exists() {
        eprintln!("Cache directory not found: {}", cache_dir.display());
        return Ok(());
    }

    // Find cached files matching the package name
    let mut matching_files: Vec<PathBuf> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&cache_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();

            // Cache files follow pattern: app#version#hash.ext
            if path.is_file()
                && (name
                    .to_lowercase()
                    .starts_with(&format!("{}-", args.package.to_lowercase()))
                    || name.contains(&args.package))
            {
                matching_files.push(path);
            }
        }
    }

    if matching_files.is_empty() {
        eprintln!("No cached files found for package '{}'", args.package);
        return Ok(());
    }

    println!("{}", "VirusTotal scan".bold());
    println!("Package: {}", args.package.green());
    println!("Files to scan: {}", matching_files.len());
    println!();

    let mut flagged = 0usize;

    for file in &matching_files {
        let filename = file.file_name().unwrap().to_string_lossy().to_string();
        print!("Scanning {} ... ", filename.green());

        match operation::virustotal_file_report(session, file, &api_key) {
            Ok(report) => {
                if !report.found {
                    println!("{}", "no existing report (not yet scanned)".yellow());
                    println!("  sha256: {}", report.sha256.dark_grey());
                    println!("  Upload the file at https://www.virustotal.com/gui/home/upload");
                } else if report.is_flagged() {
                    flagged += 1;
                    println!(
                        "{}",
                        format!(
                            "FLAGGED (malicious: {}, suspicious: {})",
                            report.malicious, report.suspicious
                        )
                        .red()
                        .bold()
                    );
                    println!("  sha256: {}", report.sha256.dark_grey());
                    println!(
                        "  harmless: {}, undetected: {}",
                        report.harmless, report.undetected
                    );
                } else {
                    println!(
                        "{}",
                        format!(
                            "clean (harmless: {}, undetected: {})",
                            report.harmless, report.undetected
                        )
                        .green()
                    );
                    println!("  sha256: {}", report.sha256.dark_grey());
                }
            }
            Err(e) => {
                println!("{}", format!("lookup failed: {e}").yellow());
            }
        }
    }

    println!();
    if flagged > 0 {
        println!(
            "{}",
            format!("{flagged} file(s) flagged by VirusTotal engines.")
                .red()
                .bold()
        );
    } else {
        println!("{}", "Done. No flags from known reports.".green());
    }

    Ok(())
}
