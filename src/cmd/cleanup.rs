use crate::util::is_admin;
use crate::Result;
use clap::{ArgAction, Parser};
use scoop_rs::{operation, Session};
use std::{
    collections::HashSet,
    fs,
    path::{Component, Path, PathBuf},
};

/// Cleanup apps by removing old versions
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// Given named app(s) to be cleaned up
    #[arg(action = ArgAction::Append)]
    app: Vec<String>,
    /// Clean up all installed apps
    #[arg(short = 'a', long, action = ArgAction::SetTrue)]
    all: bool,
    /// Clean up globally installed apps
    #[arg(short = 'g', long, action = ArgAction::SetTrue)]
    global: bool,
    /// Remove download cache simultaneously
    #[arg(short = 'k', long, action = ArgAction::SetTrue)]
    cache: bool,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    // `arg_required_else_help` only checks that a CLI argument was supplied. Validate that the
    // request names apps or explicitly asks for all apps.
    if args.app.is_empty() && !args.all {
        return Err(anyhow::anyhow!(
            "no app names given. Either give apps to clean, use --all or * wildcard."
        ));
    }

    if args.global && !is_admin() {
        return Err(anyhow::anyhow!(
            "you need admin rights to cleanup global apps"
        ));
    }

    let all_apps = args.all || args.app.iter().any(|app| app == "*");
    let config = session.config();
    let user_apps = config.root_path().join("apps");
    let global_apps = config.global_path().join("apps");
    drop(config);

    // Scoop's `--global --all` includes both scopes; named --global requests target only the
    // global installation. The user root is the only scope for non-global requests.
    let mut app_roots = vec![(user_apps, false)];
    if args.global && all_apps {
        app_roots.push((global_apps, true));
    } else if args.global {
        app_roots.clear();
        app_roots.push((global_apps, true));
    }

    let mut selected = Vec::<(String, PathBuf, bool)>::new();
    let mut seen = HashSet::new();
    if all_apps {
        for (apps_path, global) in &app_roots {
            for name in installed_app_names(apps_path)? {
                if seen.insert((apps_path.clone(), name.clone())) {
                    selected.push((name, apps_path.clone(), *global));
                }
            }
        }
    } else {
        for requested in &args.app {
            let name = match requested.split_once('/') {
                Some((bucket, name)) if is_safe_app_name(bucket) && is_safe_app_name(name) => name,
                None if is_safe_app_name(requested) => requested,
                _ => return Err(anyhow::anyhow!("invalid app name: {requested}")),
            };

            let mut found = false;
            for (apps_path, global) in &app_roots {
                let app_path = apps_path.join(name);
                if app_path.is_dir() {
                    found = true;
                    if seen.insert((apps_path.clone(), name.to_owned())) {
                        selected.push((name.to_owned(), apps_path.clone(), *global));
                    }
                }
            }
            if !found {
                println!("{} was not installed.", requested);
            }
        }
    }

    for (name, apps_path, global) in &selected {
        cleanup(
            name,
            &apps_path.join(name),
            *global,
            !all_apps,
            args.cache,
            session,
        )?;
    }

    if args.cache {
        remove_download_temporary_files(session)?;
    }

    if all_apps {
        println!("Installed app versions are clean.");
        if args.cache {
            println!("Pruned obsolete caches for installed apps and removed partial downloads.");
        } else {
            println!("Download caches were left untouched; use --cache to prune obsolete caches.");
        }
    }

    Ok(())
}

fn installed_app_names(apps_path: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();
    let entries = match fs::read_dir(apps_path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(names),
        Err(error) => return Err(error.into()),
    };

    for entry in entries {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "scoop" {
            continue;
        }
        // Ignore broken/non-installed app directories when listing all apps. Named requests
        // still report an error from cleanup if their installation metadata cannot be resolved.
        if active_version(&entry.path())?.is_some() {
            names.push(name);
        }
    }

    names.sort_unstable();
    Ok(names)
}

