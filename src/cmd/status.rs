use clap::Parser;
use crossterm::style::Stylize;
use scoop_rs::{operation, Session};

use crate::Result;

/// Show status of installed apps: held, upgradable, running processes
#[derive(Debug, Parser)]
pub struct Args {}

pub fn execute(_args: Args, session: &Session) -> Result<()> {
    let packages = operation::package_query(session, vec![], vec![], true)?;

    if packages.is_empty() {
        println!("No apps installed.");
        return Ok(());
    }

    let mut any_held = false;
    let mut any_upgradable = false;
    let mut any_running = false;

    // Stale nightly builds compare Equal, so the version check below
    // misses them; they count as upgradable explicitly.
    let stale = operation::stale_nightlies(session).unwrap_or_default();
    let is_stale = |pkg: &scoop_rs::Package| stale.iter().any(|n| n == pkg.name());

    for pkg in &packages {
        if pkg.is_held() {
            any_held = true;
        }
        if pkg.upgradable_version().is_some() || is_stale(pkg) {
            any_upgradable = true;
        }
    }

    // Check for running processes
    let mut running_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for pkg in &packages {
        if let Ok(procs) = operation::running_processes(session, pkg) {
            if !procs.is_empty() {
                any_running = true;
                running_map.insert(pkg.name().to_owned(), procs);
            }
        }
    }

    println!("=== Installed apps: {} ===\n", packages.len());

    println!("Packages:");

    for pkg in &packages {
        let mut line = format!("  {}", pkg.name().green().bold());

        if pkg.is_held() {
            line.push_str(&format!(" [{}]", "held".magenta()));
        }

        if let Some(upgrade_ver) = pkg.upgradable_version() {
            line.push_str(&format!(" -> {} (upgradable)", upgrade_ver.blue()));
        } else if is_stale(pkg) {
            line.push_str(&format!(
                " -> {} (upgradable)",
                operation::nightly_stamp().blue()
            ));
        }

        println!("{}", line);

        if let Some(procs) = running_map.get(pkg.name()) {
            any_running = true;
            println!(
                "    running: {}",
                procs
                    .iter()
                    .map(|p| p.clone().yellow().to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }

    println!("\n=== Status Summary ===");

    if any_held {
        let held: Vec<_> = packages.iter().filter(|p| p.is_held()).collect();
        print!("Held apps ({}): ", held.len());
        for (i, p) in held.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("{}", p.name());
        }
        println!();
    } else {
        println!("Held apps: 0");
    }

    if any_upgradable {
        let upgradable: Vec<_> = packages
            .iter()
            .filter(|p| p.upgradable_version().is_some() || is_stale(p))
            .collect();
        print!("Upgradable apps ({}): ", upgradable.len());
        for (i, p) in upgradable.iter().enumerate() {
            if i > 0 {
                print!(", ");
            }
            print!("{}", p.name());
        }
        println!();
    } else {
        println!("Upgradable apps: 0");
    }

    if any_running {
        println!("Running processes: {} app(s)", running_map.len());
    } else {
        println!("Running processes: 0");
    }

    Ok(())
}
