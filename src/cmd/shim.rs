use clap::{Parser, Subcommand};
use crossterm::style::Stylize;
use std::path::PathBuf;

use crate::{util, Result};

fn human_size(bytes: u64) -> String {
    util::humansize(bytes, true)
}

/// Manage shims
#[derive(Debug, Parser)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List all shims
    #[clap(alias = "ls")]
    List {
        /// Filter shims by name (substring match)
        #[arg(required = false)]
        query: Option<String>,
    },
    /// Add a shim pointing to a target executable
    Add {
        /// The shim name
        shim: String,
        /// The target executable path
        target: String,
    },
    /// Remove a shim
    #[clap(alias = "rm")]
    Remove {
        /// The shim name (without extension)
        shim: String,
    },
}

pub fn execute(args: Args, session: &scoop_rs::Session) -> Result<()> {
    let config = session.config();
    let shims_dir = PathBuf::from(config.root_path()).join("shims");

    match args.command {
        Command::List { query } => {
            list_shims(&shims_dir, query.as_deref())?;
        }
        Command::Add { shim, target } => {
            add_shim(&shims_dir, &shim, &target)?;
        }
        Command::Remove { shim } => {
            remove_shim(&shims_dir, &shim)?;
        }
    }

    Ok(())
}

fn list_shims(shims_dir: &std::path::Path, query: Option<&str>) -> Result<()> {
    if !shims_dir.exists() {
        eprintln!("Shims directory not found: {}", shims_dir.display());
        return Ok(());
    }

    let mut shims: Vec<PathBuf> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(shims_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() || path.is_symlink() {
                let name = path.file_name().unwrap().to_string_lossy().to_string();

                // Skip shim files that are just package-specific variants or non-shim files
                if name.starts_with('.') || name.contains(".shim") {
                    continue;
                }

                // Only list known shim file types
                let ext = path.extension().and_then(|e| e.to_str());
                match ext {
                    Some("exe") | Some("bat") | Some("cmd") | Some("ps1") | Some("sh") => {}
                    None => {
                        // Files without extension may be shims (e.g., bash shims)
                        // Skip if it looks like a non-shim file
                        if name.contains('.') && !name.starts_with('.') {
                            continue;
                        }
                    }
                    _ => continue,
                }

                shims.push(path);
            }
        }
    }

    shims.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

    if let Some(q) = query {
        let q_lower = q.to_lowercase();
        shims.retain(|p| {
            p.file_stem()
                .and_then(|n| n.to_str())
                .map(|n| n.to_lowercase().contains(&q_lower))
                .unwrap_or(false)
        });
    }

    if shims.is_empty() {
        println!("No shims found.");
        return Ok(());
    }

    for shim in &shims {
        let name = shim.file_name().unwrap().to_string_lossy();
        let target = if shim.is_symlink() {
            std::fs::read_link(shim)
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "(unreadable symlink)".to_string())
        } else if shim.extension().is_some() {
            // Try to read as text to extract target path
            std::fs::read_to_string(shim)
                .map(|content| {
                    let line = content.lines().find(|l| l.contains('"'));
                    if let Some(line) = line {
                        if let Some(start) = line.find('"') {
                            if let Some(end) = line[start + 1..].find('"') {
                                return line[start + 1..start + 1 + end].to_string();
                            }
                        }
                    }
                    "(binary shim)".to_string()
                })
                .unwrap_or_else(|_| {
                    // Binary shim - show file size
                    match std::fs::metadata(shim) {
                        Ok(meta) => format!("binary ({})", human_size(meta.len())),
                        Err(_) => "(unknown)".to_string(),
                    }
                })
        } else {
            "(not a file shim)".to_string()
        };

        println!("{} {}", name.green(), target.cyan());
    }

    println!("\n{} shim(s)", shims.len());

    Ok(())
}

fn add_shim(shims_dir: &std::path::Path, shim: &str, target: &str) -> Result<()> {
    let target_path = PathBuf::from(target);

    if !target_path.exists() {
        eprintln!("Target executable not found: {}", target);
        return Ok(());
    }

    std::fs::create_dir_all(shims_dir)?;

    let shim_path = shims_dir.join(shim);

    #[cfg(windows)]
    {
        // On Windows, create a batch file shim
        let content = format!("@echo off\n\"{}\"\n", target);
        std::fs::write(&shim_path, content)?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let content = format!("#!/bin/sh\n\"{}\"\n", target);
        std::fs::write(&shim_path, content)?;
        let mut perms = std::fs::metadata(&shim_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&shim_path, perms)?;
    }

    println!("Created shim '{}' -> '{}'", shim.green(), target);

    Ok(())
}

fn remove_shim(shims_dir: &std::path::Path, shim: &str) -> Result<()> {
    // Try common shim file names
    let candidates: Vec<PathBuf> = vec![
        shims_dir.join(shim),
        shims_dir.join(format!("{}.exe", shim)),
        shims_dir.join(format!("{}.bat", shim)),
        shims_dir.join(format!("{}.cmd", shim)),
        shims_dir.join(format!("{}.ps1", shim)),
        shims_dir.join(format!("{}.sh", shim)),
    ];

    let mut removed = 0;
    for candidate in &candidates {
        if candidate.exists() {
            std::fs::remove_file(candidate)?;
            println!(
                "Removed: {}",
                candidate.file_name().unwrap().to_string_lossy().green()
            );
            removed += 1;
        }
    }

    if removed == 0 {
        eprintln!("No shim found for '{}'", shim);
    } else {
        println!("Removed {} shim file(s)", removed);
    }

    Ok(())
}