fn cleanup(
    app: &str,
    app_path: &Path,
    global: bool,
    verbose: bool,
    cache: bool,
    session: &Session,
) -> Result<()> {
    if !app_path.is_dir() {
        if verbose {
            println!("{} is not installed.", app);
        }
        return Ok(());
    }

    let active_version = active_version(app_path)?
        .ok_or_else(|| anyhow::anyhow!("could not determine the active version for {app}"))?;
    let mut old_versions = Vec::new();
    for entry in fs::read_dir(app_path)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let version_name = entry.file_name().to_string_lossy().into_owned();
        if !is_cleanable_version(&version_name, &active_version.directory_name) {
            continue;
        }
        old_versions.push((version_name, entry.path()));
    }

    if old_versions.is_empty() {
        if verbose {
            println!("{} is already clean", app);
        }
    } else {
        print!("Removing {}{}:", app, if global { " (global)" } else { "" });
        for (version, version_path) in old_versions {
            // Never delete a version directory that lacks bagger install
            // metadata: it may be a failed/partial upgrade whose `current`
            // link already points at it. Deleting those destroys the
            // pending upgrade instead of the old version.
            if !version_path.join("manifest.json").is_file()
                || !version_path.join("install.json").is_file()
            {
                eprintln!("\nSkipping {app} {version}: no install metadata (leaving it alone).");
                continue;
            }
            let removal = unlink_persist_links(&version_path)
                .map_err(|error| {
                    anyhow::anyhow!("failed to unlink persist paths for {app} {version}: {error}")
                })
                .and_then(|()| {
                    remove_dir_all::remove_dir_all(&version_path).map_err(|error| {
                        anyhow::anyhow!(
                            "failed to remove old version {}: {error}",
                            version_path.display()
                        )
                    })
                });
            match removal {
                Ok(()) => print!(" {}", version),
                // Keep cleaning the rest: a locked old version (e.g. its
                // files are still loaded by a running process) must not
                // abort the whole run. Say who holds it when known.
                Err(error) => {
                    let mut hint = String::new();
                    if let Ok(procs) = scoop_rs::running_apps_under(app_path) {
                        if !procs.is_empty() {
                            hint =
                                format!(" (in use by: {}; quit them and retry)", procs.join(", "));
                        }
                    }
                    eprintln!("\nCouldn't remove {app} {version}: {error}{hint}");
                }
            }
        }
        println!();
    }

    if cache {
        let files = operation::cache_list(session, "*")?;
        for file in files {
            // Without a known active manifest version (dangling `current`),
            // keep every cache file rather than risk pruning the live one.
            let stale = match &active_version.manifest_version {
                Some(active) => file.version() != *active,
                None => false,
            };
            if file.package_name().eq_ignore_ascii_case(app) && stale {
                fs::remove_file(file.path())?;
            }
        }
    }

    Ok(())
}

struct ActiveVersion {
    directory_name: String,
    manifest_version: Option<String>,
}

/// Whether a version directory is cleanup-eligible: not `current`, not the
/// active version, and not a `_<version>.old*` force-reinstall backup
/// (upstream `Get-InstalledVersion` excludes those too — they are user
/// evidence, not old versions).
fn is_cleanable_version(name: &str, active: &str) -> bool {
    name != "current" && name != active && !(name.starts_with('_') && name.contains(".old"))
}

fn active_version(app_path: &Path) -> Result<Option<ActiveVersion>> {
    let current_path = app_path.join("current");

    // The `current` link target is sacred: whatever it points at is the
    // live version and must never be treated as an old version — even when
    // its directory lacks install metadata (e.g. a failed upgrade left a
    // manifest-less dir behind and repointed the link). Only fall back to
    // manifest scanning when there is no `current` link at all.
    if let Ok(target) = fs::read_link(&current_path) {
        let directory_name = target.file_name().map(|n| n.to_string_lossy().into_owned());
        if let Some(directory_name) = directory_name {
            let current_manifest = current_path.join("manifest.json");
            let manifest_version = read_manifest_version(&current_manifest).ok();
            return Ok(Some(ActiveVersion {
                directory_name,
                manifest_version,
            }));
        }
    }

    let current_manifest = current_path.join("manifest.json");
    if current_manifest.is_file() {
        let active_path = fs::canonicalize(&current_path)?;
        for entry in fs::read_dir(app_path)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if !file_type.is_dir() || file_type.is_symlink() {
                continue;
            }
            if fs::canonicalize(entry.path())? == active_path {
                return Ok(Some(ActiveVersion {
                    directory_name: entry.file_name().to_string_lossy().into_owned(),
                    manifest_version: Some(read_manifest_version(&current_manifest)?),
                }));
            }
        }
        return Err(anyhow::anyhow!(
            "`current` for {} does not resolve to a version directory",
            app_path.display()
        ));
    }

    // If there is no `current` alias (as with no-junction installations), a sole installed
    // version is safe to preserve. With multiple candidates the active version is ambiguous, so
    // refuse to delete anything rather than risk removing it.
    let mut candidates = Vec::new();
    for entry in fs::read_dir(app_path)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let version_path = entry.path();
        let manifest = version_path.join("manifest.json");
        let install_info = version_path.join("install.json");
        if manifest.is_file() && install_info.is_file() {
            candidates.push(ActiveVersion {
                directory_name: entry.file_name().to_string_lossy().into_owned(),
                manifest_version: Some(read_manifest_version(&manifest)?),
            });
        }
    }

    if candidates.len() == 1 {
        return Ok(candidates.pop());
    }
    if candidates.is_empty() {
        return Ok(None);
    }

    Err(anyhow::anyhow!(
        "multiple versions found for {} but no `current` link identifies the active one",
        app_path.display()
    ))
}

