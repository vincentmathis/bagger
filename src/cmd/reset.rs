use clap::Parser;
use crossterm::style::Stylize;
use scoop_rs::{operation, Session};
use std::path::PathBuf;

use crate::Result;

/// Reset an installed package to a specific version or re-extract it
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package name
    package: String,
    /// Specific version to reset to (default: current version)
    version: Option<String>,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    let config = session.config();

    // Query installed packages
    let packages = operation::package_query(session, vec![&args.package], vec![], true)?;

    if packages.is_empty() {
        eprintln!("Package '{}' is not installed.", args.package);
        return Ok(());
    }

    let pkg = &packages[0];
    let apps_dir = PathBuf::from(config.root_path()).join("apps");
    let app_dir = apps_dir.join(&args.package);

    // List available versions (directories under app_dir, excluding 'current')
    let mut versions: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&app_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name != "current" && entry.path().is_dir() {
                versions.push(name);
            }
        }
    }
    versions.sort();

    if versions.is_empty() {
        eprintln!("No versions found for package '{}'", args.package);
        return Ok(());
    }

    // Determine target version
    let target_version = if let Some(v) = &args.version {
        if !versions.contains(v) {
            eprintln!(
                "Version '{}' not found for '{}. Available: {}",
                v,
                args.package,
                versions.join(", ")
            );
            return Ok(());
        }
        v.clone()
    } else {
        // Default: re-extract current version from cache
        let current_version = pkg.installed_version().unwrap_or(pkg.version());
        current_version.to_string()
    };

    let version_dir = app_dir.join(&target_version);

    if !version_dir.exists() {
        eprintln!("Version directory not found: {}", version_dir.display());
        return Ok(());
    }

    println!(
        "Resetting '{}' to version '{}'",
        args.package, target_version
    );

    if config.no_junction() {
        // No junction mode: apps use version directories directly
        println!(
            "Reset complete. App directory: {}",
            version_dir.display().to_string().green()
        );
    } else {
        // Junction mode: point 'current' symlink to the version directory
        let current_link = app_dir.join("current");

        // Remove the existing 'current' link. This must be junction-aware:
        // `remove_file` fails on directory links on Windows, which then
        // breaks re-creation with "already exists" (os error 183).
        if current_link.symlink_metadata().is_ok() {
            let is_dir = current_link
                .metadata()
                .map(|m| m.file_type().is_dir())
                .unwrap_or(false);
            let _ = match is_dir {
                true => std::fs::remove_dir(&current_link),
                false => std::fs::remove_file(&current_link),
            };
        }

        // Create new symlink
        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(&version_dir, &current_link)?;

        println!(
            "Reset complete. 'current' now points to: {}",
            version_dir.display().to_string().green()
        );
    }

    // Show available versions
    println!(
        "\nAvailable versions: {}",
        versions
            .iter()
            .map(|v| {
                if v == &target_version {
                    format!("{}", v.clone().green().bold())
                } else {
                    v.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(())
}
