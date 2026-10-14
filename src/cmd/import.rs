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

    let (buckets, apps) = parse_export(&input)?;
    if apps.is_empty() {
        println!("Nothing to import.");
        return Ok(());
    }

    // Upstream `scoop import` adds missing buckets first.
    if !buckets.is_empty() {
        let existing: Vec<String> = scoop_rs::operation::bucket_list(session)?
            .iter()
            .map(|b| b.name().to_owned())
            .collect();
        for bucket in &buckets {
            if existing
                .iter()
                .any(|e| e.eq_ignore_ascii_case(&bucket.name))
            {
                continue;
            }
            if bucket.source.is_empty() {
                eprintln!(
                    "warning: bucket '{}' has no source URL; skipping",
                    bucket.name
                );
                continue;
            }
            println!("Adding bucket {}...", bucket.name);
            if let Err(e) = scoop_rs::operation::bucket_add(session, &bucket.name, &bucket.source) {
                eprintln!("warning: could not add bucket '{}': {e}", bucket.name);
            }
        }
    }

    let mut queries = vec![];
    let mut held = vec![];
    let mut skipped = vec![];

    for app in &apps {
        let name = app.name.as_str();
        if name.is_empty() {
            continue;
        }
        let bucket = app.bucket.as_str();

        // Isolated apps have no bucket manifest to reinstall from.
        if bucket == "__isolated__" {
            skipped.push(name.to_owned());
            continue;
        }

        // Per-app architecture from the export, unless globally overridden.
        let arch = match args.arch.as_deref() {
            Some(arch) => Some(arch.to_owned()),
            None => app.arch.clone(),
        };

        let query = match bucket.is_empty() {
            true => name.to_owned(),
            false => format!("{bucket}/{name}"),
        };
        queries.push((arch, query));

        if app.held {
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

/// A bucket entry from an export file.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ExportBucket {
    name: String,
    source: String,
}

/// An app entry from an export file.
#[derive(Clone, Debug, PartialEq, Eq)]
struct ExportApp {
    name: String,
    bucket: String,
    arch: Option<String>,
    held: bool,
}

/// Read a string field under any of several key spellings (upstream
/// PascalCase or bagger lowercase).
fn str_field(obj: &serde_json::Map<String, serde_json::Value>, keys: &[&str]) -> String {
    keys.iter()
        .filter_map(|k| obj.get(*k)?.as_str())
        .next()
        .unwrap_or("")
        .to_owned()
}

/// Parse an export file in either shape: upstream's
/// `{"buckets": [...], "apps": [...]}` object or bagger's legacy bare
/// array. Unknown keys are ignored; entries without names are skipped.
fn parse_export(input: &str) -> Result<(Vec<ExportBucket>, Vec<ExportApp>)> {
    let value: serde_json::Value = serde_json::from_str(input)?;
    let (buckets_value, apps_value) = match &value {
        serde_json::Value::Object(map) => (
            map.get("buckets").or(map.get("Buckets")),
            map.get("apps")
                .or(map.get("Apps"))
                .cloned()
                .unwrap_or(serde_json::Value::Array(vec![])),
        ),
        serde_json::Value::Array(_) => (None, value.clone()),
        _ => {
            return Err(anyhow::anyhow!(
                "export file must contain an app list (array, or {{\"buckets\", \"apps\"}} object)"
            ))
        }
    };

    let mut buckets = Vec::new();
    if let Some(list) = buckets_value.and_then(|v| v.as_array()) {
        for entry in list {
            let Some(obj) = entry.as_object() else {
                continue;
            };
            let name = str_field(obj, &["name", "Name", "bucket"]);
            if name.is_empty() || name == "__isolated__" {
                continue;
            }
            buckets.push(ExportBucket {
                name,
                source: str_field(obj, &["source", "Source", "url", "URL"]),
            });
        }
    }

    let mut apps = Vec::new();
    let list = apps_value.as_array().ok_or_else(|| {
        anyhow::anyhow!(
            "export file must contain an app list (array, or {{\"buckets\", \"apps\"}} object)"
        )
    })?;
    for entry in list {
        let Some(obj) = entry.as_object() else {
            continue;
        };
        let name = str_field(obj, &["Name", "name"]);
        if name.is_empty() {
            continue;
        }
        let arch = str_field(obj, &["architecture", "Architecture"]);
        apps.push(ExportApp {
            name,
            bucket: str_field(obj, &["Source", "source", "bucket"]),
            arch: (!arch.is_empty()).then_some(arch),
            held: obj
                .get("held")
                .or(obj.get("Held"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
        });
    }

    Ok((buckets, apps))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Upstream `scoop export` shape (PascalCase object) parses, including
    /// the bucket list for auto-adding.
    #[test]
    fn parses_upstream_export_shape() {
        let input = r#"{
            "buckets": [{"Name": "extras", "Source": "https://github.com/ScoopInstaller/Extras"}],
            "apps": [
                {"Name": "bat", "Version": "0.26.1", "Source": "extras", "Updated": "2026-01-01", "Info": ""},
                {"Name": "", "Source": "main"}
            ]
        }"#;
        let (buckets, apps) = parse_export(input).unwrap();
        assert_eq!(
            buckets,
            vec![ExportBucket {
                name: "extras".to_owned(),
                source: "https://github.com/ScoopInstaller/Extras".to_owned(),
            }]
        );
        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].name, "bat");
        assert_eq!(apps[0].bucket, "extras");
        assert_eq!(apps[0].arch, None);
        assert!(!apps[0].held);
    }

    /// Bagger's own shape (bare array, lowercase + extensions) parses.
    #[test]
    fn parses_legacy_array_shape() {
        let input = r#"[
            {"name": "zstd", "bucket": "main", "version": "1.5.7", "architecture": "64bit", "held": true},
            {"name": "lonely", "bucket": "__isolated__", "version": "1.0", "architecture": "", "held": false}
        ]"#;
        let (buckets, apps) = parse_export(input).unwrap();
        assert!(buckets.is_empty());
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].arch, Some("64bit".to_owned()));
        assert!(apps[0].held);
        // The isolated marker survives parsing (the installer skips it).
        assert_eq!(apps[1].bucket, "__isolated__");
    }

    #[test]
    fn rejects_non_list_exports() {
        assert!(parse_export("{}").unwrap().1.is_empty());
        assert!(parse_export("42").is_err());
    }
}
