use clap::{ArgAction, Parser};
use crossterm::style::Stylize;
use scoop_rs::{operation, QueryOption, Session};

use crate::Result;

/// List installed package(s)
#[derive(Debug, Parser)]
pub struct Args {
    /// The query string (regex supported by default)
    #[arg(action = ArgAction::Append)]
    query: Vec<String>,
    /// Turn regex off and use explicit matching
    #[arg(short = 'e', long, action = ArgAction::SetTrue)]
    explicit: bool,
    /// List upgradable package(s)
    #[arg(short = 'u', long, action = ArgAction::SetTrue)]
    upgradable: bool,
    /// List held package(s)
    #[arg(short = 'H', long, action = ArgAction::SetTrue)]
    held: bool,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    let queries = args.query.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let mut options = vec![];

    if args.explicit {
        options.push(QueryOption::Explicit);
    }

    if args.upgradable {
        options.push(QueryOption::Upgradable);
    }

    match operation::package_query(session, queries, options, true) {
        Err(e) => Err(e.into()),
        Ok(packages) => {
            // Stale nightly builds are filtered out by the Upgradable
            // query (they compare Equal); union them back in explicitly.
            let mut packages = packages;
            let stale = if args.upgradable {
                operation::stale_nightlies(session).unwrap_or_default()
            } else {
                Vec::new()
            };
            if !stale.is_empty() {
                let names: Vec<&str> = stale.iter().map(String::as_str).collect();
                if let Ok(extra) = operation::package_query(session, names, vec![], true) {
                    for pkg in extra {
                        if !packages.iter().any(|p| p.name() == pkg.name()) {
                            packages.push(pkg);
                        }
                    }
                    packages.sort_by_key(|p| p.name().to_owned());
                }
            }
            for pkg in packages {
                let mut output = String::new();
                output.push_str(
                    format!("{}/{} {}", pkg.name(), pkg.bucket().green(), pkg.version()).as_str(),
                );

                let held = pkg.is_held();
                if args.held && !held {
                    continue;
                }

                let upgradable = pkg.upgradable_version();
                if args.upgradable {
                    if let Some(version) = upgradable {
                        output.push_str(format!(" -> {}", version.blue()).as_str());
                    } else if stale.iter().any(|n| n == pkg.name()) {
                        output.push_str(
                            format!(" -> {}", operation::nightly_stamp().blue()).as_str(),
                        );
                    }
                }

                if held {
                    output.push_str(format!(" [{}]", "held".magenta()).as_str());
                }

                println!("{}", output);
            }
            Ok(())
        }
    }
}
