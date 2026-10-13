use once_cell::sync::Lazy;
use std::path::{Path, PathBuf};

use crate::{error::Fallible, internal, package::Package, Event, Session};

static SCOOP_SHORTCUT_DIR: Lazy<PathBuf> = Lazy::new(shortcut_dir);

/// Return the path to the per-user shortcut directory.
///
/// `~\AppData\Roaming\Microsoft\Windows\Start Menu\Programs\Scoop Apps`
fn shortcut_dir() -> PathBuf {
    let mut dir = dirs::config_dir().unwrap();
    dir.push("Microsoft/Windows/Start Menu/Programs/Scoop Apps");
    internal::path::normalize_path(dir)
}

/// Shortcut directory for the given scope: global installs land in the
/// all-users Start Menu (upstream parity), user installs in the per-user
/// one. Falls back to per-user when `ProgramData` is unavailable.
fn scope_shortcut_dir(global: bool) -> PathBuf {
    if global {
        if let Some(program_data) = std::env::var_os("ProgramData") {
            let mut dir = PathBuf::from(program_data);
            dir.push("Microsoft/Windows/Start Menu/Programs/Scoop Apps");
            return internal::path::normalize_path(dir);
        }
    }
    SCOOP_SHORTCUT_DIR.clone()
}

/// Shortcut file path for a manifest shortcut name. The `.lnk` suffix is
/// appended (never `set_extension`, which truncates dotted names like
/// `Node.js` to `Node.lnk`); backslashes in the name create subfolders
/// (e.g. `Cygwin\Cygwin`).
fn shortcut_path(base: &Path, name: &str) -> PathBuf {
    base.join(format!("{name}.lnk"))
}

/// Escape a string for embedding in a single-quoted PowerShell literal.
fn ps_quote(s: &str) -> String {
    s.replace('\'', "''")
}

/// Create a Windows `.lnk` shortcut using PowerShell.
fn create_lnk(
    shortcut_path: &Path,
    target: &Path,
    args: Option<&str>,
    icon: Option<&Path>,
) -> Fallible<()> {
    #[cfg(windows)]
    {
        let target_str = ps_quote(&target.to_string_lossy().replace('\\', "\\\\"));
        let args_str = ps_quote(args.unwrap_or(""));
        let shortcut_str = ps_quote(&shortcut_path.to_string_lossy().replace('\\', "\\\\"));
        let icon_line = match icon {
            Some(icon) => format!(
                "$sc.IconLocation = '{}'; ",
                ps_quote(&icon.to_string_lossy().replace('\\', "\\\\"))
            ),
            None => String::new(),
        };

        let ps_cmd = format!(
            "$ws = New-Object -ComObject WScript.Shell; \
            $sc = $ws.CreateShortcut('{}'); \
            $sc.TargetPath = '{}'; \
            $sc.Arguments = '{}'; \
            {} $sc.Save()",
            shortcut_str, target_str, args_str, icon_line
        );

        let output = std::process::Command::new("powershell.exe")
            .arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(&ps_cmd)
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(crate::Error::Custom(format!(
                "failed to create shortcut '{}': {}",
                shortcut_path.display(),
                stderr
            )));
        }

        Ok(())
    }

    #[cfg(unix)]
    {
        // On Unix, create a symlink as a simple shortcut
        let _ = std::fs::remove_file(shortcut_path);
        std::os::unix::fs::symlink(target, shortcut_path)?;
        Ok(())
    }

    #[cfg(not(any(windows, unix)))]
    {
        Err(crate::Error::Custom(
            "unsupported platform for shortcut creation".to_string(),
        ))
    }
}

