use clap::Parser;
use scoop_rs::{operation, Session};

use crate::Result;

/// Export list of installed apps to stdout (JSON)
#[derive(Debug, Parser)]
pub struct Args {}

pub fn execute(_args: Args, session: &Session) -> Result<()> {
    let packages = operation::package_query(session, vec![], vec![], true)?;

    let output: Vec<_> = packages
        .iter()
        .map(|pkg| {
            serde_json::json!({
                "name": pkg.name(),
                "bucket": pkg.bucket(),
                "version": pkg.installed_version().unwrap_or(pkg.version()),
                "architecture": pkg.installed_arch().unwrap_or(""),
                "held": pkg.is_held(),
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
