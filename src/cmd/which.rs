use clap::Parser;
use std::path::PathBuf;

use crate::Result;

/// Find which application owns a given executable (shim)
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The command name to look for
    command: String,
}

pub fn execute(args: Args, session: &scoop_rs::Session) -> Result<()> {
    let config = session.config();
    let shims_dir = PathBuf::from(config.root_path()).join("shims");

    if !shims_dir.exists() {
        eprintln!("Scoop shims directory not found: {}", shims_dir.display());
        return Ok(());
    }

    let command = &args.command;

    let candidates: Vec<PathBuf> = vec![
        shims_dir.join(command),
        shims_dir.join(format!("{}.exe", command)),
        shims_dir.join(format!("{}.cmd", command)),
        shims_dir.join(format!("{}.ps1", command)),
        shims_dir.join(format!("{}.bat", command)),
        shims_dir.join(format!("{}.sh", command)),
    ];

    for candidate in candidates {
        if candidate.exists() {
            let content = std::fs::read_to_string(&candidate).unwrap_or_default();
            let (app, target) = parse_shim(&content, &candidate);
            match (&app, &target) {
                (Some(app), Some(target)) => {
                    let display_target = if target.exists() {
                        target
                            .canonicalize()
                            .unwrap_or(target.clone())
                            .display()
                            .to_string()
                    } else {
                        target.display().to_string()
                    };
                    println!("{} -> {} ({})", command, display_target, app);
                }
                _ => {
                    let canonical = candidate.canonicalize().unwrap_or(candidate);
                    println!("{} -> {} (shim)", command, canonical.display());
                }
            }
            return Ok(());
        }
    }

    eprintln!("No shim found for command '{}'", command);
    Ok(())
}

/// Parse a shim file to extract the app name and target executable path.
///
/// Shim files look like:
/// ```batch
/// @echo off
/// "C:\scoop\apps\ninja\current\ninja.exe"
/// ```
fn parse_shim(content: &str, _shim_path: &PathBuf) -> (Option<String>, Option<PathBuf>) {
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("@echo off") || line.starts_with("#!") {
            continue;
        }

        // Extract the quoted path
        if let Some(start) = line.find('"') {
            if let Some(end) = line[start + 1..].find('"') {
                let target = &line[start + 1..start + 1 + end];
                let target_path = PathBuf::from(target);

                // Find app name from path structure: .../apps/<app>/...
                if let Some(parent) = target_path.parent() {
                    if let Some(grandparent) = parent.parent() {
                        let dir_name = grandparent.file_name().and_then(|n| n.to_str());
                        if dir_name == Some("apps") {
                            if let Some(app_name) = parent.file_name().and_then(|n| n.to_str()) {
                                return (Some(app_name.to_string()), Some(target_path));
                            }
                        }
                    }
                }

                return (None, Some(target_path));
            }
        }
    }

    (None, None)
}
