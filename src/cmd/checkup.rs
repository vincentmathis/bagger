use clap::Parser;
use crossterm::style::Stylize;
use scoop_rs::{operation, Session};

use crate::Result;

/// Check for updates and report app status
#[derive(Debug, Parser)]
pub struct Args {}

pub fn execute(_args: Args, session: &Session) -> Result<()> {
    println!("Checking for app updates...");

    let installed = operation::package_query(session, vec![], vec![], true)?;

    let mut upgradable: Vec<_> = Vec::new();
    let mut held: Vec<_> = Vec::new();
    let mut running: Vec<_> = Vec::new();

    for pkg in &installed {
        if pkg.is_held() {
            held.push(pkg.name().to_owned());
            continue;
        }

        if pkg.upgradable_version().is_some() {
            upgradable.push(pkg);
        }

        if let Ok(procs) = operation::running_processes(session, pkg) {
            if !procs.is_empty() {
                running.push((pkg.name().to_owned(), procs));
            }
        }
    }

    println!("Found {} installed app(s).\n", installed.len());

    // Running processes warning
    if !running.is_empty() {
        println!(
            "{}",
            "WARNING: The following apps have running processes:"
                .yellow()
                .bold()
        );
        for (name, procs) in &running {
            println!("  {} ({})", name.clone().yellow(), procs.join(", "));
        }
        println!();
    }

    // Held apps
    if !held.is_empty() {
        println!("{}", "Held apps:".cyan());
        for name in &held {
            println!("  {}", name.clone().magenta());
        }
        println!();
    }

    // Upgradable apps
    if !upgradable.is_empty() {
        println!("{}", "Apps with updates:".green().bold());
        for pkg in &upgradable {
            let new_ver = pkg.upgradable_version().unwrap_or("");
            println!(
                "  {} {} -> {}",
                pkg.name().green(),
                pkg.version(),
                new_ver.blue()
            );
        }
        println!("\nRun '{}' to update them.", "bagger upgrade".cyan());
    } else {
        println!("{}", "All apps are up to date.".green());
    }

    // Check for missing executables (shims pointing to non-existent targets)
    let config = session.config();
    let shims_dir = std::path::Path::new(config.root_path()).join("shims");
    let mut broken_shims: Vec<String> = Vec::new();

    if shims_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&shims_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let name = path.file_name().unwrap().to_string_lossy().to_string();
                if name.starts_with('.') {
                    continue;
                }
                // Only assess recognized shim files: bare shims have no
                // extension; anything else must be a shim flavor. Stray
                // files (e.g. a `.txt` dropped in the directory) are not
                // shims and would only produce nonsense targets.
                let is_shim = match path.extension().and_then(|e| e.to_str()) {
                    None => !name.contains('.'),
                    Some(ext) => matches!(
                        ext.to_ascii_lowercase().as_str(),
                        "exe" | "cmd" | "ps1" | "bat" | "sh"
                    ),
                };
                if !is_shim {
                    continue;
                }
                match scoop_rs::shim_target_of(&path) {
                    Some(target) if !target.exists() => {
                        broken_shims.push(format!("{} (missing target {})", name, target.display()))
                    }
                    _ => {}
                }
            }
        }
    }

    if !broken_shims.is_empty() {
        println!("\n{}", "WARNING: Broken shims found:".yellow().bold());
        for shim in &broken_shims {
            println!("  {}", shim.clone().yellow());
        }
    }

    Ok(())
}
