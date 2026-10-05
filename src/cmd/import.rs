use clap::Parser;
use serde_json;
use std::collections::HashMap;

use crate::Result;
use crossterm::style::Stylize;

/// Import list of apps from stdin (JSON)
#[derive(Debug, Parser)]
pub struct Args {}

pub fn execute(_args: Args, _session: &scoop_rs::Session) -> Result<()> {
    let mut input = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut input)?;

    let apps: Vec<HashMap<String, String>> = serde_json::from_str(&input)?;

    println!("Imported {} app(s) from stdin.", apps.len());
    println!("\nApps to install:");
    for app in &apps {
        let name = app.get("name").map(|s| s.as_str()).unwrap_or("unknown");
        let bucket = app.get("bucket").map(|s| s.as_str()).unwrap_or("main");
        let version = app.get("version").map(|s| s.as_str()).unwrap_or("");

        if version.is_empty() {
            println!("  {}{}", name, format!(" ({})", bucket).dark_cyan());
        } else {
            println!("  {} {} ({})", name, version, bucket);
        }
    }

    println!("\nTo install, run: bagger install <app>");
    Ok(())
}