/// Shortcut display names a manifest declares: `shortcuts[i][1]`, or the
/// target's leaf basename for lone-target entries. Shared by add/remove
/// so both sides resolve the same files (including `A\B` subfolders).
pub(crate) fn shortcut_names(
    manifest: &crate::package::manifest::Manifest,
    package_name: &str,
    version_dir: &Path,
) -> Vec<String> {
    manifest
        .shortcuts()
        .unwrap_or_default()
        .into_iter()
        .filter(|def| !def.is_empty())
        .map(|def| {
            if def.len() > 1 {
                def[1].to_owned()
            } else {
                internal::path::leaf_base(&version_dir.join(def[0]))
                    .unwrap_or(package_name)
                    .to_owned()
            }
        })
        .collect()
}

/// Remove shortcuts by display name (upgrade finalization): the stale set
/// a new manifest no longer declares. Missing files are fine.
pub(crate) fn remove_stale(
    session: &Session,
    stale: &std::collections::HashSet<String>,
) -> Fallible<()> {
    if stale.is_empty() {
        return Ok(());
    }
    let base = scope_shortcut_dir(session.config().is_global_scope());
    for name in stale {
        let path = shortcut_path(&base, name);
        if path.is_file() {
            if let Some(tx) = session.emitter() {
                let _ = tx.send(Event::PackageShortcutRemoveProgress(name.clone()));
            }
            let _ = std::fs::remove_file(&path);
        }
    }
    Ok(())
}

/// Add shortcut(s) for a given package.
///
/// Creates `.lnk` shortcuts in the user's Start Menu for each shortcut defined
/// in the package's `shortcuts` manifest field.
pub fn add(session: &Session, package: &Package) -> Fallible<()> {
    if let Some(shortcuts) = package.manifest().shortcuts() {
        let shortcut_base = scope_shortcut_dir(session.config().is_global_scope());
        internal::fs::ensure_dir(&shortcut_base)?;

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageShortcutRemoveStart);
        }

        let config = session.config();
        let version = if config.no_junction() {
            package.installed_version().unwrap_or(package.version())
        } else {
            "current"
        };

        let app_path = config.root_path().join("apps").join(package.name());
        let version_dir = app_path.join(version);

        for shortcut in shortcuts {
            assert!(!shortcut.is_empty());

            let target = version_dir.join(shortcut[0]);

            let name = if shortcut.len() > 1 {
                shortcut[1]
            } else {
                internal::path::leaf_base(&target).unwrap_or(package.name())
            };

            let path = shortcut_path(&shortcut_base, name);
            if let Some(parent) = path.parent() {
                internal::fs::ensure_dir(parent)?;
            }

            // `shortcuts` entries are `[target, name, args?, icon?]`;
            // the icon resolves against the app version directory.
            let args = shortcut.get(2).filter(|s| !s.is_empty()).copied();
            let icon = shortcut
                .get(3)
                .filter(|s| !s.is_empty())
                .map(|rel| version_dir.join(rel))
                .filter(|p| p.is_file());

            create_lnk(&path, &target, args, icon.as_deref())?;
        }

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageShortcutRemoveDone);
        }
    }

    Ok(())
}

/// Remove shortcut(s) for a given package.
pub fn remove(session: &Session, package: &Package) -> Fallible<()> {
    assert!(package.is_installed());

    if let Some(shortcuts) = package.manifest().shortcuts() {
        // Removal observes the install scope: installed packages are only
        // visible to queries under their own root, so the session scope
        // here is the package's scope.
        let shortcut_base = scope_shortcut_dir(session.config().is_global_scope());

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageShortcutRemoveStart);
        }

        for shortcut in shortcuts {
            let length = shortcut.len();
            assert!(length > 1);

            let path = shortcut_path(&shortcut_base, shortcut[1]);

            if let Some(tx) = session.emitter() {
                let shortcut_name = path.file_name().unwrap().to_str().unwrap().to_owned();
                let _ = tx.send(Event::PackageShortcutRemoveProgress(shortcut_name));
            }

            let _ = std::fs::remove_file(&path);
        }

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageShortcutRemoveDone);
        }
    }
    Ok(())
}
