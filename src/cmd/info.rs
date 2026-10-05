use clap::Parser;
use scoop_rs::{operation, Session};

use crate::Result;

/// Show package(s) basic information
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The query string (regex supported)
    query: String,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    let query = args.query;

    let queries = vec![query.as_str()];
    let options = vec![];
    let packages = operation::package_query(session, queries, options, false)?;
    let length = packages.len();
    match length {
        0 => eprintln!("Could not find package for query '{}'.", query),
        _ => {
            if length == 1 {
                println!("Found 1 package for query '{}':", query);
            } else {
                println!("Found {} package(s) for query '{}':", length, query);
            }

            for (idx, pkg) in packages.iter().enumerate() {
                // Ident
                println!("Identity: {}", pkg.ident());
                // Name
                println!("Name: {}", pkg.name());
                // Bucket
                println!("Bucket: {}", pkg.bucket());
                // Description
                println!(
                    "Description: {}",
                    pkg.description().unwrap_or("<no description>")
                );
                // Version
                println!("Version: {}", pkg.version());
                // Homepage
                println!("Homepage: {}", pkg.homepage());
                // License
                println!("License: {}", pkg.license());
                // Binaries
                println!(
                    "Shims: {}",
                    pkg.shims()
                        .map(|v| v.join(","))
                        .unwrap_or("<no shims>".to_owned())
                );
                // Dependencies (includes runtime deps)
                let deps = pkg.dependencies();
                println!(
                    "Dependencies: {}",
                    if deps.is_empty() {
                        "<none>".to_owned()
                    } else {
                        deps.join(", ")
                    }
                );
                // Explicit runtime deps
                if let Some(runtime) = pkg.manifest().runtime() {
                    if !runtime.is_empty() {
                        println!("Runtime: {}", runtime.join(", "));
                    }
                }

                if idx != (length - 1) {
                    println!();
                }
            }
        }
    }
    Ok(())
}