fn read_manifest_version(manifest_path: &Path) -> Result<String> {
    let manifest: serde_json::Value = serde_json::from_reader(fs::File::open(manifest_path)?)?;
    manifest
        .get("version")
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| anyhow::anyhow!("manifest {} has no version", manifest_path.display()))
}

fn unlink_persist_links(version_path: &Path) -> Result<()> {
    let manifest_path = version_path.join("manifest.json");
    if !manifest_path.is_file() {
        return Ok(());
    }
    let manifest: serde_json::Value = serde_json::from_reader(fs::File::open(manifest_path)?)?;
    let Some(entries) = manifest
        .get("persist")
        .and_then(serde_json::Value::as_array)
    else {
        return Ok(());
    };

    for entry in entries {
        let source = match entry {
            serde_json::Value::String(path) => Some(path.as_str()),
            serde_json::Value::Array(paths) => paths.first().and_then(serde_json::Value::as_str),
            _ => None,
        };
        let Some(source) = source else { continue };
        let relative = Path::new(source);
        if relative.is_absolute()
            || relative.components().any(|part| {
                matches!(
                    part,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(anyhow::anyhow!(
                "unsafe persist path in {}: {source}",
                version_path.display()
            ));
        }

        // Unlink via the hardened helper: raw remove_dir fails on
        // junctions with os error 5.
        scoop_rs::persist_unlink_links(version_path, &[source])?;
    }

    Ok(())
}

fn remove_download_temporary_files(session: &Session) -> Result<()> {
    let cache_path = session.config().cache_path().to_owned();
    let entries = match fs::read_dir(cache_path) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };

    for entry in entries {
        let entry = entry?;
        if entry.file_type()?.is_file()
            && entry
                .path()
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.ends_with(".download"))
        {
            fs::remove_file(entry.path())?;
        }
    }
    Ok(())
}

fn is_safe_app_name(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('\\')
        && !name.contains('/')
        && !name.contains(':')
        && !name.contains("..")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_manifest(dir: &std::path::Path, version: &str) {
        std::fs::write(
            dir.join("manifest.json"),
            format!(r#"{{"version": "{version}"}}"#),
        )
        .unwrap();
        std::fs::write(dir.join("install.json"), "{}").unwrap();
    }

    /// The `current` target is sacred: even when it lacks install metadata
    /// (failed upgrade), it must be reported as active — never deleted as
    /// old. This is the regression test for cleanup eating pending upgrades.
    #[test]
    #[cfg(windows)]
    fn active_version_prefers_current_target() {
        let base = std::env::temp_dir().join("bagger-test-cleanup-active");
        let _ = std::fs::remove_dir_all(&base);
        let app = base.join("app");
        let old = app.join("1.0");
        let pending = app.join("2.0");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&pending).unwrap();
        write_manifest(&old, "1.0");
        // 2.0 has no manifests (failed upgrade), yet `current` points at it.
        junction::create(&pending, app.join("current")).unwrap();

        let active = active_version(&app).unwrap().expect("active expected");
        assert_eq!(active.directory_name, "2.0");
        assert_eq!(active.manifest_version, None);

        std::fs::remove_dir_all(&base).ok();
    }

    /// A healthy layout still resolves through manifests.
    #[test]
    #[cfg(windows)]
    fn active_version_resolves_healthy_layout() {
        let base = std::env::temp_dir().join("bagger-test-cleanup-healthy");
        let _ = std::fs::remove_dir_all(&base);
        let app = base.join("app");
        let old = app.join("1.0");
        let new = app.join("2.0");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        write_manifest(&old, "1.0");
        write_manifest(&new, "2.0");
        junction::create(&new, app.join("current")).unwrap();

        let active = active_version(&app).unwrap().expect("active expected");
        assert_eq!(active.directory_name, "2.0");
        assert_eq!(active.manifest_version.as_deref(), Some("2.0"));

        std::fs::remove_dir_all(&base).ok();
    }

    /// Force-reinstall backups (`_<version>.old*`) are user evidence, not
    /// old versions: cleanup must leave them alone, like upstream's
    /// `Get-InstalledVersion` exclusion.
    #[test]
    fn cleanable_version_skips_current_active_and_backups() {
        assert!(!is_cleanable_version("current", "1.0"));
        assert!(!is_cleanable_version("1.0", "1.0"));
        assert!(is_cleanable_version("0.9", "1.0"));
        assert!(!is_cleanable_version("_1.0.old", "1.0"));
        assert!(!is_cleanable_version("_1.0.old(1)", "1.0"));
        // Other underscore names are still eligible.
        assert!(is_cleanable_version("_tmp", "1.0"));
    }
}
