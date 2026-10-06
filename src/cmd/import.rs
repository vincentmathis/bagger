use clap::{ArgAction, Parser};
use crossterm::style::Stylize;
use std::path::PathBuf;

use super::install;
use crate::Result;

/// Import apps from an export file (or stdin) and install them
#[derive(Debug, Parser)]
pub struct Args {
    /// Export file to read (defaults to stdin)
    file: Option<PathBuf>,
    /// Assume yes to all prompts and run non-interactively
    #[arg(short = 'y', long, action = ArgAction::SetTrue)]
    assume_yes: bool,
    /// Target architecture for all imported apps (overrides per-app export)
    #[arg(long)]
    arch: Option<String>,
}

pub fn execute(args: Args, session: &scoop_rs::Session) -> Result<()> {
    let input = match &args.file {
        Some(path) => std::fs::read_to_string(path)?,
        None => {
            let mut input = String::new();
            std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;
            input
        }
    };

    let apps: Vec<serde_json::Value> = serde_json::from_str(&input)?;
    if apps.is_empty() {
        println!("Nothing to import.");
        return Ok(());
    }

    let mut queries = vec![];
    let mut held = vec![];
    let mut skipped = vec![];

    for app in &apps {
        let name = app.get("name").and_then(|v| v.as_str()).unwrap_or("");
        if name.is_empty() {
            continue;
        }
        let bucket = app.get("bucket").and_then(|v| v.as_str()).unwrap_or("");

        // Isolated apps have no bucket manifest to reinstall from.
        if bucket == "__isolated__" {
            skipped.push(name.to_owned());
            continue;
        }

        // Per-app architecture from the export, unless globally overridden.
        let arch = match args.arch.as_deref() {
            Some(arch) => Some(arch.to_owned()),
            None => app
                .get("architecture")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_owned()),
        };

        let query = match bucket.is_empty() {
            true => name.to_owned(),
            false => format!("{bucket}/{name}"),
        };
        queries.push((arch, query));

        if app.get("held").and_then(|v| v.as_bool()).unwrap_or(false) {
            held.push(name.to_owned());
        }
    }

    if !skipped.is_empty() {
        println!(
            "{} {}",
            "Skipping isolated apps (no bucket manifest):".yellow(),
            skipped.join(", ")
        );
    }

    if queries.is_empty() {
        println!("Nothing installable to import.");
        return Ok(());
    }

    println!("Importing {} app(s).", queries.len());

    // Group by architecture: the resolution override is process-wide, so
    // each arch group installs in its own transaction.
    queries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut idx = 0;
    while idx < queries.len() {
        let arch = queries[idx].0.clone();
        let mut end = idx;
        while end < queries.len() && queries[end].0 == arch {
            end += 1;
        }
        let group: Vec<String> = queries[idx..end]
            .iter()
            .map(|(_, query)| query.clone())
            .collect();

        install::execute(
            install::Args::from_packages(group, args.assume_yes, arch),
            session,
        )?;
        idx = end;
    }
    // The resolution override is process-wide: never leak it past import.
    scoop_rs::arch::clear_override();

    // Restore held states recorded in the export.
    for name in &held {
        if let Err(e) = scoop_rs::operation::package_hold(session, name, true) {
            eprintln!("Could not re-hold '{}': {}", name, e);
        }
    }

    Ok(())
}
