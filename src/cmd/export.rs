use clap::Parser;
use scoop_rs::{operation, Session};

use crate::Result;

/// Export list of installed apps to stdout (JSON)
#[derive(Debug, Parser)]
pub struct Args {}

pub fn execute(_args: Args, session: &Session) -> Result<()> {
    let packages = operation::package_query(session, vec![], vec![], true)?;

    // Upstream export shape (`{"buckets": [...], "apps": [...]}` with
    // PascalCase keys) so `scoop import` accepts our output directly.
    // `architecture`/`held` are bagger extensions `scoop import` ignores.
    let buckets: Vec<_> = operation::bucket_list(session)?
        .iter()
        .map(|b| {
            serde_json::json!({
                "Name": b.name(),
                "Source": b.remote_url().unwrap_or(""),
            })
        })
        .collect();

    let apps: Vec<_> = packages
        .iter()
        .map(|pkg| {
            serde_json::json!({
                "Name": pkg.name(),
                "Version": pkg.installed_version().unwrap_or(pkg.version()),
                "Source": pkg.bucket(),
                "architecture": pkg.installed_arch().unwrap_or(""),
                "held": pkg.is_held(),
            })
        })
        .collect();

    let output = serde_json::json!({ "buckets": buckets, "apps": apps });
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}
