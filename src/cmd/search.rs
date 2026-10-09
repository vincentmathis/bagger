use clap::{ArgAction, Parser};
use crossterm::style::Stylize;
use scoop_rs::{operation, QueryOption, Session};

use crate::Result;

/// Search available package(s)
///
/// Search available package(s) from synced buckets. Matching covers
/// package names and binaries by default (like upstream); use
/// --with-description to also search descriptions, --explicit for
/// literal instead of regex matching.
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The query string (regex supported by default)
    #[arg(required = true, action = ArgAction::Append)]
    query: Vec<String>,
    /// Turn regex off and use explicit matching
    #[arg(short = 'e', long, action = ArgAction::SetTrue)]
    explicit: bool,
    /// Also show all package binaries in results
    #[arg(short = 'B', long, action = ArgAction::SetTrue)]
    with_binary: bool,
    /// Search through package descriptions as well
    #[arg(short = 'D', long, action = ArgAction::SetTrue)]
    with_description: bool,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    let queries = args.query.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let mut options = vec![QueryOption::Binary];

    if args.with_description {
        options.push(QueryOption::Description);
    }

    if args.explicit {
        options.push(QueryOption::Explicit);
    }

    let packages = operation::package_query(session, queries, options, false)?;

    // Precompiled display matchers (query construction already validated
    // any regex, so build failures here fall back to literal matching).
    let display_matchers: Vec<Option<regex::Regex>> = args
        .query
        .iter()
        .map(|q| {
            (!args.explicit)
                .then(|| {
                    regex::RegexBuilder::new(q)
                        .case_insensitive(true)
                        .multi_line(true)
                        .build()
                        .ok()
                })
                .flatten()
        })
        .collect();
    let name_hit = |name: &str| {
        args.query.iter().enumerate().any(|(i, q)| {
            if args.explicit || display_matchers[i].is_none() {
                name.to_lowercase().contains(&q.to_lowercase())
            } else {
                display_matchers[i]
                    .as_ref()
                    .is_some_and(|re| re.is_match(name))
            }
        })
    };

    for pkg in &packages {
        let mut output = String::new();
        output.push_str(
            format!("{}/{} {}", pkg.name(), pkg.bucket().green(), pkg.version()).as_str(),
        );

        if pkg.is_strictly_installed() {
            let manifest_version = pkg.version();
            let installed_version = pkg.installed_version().unwrap();
            if manifest_version != installed_version {
                output.push_str(
                    format!(" [installed: {}]", installed_version)
                        .blue()
                        .to_string()
                        .as_str(),
                );
            } else {
                output.push_str(" [installed]".blue().to_string().as_str());
            }
        }

        if args.with_description {
            let description = pkg.description().unwrap_or("<no description>");
            output.push_str(format!("\n  {}", description).as_str());
        }

        if args.with_binary {
            let shims = match pkg.shims() {
                None => "<no shims>".to_owned(),
                Some(shims) => shims.join(","),
            };
            output.push_str(format!("\n  {}", shims).as_str());
        } else {
            // Show why this matched when the name didn't: the matched
            // binaries, like upstream's Binaries column.
            let mut matched = Vec::new();
            for q in &args.query {
                matched.extend(operation::matching_bin_names(
                    pkg.manifest(),
                    q,
                    args.explicit,
                ));
            }
            matched.sort();
            matched.dedup();
            if !matched.is_empty() && !name_hit(pkg.name()) {
                output.push_str(format!("\n  {}", matched.join(",")).as_str());
            }
        }

        println!("{}", output);
    }

    // Fall back to not-yet-added known buckets (upstream `search_remotes`).
    let mut remote: Vec<(String, String)> = Vec::new();
    if packages.is_empty() {
        remote = operation::search_remote_buckets(session, &args.query.join("|"))?;
        if !remote.is_empty() {
            println!("\nResults from other known buckets...");
            println!("(add them using 'bagger bucket add <bucket name>')");
            for (bucket, name) in &remote {
                println!("  {name}/{bucket}");
            }
        }
    }

    if packages.is_empty() && remote.is_empty() {
        eprintln!("No matches found.");
        std::process::exit(1);
    }

    Ok(())
}
