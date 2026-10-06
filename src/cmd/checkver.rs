use clap::{ArgAction, Parser};
use crossterm::style::Stylize;
use scoop_rs::{operation, QueryOption, Session};

use crate::Result;

/// Check for app updates without modifying anything
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package name (or "all" for all installed apps)
    #[arg(required = true)]
    package: String,

    /// Show all versions, not just the latest
    #[arg(short = 'a', long, action = ArgAction::SetTrue)]
    all: bool,

    /// Force check even if checkver is not defined
    #[arg(short = 'f', long, action = ArgAction::SetTrue)]
    force: bool,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    if args.package == "all" {
        // Check all installed packages
        let packages = operation::package_query(session, vec![], vec![], true)?;

        let mut upgradable: Vec<_> = Vec::new();

        for pkg in &packages {
            if pkg.is_held() && !args.force {
                continue;
            }

            let result = operation::checkver(session, pkg)?;

            if let Some(latest) = &result.latest_version {
                if latest.as_str() != result.current_version.as_deref().unwrap_or("") {
                    if args.all {
                        println!(
                            "  {}: {} -> {}",
                            pkg.name().green(),
                            result.current_version.as_deref().unwrap_or("?"),
                            latest.clone().yellow()
                        );
                    } else {
                        upgradable.push(pkg);
                    }
                }
            }
        }

        if !args.all {
            if upgradable.is_empty() {
                println!("All apps are up to date.");
            } else {
                println!("{}", "Apps with updates:".green().bold());
                for pkg in &upgradable {
                    let result = operation::checkver(session, pkg)?;
                    println!(
                        "  {}: {} -> {}",
                        pkg.name().green(),
                        result.current_version.as_deref().unwrap_or("?"),
                        result
                            .latest_version
                            .unwrap_or("unknown".to_string())
                            .yellow()
                    );
                }
            }
        }

        return Ok(());
    }

    // Check a single package (from all buckets, not just installed)
    let query = &args.package;
    let options = vec![QueryOption::Explicit];
    let result = operation::package_query(session, vec![query.as_str()], options, false)?;

    if result.is_empty() {
        eprintln!("Package '{}' not found in any bucket.", query);
        return Ok(());
    }

    let pkg = &result[0];

    if pkg.manifest().effective_checkver().is_none() && !args.force {
        eprintln!("Package '{}' has no checkver definition.", pkg.name());
        eprintln!("Use --force to check anyway using the homepage.");
        return Ok(());
    }

    print!("Checking {}... ", pkg.name().green());

    let check_result = operation::checkver(session, pkg)?;

    if let Some(latest) = &check_result.latest_version {
        let current = check_result.current_version.as_deref().unwrap_or("unknown");

        if latest == current {
            println!("{}", "up to date".green());
        } else {
            println!("{} -> {}", current, format!("{} (latest)", latest).yellow());
        }
    } else {
        println!("{}", "could not determine latest version".yellow());
    }

    Ok(())
}
