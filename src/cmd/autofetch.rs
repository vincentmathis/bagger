use clap::{ArgAction, Parser};
use crossterm::style::Stylize;
use scoop_rs::{operation, QueryOption, Session};

use crate::Result;

/// Fetch latest versions and preview autoupdate URLs for apps
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package name (or "all" for all installed apps with autoupdate)
    #[arg(required = true)]
    package: String,

    /// Rewrite manifests with the new version, URLs and download-mode hashes
    #[arg(short = 'w', long, action = ArgAction::SetTrue)]
    write: bool,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    if args.package == "all" {
        let packages = operation::package_query(session, vec![], vec![], true)?;

        let mut updated = 0usize;
        for pkg in &packages {
            if autofetch_one(session, pkg, args.write)? {
                updated += 1;
            }
        }

        if updated == 0 {
            println!("All apps are up to date (or have no checkver/autoupdate info).");
        } else {
            println!("\n{updated} app(s) have updates available.");
        }

        return Ok(());
    }

    let result = operation::package_query(
        session,
        vec![args.package.as_str()],
        vec![QueryOption::Explicit],
        false,
    )?;

    if result.is_empty() {
        eprintln!("Package '{}' not found in any bucket.", args.package);
        return Ok(());
    }

    autofetch_one(session, &result[0], args.write)?;

    Ok(())
}

/// Run checkver + autoupdate URL expansion for a single package.
///
/// Returns `Ok(true)` if a newer version was found.
fn autofetch_one(session: &Session, pkg: &scoop_rs::Package, write: bool) -> Result<bool> {
    let manifest = pkg.manifest();

    if manifest.effective_checkver().is_none() {
        println!(
            "{}: {}",
            pkg.name(),
            "no checkver definition, skipping".dark_grey()
        );
        return Ok(false);
    }

    let result = operation::checkver(session, pkg)?;
    let latest = match result.latest_version {
        Some(v) => v,
        None => {
            println!(
                "{}: {}",
                pkg.name().green(),
                "could not determine latest version".yellow()
            );
            return Ok(false);
        }
    };

    let current = result.current_version.as_deref().unwrap_or("unknown");
    if latest == current {
        println!(
            "{}: {} ({})",
            pkg.name().green(),
            "up to date".green(),
            current
        );
        return Ok(false);
    }

    println!(
        "{}: {} -> {}",
        pkg.name().green().bold(),
        current,
        latest.clone().yellow().bold()
    );

    let Some(autoupdate) = manifest.autoupdate() else {
        println!("  (no autoupdate section; bump version manually)");
        return Ok(true);
    };

    // Collect URL templates from the autoupdate section (top-level + per-arch).
    let mut templates: Vec<(&str, String)> = Vec::new();

    if let Some(urls) = autoupdate.url.as_ref() {
        for u in urls.devectorize() {
            templates.push(("noarch", u.to_string()));
        }
    }

    if let Some(arch) = autoupdate.architecture.as_ref() {
        for (label, spec) in [
            ("32bit", arch.ia32.as_ref()),
            ("64bit", arch.amd64.as_ref()),
            ("arm64", arch.aarch64.as_ref()),
        ] {
            if let Some(spec) = spec {
                if let Some(urls) = spec.url.as_ref() {
                    for u in urls.devectorize() {
                        templates.push((label, u.to_string()));
                    }
                }
            }
        }
    }

    if templates.is_empty() {
        println!("  (autoupdate has no URL templates)");
    } else {
        for (label, template) in &templates {
            let expanded =
                operation::expand_autoupdate_template(template, &latest, &result.captures);
            if expanded == *template {
                println!("  [{label}] {template}  (no $version/$match placeholder)");
            } else {
                println!("  [{label}] {expanded}");
            }
        }
    }

    // Report hash extraction modes so users know what autoupdate
    // automation will do next (a bare URL means extract mode upstream).
    let mut hash_modes: Vec<String> = Vec::new();
    if let Some(hashes) = autoupdate.hash.as_ref() {
        for h in hashes.devectorize() {
            let mode = h
                .mode
                .as_ref()
                .map(|m| format!("{m:?}").to_lowercase())
                .unwrap_or_else(|| {
                    if h.url.is_some() {
                        "extract".to_string()
                    } else {
                        "download".to_string()
                    }
                });
            let url = h.url.as_deref().unwrap_or("<asset url>");
            hash_modes.push(format!("{mode} from {url}"));
        }
    }
    if let Some(arch) = autoupdate.architecture.as_ref() {
        for spec in [&arch.ia32, &arch.amd64, &arch.aarch64]
            .into_iter()
            .flatten()
        {
            if let Some(hashes) = spec.hash.as_ref() {
                for h in hashes.devectorize() {
                    let mode = h
                        .mode
                        .as_ref()
                        .map(|m| format!("{m:?}").to_lowercase())
                        .unwrap_or_else(|| {
                            if h.url.is_some() {
                                "extract".to_string()
                            } else {
                                "download".to_string()
                            }
                        });
                    let url = h.url.as_deref().unwrap_or("<asset url>");
                    hash_modes.push(format!("{mode} from {url}"));
                }
            }
        }
    }
    for mode in &hash_modes {
        println!("  hash: {mode}");
    }

    if write {
        match operation::autoupdate_apply(session, pkg, &latest, &result.captures, true) {
            Ok(applied) => {
                for url in &applied.rewritten_urls {
                    println!("  {} {}", "hashed".green(), url);
                }
                for hash in &applied.rewritten_hashes {
                    println!("  {} {}", "hash:".dark_grey(), hash);
                }
                for skipped in &applied.skipped {
                    println!("  {} {}", "skipped:".yellow(), skipped);
                }
                println!(
                    "  {} {}",
                    "updated".green(),
                    pkg.manifest().path().display()
                );
            }
            Err(e) => {
                eprintln!("  {} {}", "autoupdate failed:".red(), e);
            }
        }
    }

    Ok(true)
}
