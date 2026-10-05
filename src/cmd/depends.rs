use clap::Parser;
use crossterm::style::Stylize;
use scoop_rs::{operation, QueryOption, Session};

use crate::Result;

/// Show dependencies for a package
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package name
    package: String,
    /// Show reverse dependencies (who depends on this package)
    #[arg(short = 'r', long, action = clap::ArgAction::SetTrue)]
    reverse: bool,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    let query = &args.package;

    let options = vec![QueryOption::Explicit];
    let result = operation::package_query(session, vec![query.as_str()], options, false)?;

    if result.is_empty() {
        eprintln!("Package '{}' not found in any bucket.", query);
        return Ok(());
    }

    let pkg = &result[0];

    // Query installed packages for reverse dependency checking
    let installed = operation::package_query(session, vec![], vec![], true)?;

    if !args.reverse {
        let deps = pkg.dependencies();

        if deps.is_empty() {
            println!("{} has no dependencies.", pkg.name().green());
        } else {
            println!("Dependencies for {}:", pkg.name().green().bold());
            for dep in &deps {
                println!("  - {}", dep);
            }
        }
    } else {
        println!(
            "Reverse dependencies (packages that depend on {}):",
            pkg.name().green().bold()
        );

        let mut found = false;
        for ipkg in &installed {
            let deps = ipkg.dependencies();
            let dep_names: Vec<String> = deps
                .iter()
                .map(|d| {
                    let clean = d.split('/').next_back().unwrap_or(d);
                    clean.to_string()
                })
                .collect();

            if dep_names.iter().any(|dn| dn == &args.package) {
                println!("  - {}", ipkg.name());
                found = true;
            }
        }

        if !found {
            println!("  (none)");
        }
    }

    Ok(())
}
