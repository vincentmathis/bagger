use clap::Parser;
use scoop_rs::{operation, QueryOption, Session};
use std::path::PathBuf;

use crate::Result;

/// Get the installation path of a package
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package name
    package: String,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    let query = args.package;

    let queries = vec![query.as_str()];
    let options = vec![QueryOption::Explicit];
    let result = operation::package_query(session, queries, options, true)?;

    match result.len() {
        0 => {
            eprintln!("Package not installed: {}", query);
            Ok(())
        }
        1 => {
            let package = &result[0];
            let config = session.config();
            let apps_dir = PathBuf::from(config.root_path()).join("apps");

            let install_path = if config.no_junction() {
                apps_dir
                    .join(package.name())
                    .join(package.installed_version().unwrap_or(package.version()))
            } else {
                apps_dir.join(package.name()).join("current")
            };

            println!("{}", install_path.display());
            Ok(())
        }
        _ => {
            eprintln!("Found multiple installed packages named '{}'", query);
            Ok(())
        }
    }
}
