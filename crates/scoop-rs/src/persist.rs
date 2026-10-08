use crate::{error::Fallible, internal, package::Package, Session};

/// Remove persisted files/directories symlinks for a package.
pub fn unlink(session: &Session, package: &Package) -> Fallible<()> {
    assert!(package.is_installed());

    if let Some(persists) = package.manifest().persist() {
        let config = session.config();
        let mut app_path = config.root_path().join("apps");
        app_path.push(package.name());

        let version = if config.no_junction() {
            package.installed_version().unwrap()
        } else {
            "current"
        };

        let persist_path = app_path.join(version);
        for persist in persists {
            assert!(!persist.is_empty());

            let src = internal::path::normalize_path(persist_path.join(persist[0]));
            internal::fs::remove_symlink(src)?;
        }
    }
    Ok(())
}

/// Create persisted files/directories symlinks for a package.
///
/// This moves data that should persist across updates to a `persist` directory
/// and creates symlinks from the app's install directory to the persist directory.
pub fn link(session: &Session, package: &Package) -> Fallible<()> {
    if let Some(persists) = package.manifest().persist() {
        let config = session.config();
        let mut app_path = config.root_path().join("apps");
        app_path.push(package.name());

        let version = if config.no_junction() {
            package.version()
        } else {
            "current"
        };

        let persist_root = config.root_path().join("persist").join(package.name());
        let app_version_path = app_path.join(version);

        for persist in persists {
            assert!(!persist.is_empty());

            // The persist entry can be either a file or directory path
            let rel_path = &persist[0];
            let app_path_item = app_version_path.join(rel_path);
            let persist_path_item = persist_root.join(rel_path);

            // Mirror upstream `persist_data`: existing store data always
            // wins (e.g. upgrading over a previous install); fresh app
            // content is moved into the store; otherwise an empty store
            // directory is created. Never rename over existing data.
            if persist_path_item.exists() {
                // Stash conflicting fresh content as `<name>.original`
                // (upstream parity), then link the store over it.
                if app_path_item.exists() && !is_link(&app_path_item) {
                    let original = app_path_item.with_file_name(format!(
                        "{}.original",
                        app_path_item
                            .file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_default()
                    ));
                    let _ = std::fs::remove_file(&original);
                    let _ = std::fs::remove_dir_all(&original);
                    std::fs::rename(&app_path_item, &original)?;
                }
                link_item(&persist_path_item, &app_path_item)?;
            } else if app_path_item.exists() && !is_link(&app_path_item) {
                internal::fs::ensure_dir(persist_path_item.parent().unwrap())?;
                std::fs::rename(&app_path_item, &persist_path_item)?;
                link_item(&persist_path_item, &app_path_item)?;
            } else {
                // Neither side exists yet. Upstream creates a directory by
                // default (a file entry misdetected as a directory is the
                // documented tradeoff); link it so later content persists.
                if !is_link(&app_path_item) {
                    internal::fs::ensure_dir(&persist_path_item)?;
                    link_item(&persist_path_item, &app_path_item)?;
                }
            }
        }
    }
    Ok(())
}

