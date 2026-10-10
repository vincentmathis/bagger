use clap::Parser;
use crossterm::style::Stylize;
use scoop_rs::{operation, Session};
use std::path::PathBuf;

use crate::Result;

/// Reset an installed package to a specific version or re-extract it
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package name
    package: String,
    /// Specific version to reset to (default: current version)
    version: Option<String>,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    let config = session.config();

    // Query installed packages
    let packages = operation::package_query(session, vec![&args.package], vec![], true)?;

    if packages.is_empty() {
        eprintln!("Package '{}' is not installed.", args.package);
        return Ok(());
    }

    let pkg = &packages[0];
    let apps_dir = PathBuf::from(config.root_path()).join("apps");
    let app_dir = apps_dir.join(&args.package);

    // List available versions (directories under app_dir, excluding 'current')
    let mut versions: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&app_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name != "current" && entry.path().is_dir() {
                versions.push(name);
            }
        }
    }
    versions.sort();

    if versions.is_empty() {
        eprintln!("No versions found for package '{}'", args.package);
        return Ok(());
    }

    // Determine target version
    let target_version = if let Some(v) = &args.version {
        if !versions.contains(v) {
            eprintln!(
                "Version '{}' not found for '{}. Available: {}",
                v,
                args.package,
                versions.join(", ")
            );
            return Ok(());
        }
        v.clone()
    } else {
        // Default: re-extract current version from cache
        let current_version = pkg.installed_version().unwrap_or(pkg.version());
        current_version.to_string()
    };

    let version_dir = app_dir.join(&target_version);

    if !version_dir.exists() {
        eprintln!("Version directory not found: {}", version_dir.display());
        return Ok(());
    }

    println!(
        "Resetting '{}' to version '{}'",
        args.package, target_version
    );

    if config.no_junction() {
        // No junction mode: apps use version directories directly
        println!(
            "Reset complete. App directory: {}",
            version_dir.display().to_string().green()
        );
    } else {
        // Junction mode: point 'current' symlink to the version directory.
        // Remove through the hardened helper (clears the readonly flag
        // junctions carry, handles junctions vs symlinks) and check the
        // result: a failed removal followed by creation is exactly the
        // "already exists" (os error 183) failure. Upstream does
        // `attrib -R` + remove for the same reason.
        let current_link = app_dir.join("current");

        scoop_rs::remove_symlink(&current_link)?;
        scoop_rs::symlink_dir(&version_dir, &current_link)?;

        println!(
            "Reset complete. 'current' now points to: {}",
            version_dir.display().to_string().green()
        );
    }

    // Show available versions
    println!(
        "\nAvailable versions: {}",
        versions
            .iter()
            .map(|v| {
                if v == &target_version {
                    format!("{}", v.clone().green().bold())
                } else {
                    v.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Resetting onto a readonly `current` junction must not fail with
    /// os error 183: removal clears the flag (upstream marks junctions
    /// `+R`) instead of silently skipping it.
    #[test]
    #[cfg(windows)]
    fn reset_over_readonly_current_link() {
        let base = std::env::temp_dir().join("bagger-test-reset-readonly");
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("root");
        let app = root.join("apps").join("resetapp");
        let version = app.join("1.0");
        std::fs::create_dir_all(&version).unwrap();
        std::fs::write(
            version.join("manifest.json"),
            r#"{"version": "1.0", "homepage": "https://example.com", "license": "MIT"}"#,
        )
        .unwrap();
        std::fs::write(version.join("install.json"), r#"{"architecture": "64bit"}"#).unwrap();

        std::env::set_var("SCOOP", &root);
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));

        junction::create(&version, app.join("current")).unwrap();
        let mut perms = std::fs::symlink_metadata(app.join("current"))
            .unwrap()
            .permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(app.join("current"), perms).unwrap();

        let session = Session::new();
        execute(
            Args {
                package: "resetapp".to_owned(),
                version: None,
            },
            &session,
        )
        .expect("reset over a readonly link should succeed");

        let target = std::fs::read_link(app.join("current")).unwrap();
        assert_eq!(target.file_name(), version.file_name());

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        // Clear readonly before removal so cleanup cannot fail on it.
        let mut perms = std::fs::symlink_metadata(app.join("current"))
            .unwrap()
            .permissions();
        perms.set_readonly(false);
        std::fs::set_permissions(app.join("current"), perms).unwrap();
        std::fs::remove_dir_all(&base).ok();
    }
}