/// Whether `path` is itself a symlink/junction (not just pointing at one).
fn is_link(path: &std::path::Path) -> bool {
    std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Link `app_path` at the persist store entry, removing a stale link first.
fn link_item(persist_path_item: &std::path::Path, app_path_item: &std::path::Path) -> Fallible<()> {
    internal::fs::remove_symlink(app_path_item)?;
    if persist_path_item.is_dir() {
        internal::fs::symlink_dir(persist_path_item, app_path_item)?;
    } else {
        internal::fs::symlink_file(persist_path_item, app_path_item)?;
    }
    Ok(())
}

/// Grant the `Users` group write access to the persist root.
///
/// Mirrors upstream `persist_permission`, which global installs apply when
/// running elevated: without it, non-admin users cannot write the data of
/// globally installed apps. Only the root entry is stamped (`(OI)` object
/// inherit, like upstream's `ObjectInherit` rule); existing content keeps
/// its ACLs.
#[cfg(windows)]
pub fn grant_users_write(persist_root: &std::path::Path) -> Fallible<()> {
    let output = std::process::Command::new("icacls")
        .arg(persist_root)
        .arg("/grant")
        .arg("*S-1-5-32-545:(OI)W")
        .output()
        .map_err(|e| crate::Error::Custom(format!("failed to run icacls: {e}")))?;
    if !output.status.success() {
        return Err(crate::Error::Custom(format!(
            "icacls failed to grant Users write on '{}': {}",
            persist_root.display(),
            String::from_utf8_lossy(&output.stderr).trim(),
        )));
    }
    Ok(())
}

#[cfg(not(windows))]
pub fn grant_users_write(_persist_root: &std::path::Path) -> Fallible<()> {
    Ok(())
}

/// Remove persist links inside an app version directory.
///
/// Used when dropping old versions (`cleanup`): each entry is unlinked only
/// when it is itself a link — regular files and directories (including
/// stashed `<name>.original` copies) are left alone, and the persist store
/// itself is never touched. Junction removal goes through the hardened
/// helper (readonly flags cleared, file-vs-dir aware) because raw
/// `remove_dir` fails on junctions with os error 5.
pub fn unlink_links(version_path: &std::path::Path, rel_paths: &[&str]) -> Fallible<()> {
    for rel in rel_paths {
        let link_path = version_path.join(rel);
        match std::fs::symlink_metadata(&link_path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                internal::fs::remove_symlink(&link_path)?
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::manifest::Manifest;
    use crate::package::Package;

    fn persist_package() -> Package {
        let manifest = Manifest::parse_bytes(
            br#"{"version": "2.0", "homepage": "https://example.com", "license": "MIT", "persist": ["Configurations"]}"#,
            std::path::Path::new("persist.json"),
        )
        .expect("fixture manifest should parse");
        Package::from("persistapp", "main", manifest)
    }

    fn test_session(base: &std::path::Path) -> Session {
        std::env::set_var("SCOOP", base.join("root"));
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));
        Session::new()
    }

    /// Upgrading over an existing store keeps user data (the old code
    /// renamed the fresh dir over the store and died with os error 5).
    #[test]
    fn link_keeps_existing_store_on_upgrade() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-persist-upgrade");
        let _ = std::fs::remove_dir_all(&base);
        let session = test_session(&base);
        let pkg = persist_package();

        // Previous install's user data already in the store.
        let store = base.join("root/persist/persistapp/Configurations");
        std::fs::create_dir_all(&store).unwrap();
        std::fs::write(store.join("user.txt"), "user-data").unwrap();

        // Fresh version dir ships defaults.
        let app_item = base.join("root/apps/persistapp/current/Configurations");
        std::fs::create_dir_all(&app_item).unwrap();
        std::fs::write(app_item.join("defaults.txt"), "defaults").unwrap();

        link(&session, &pkg).expect("link should succeed");

        // Store wins; defaults stashed as `.original`; app path links store.
        assert_eq!(
            std::fs::read_to_string(store.join("user.txt")).unwrap(),
            "user-data"
        );
        assert_eq!(
            std::fs::read_to_string(app_item.join("user.txt")).unwrap(),
            "user-data"
        );
        assert_eq!(
            std::fs::read_to_string(
                base.join("root/apps/persistapp/current/Configurations.original/defaults.txt")
            )
            .unwrap(),
            "defaults"
        );

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }

    /// Fresh installs still move app content into the store and link back.
    #[test]
    fn link_moves_fresh_content_into_store() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-persist-fresh");
        let _ = std::fs::remove_dir_all(&base);
        let session = test_session(&base);
        let pkg = persist_package();

        let app_item = base.join("root/apps/persistapp/current/Configurations");
        std::fs::create_dir_all(&app_item).unwrap();
        std::fs::write(app_item.join("fresh.txt"), "fresh").unwrap();

        link(&session, &pkg).expect("link should succeed");

        let store = base.join("root/persist/persistapp/Configurations");
        assert_eq!(
            std::fs::read_to_string(store.join("fresh.txt")).unwrap(),
            "fresh"
        );
        assert_eq!(
            std::fs::read_to_string(app_item.join("fresh.txt")).unwrap(),
            "fresh"
        );

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }

    /// `unlink_links` removes junctions (the raw `remove_dir` used by the
    /// old cleanup path fails on them with os error 5) while leaving real
    /// files, real dirs, and the store itself alone.
    #[test]
    #[cfg(windows)]
    fn unlink_links_removes_junctions_only() {
        let base = std::env::temp_dir().join("bagger-test-persist-unlink");
        let _ = std::fs::remove_dir_all(&base);
        let version = base.join("1.0");
        let store = base.join("store/Data");
        std::fs::create_dir_all(&version).unwrap();
        std::fs::create_dir_all(&store).unwrap();
        std::fs::write(store.join("keep.txt"), "keep").unwrap();

        // A junction like installs create (possibly readonly, like
        // upstream's `attrib +R` junctions).
        internal::fs::symlink_dir(&store, &version.join("Data")).unwrap();
        let mut perms = std::fs::symlink_metadata(version.join("Data"))
            .unwrap()
            .permissions();
        perms.set_readonly(true);
        std::fs::set_permissions(version.join("Data"), perms).unwrap();

        // A real dir and a real file must survive.
        std::fs::create_dir_all(version.join("Real")).unwrap();
        std::fs::write(version.join("notes.txt"), "notes").unwrap();

        unlink_links(&version, &["Data", "Real", "notes.txt", "missing"]).unwrap();

        assert!(
            !version.join("Data").exists()
                || std::fs::symlink_metadata(version.join("Data")).is_err()
        );
        assert!(version.join("Real").is_dir());
        assert!(version.join("notes.txt").is_file());
        assert_eq!(
            std::fs::read_to_string(store.join("keep.txt")).unwrap(),
            "keep"
        );

        std::fs::remove_dir_all(&base).ok();
    }

    /// `grant_users_write` stamps the Users SID with write rights (verified
    /// through the SID, not the localized group name).
    #[test]
    #[cfg(windows)]
    fn grant_users_write_stamps_users_sid() {
        let base = std::env::temp_dir().join("bagger-test-persist-acl");
        let _ = std::fs::remove_dir_all(&base);
        std::fs::create_dir_all(&base).unwrap();

        grant_users_write(&base).expect("icacls should succeed");

        // Verified through the SID (locale-proof) via .NET directly, since
        // the `Get-Acl` cmdlet's module may not load in constrained hosts.
        let script = format!(
            "$rules = [System.IO.Directory]::GetAccessControl('{}').GetAccessRules($true, $false, [System.Security.Principal.SecurityIdentifier]); @($rules | Where-Object {{ $_.IdentityReference.Value -eq 'S-1-5-32-545' -and (($_.FileSystemRights -band [System.Security.AccessControl.FileSystemRights]::Write) -ne 0) }}).Count",
            base.display().to_string().replace('\'', "''")
        );
        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .output()
            .expect("powershell should run");
        assert!(
            output.status.success(),
            "powershell failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let count: u32 = String::from_utf8_lossy(&output.stdout)
            .trim()
            .parse()
            .unwrap_or(0);
        assert!(count >= 1, "Users SID should hold write rights");

        std::fs::remove_dir_all(&base).ok();
    }
}
