//! Name providers for shell completion.
//!
//! These feed `clap_complete` dynamic completers (`ArgValueCompleter`) so
//! e.g. `bagger uninstall neo<TAB>` offers `neovim`. They are deliberately
//! total and silent: completion runs on every TAB press in a bare process
//! whose stdout is the candidate protocol, so errors fold to an empty list
//! and nothing is ever printed.

use crate::Session;

/// Names of installed apps, sorted.
pub fn installed_app_names(session: &Session) -> Vec<String> {
    let mut names = crate::package::query::query_installed(session, &["*"], &[])
        .map(|pkgs| pkgs.iter().map(|p| p.name().to_owned()).collect::<Vec<_>>())
        .unwrap_or_default();
    names.sort();
    names
}

/// Names of installed apps paired with their installed version (used as the
/// completion help text).
pub fn installed_app_versions(session: &Session) -> Vec<(String, String)> {
    let mut pairs = crate::package::query::query_installed(session, &["*"], &[])
        .map(|pkgs| {
            pkgs.iter()
                .map(|p| {
                    (
                        p.name().to_owned(),
                        p.installed_version().unwrap_or_default().to_owned(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    pairs.sort();
    pairs
}

/// Names of added buckets, sorted.
pub fn bucket_names(session: &Session) -> Vec<String> {
    let mut names = crate::bucket::bucket_added(session)
        .map(|buckets| {
            buckets
                .iter()
                .map(|b| b.name().to_owned())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// `(app, bucket)` pairs for every manifest in every added bucket, sorted.
/// This is the universe `install`/`search`/`download` resolve against.
pub fn bucket_app_names(session: &Session) -> Vec<(String, String)> {
    let mut pairs = crate::bucket::bucket_added(session)
        .map(|buckets| {
            buckets
                .iter()
                .flat_map(|b| {
                    let bucket = b.name().to_owned();
                    b.manifests()
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(move |de| {
                            let name = de.file_name();
                            let name = name.to_str()?;
                            name.strip_suffix(".json")
                                .map(|stem| (stem.to_owned(), bucket.clone()))
                        })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    pairs.sort();
    pairs.dedup();
    pairs
}

/// Union of installed and bucket-known app names, sorted and deduplicated.
/// Advisory superset for commands that search both universes.
pub fn all_app_names(session: &Session) -> Vec<String> {
    let mut names: Vec<String> = installed_app_names(session);
    names.extend(bucket_app_names(session).into_iter().map(|(app, _)| app));
    names.sort();
    names.dedup();
    names
}

/// Keys accepted by `Config::set` (i.e. by `bagger config set/unset`).
/// Keep in sync with the match arms there; both spellings included where
/// aliases exist.
pub fn config_keys() -> Vec<&'static str> {
    vec![
        "7zipextract_use_external",
        "alias",
        "aria2-enabled",
        "aria2-fallback-enabled",
        "aria2-warning-enabled",
        "aria2_enabled",
        "aria2_fallback_enabled",
        "aria2_warning_enabled",
        "cat_style",
        "debug",
        "default_architecture",
        "force_update",
        "gh_token",
        "ignore_running_processes",
        "last_update",
        "manifest_review",
        "private_hosts",
        "proxy",
        "show_manifest",
        "show_update_log",
        "update_nightly",
        "use_external_7zip",
        "use_git_history",
        "use_isolated_path",
        "use_lessmsi",
        "use_sqlite_cache",
    ]
}

/// Basenames of files in the `shims` directory with well-known shim
/// extensions stripped, sorted and deduplicated. This is the universe
/// `which` resolves against.
pub fn shim_names(session: &Session) -> Vec<String> {
    const SHIM_EXTS: [&str; 5] = ["exe", "cmd", "ps1", "bat", "shim"];
    let shims_dir = session.config().root_path().join("shims");
    let mut names = shims_dir
        .read_dir()
        .map(|entries| {
            entries
                .filter_map(|de| de.ok())
                .filter_map(|de| {
                    let name = de.file_name();
                    let name = name.to_str()?;
                    if !de.path().is_file() {
                        return None;
                    }
                    match name.rsplit_once('.') {
                        Some((stem, ext))
                            if SHIM_EXTS.contains(&ext.to_ascii_lowercase().as_str()) =>
                        {
                            Some(stem.to_owned())
                        }
                        _ => Some(name.to_owned()),
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Fabricate a root with one installed app and one bucket holding two
    /// manifests, mirroring the query-test layout.
    fn fixture() -> (std::sync::MutexGuard<'static, ()>, std::path::PathBuf) {
        let guard = crate::test_support::env_guard();
        let base =
            std::env::temp_dir().join(format!("bagger-test-complete-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("root");

        let version = root.join("apps/someapp/1.0");
        std::fs::create_dir_all(&version).unwrap();
        std::fs::write(
            version.join("manifest.json"),
            r#"{"version": "1.0", "homepage": "https://example.com", "license": "MIT"}"#,
        )
        .unwrap();
        std::fs::write(
            version.join("install.json"),
            r#"{"architecture": "64bit", "bucket": "fake"}"#,
        )
        .unwrap();
        crate::internal::fs::symlink_dir(&version, root.join("apps/someapp/current")).unwrap();

        let bucket = root.join("buckets/fakeb/bucket");
        std::fs::create_dir_all(&bucket).unwrap();
        std::fs::write(bucket.join("someapp.json"), "{}").unwrap();
        std::fs::write(bucket.join("otherapp.json"), "{}").unwrap();

        std::fs::create_dir_all(root.join("shims")).unwrap();
        std::fs::write(root.join("shims/otherapp.cmd"), "@echo off\n").unwrap();

        std::env::set_var("SCOOP", &root);
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));
        (guard, base)
    }

    fn cleanup(base: &std::path::Path) {
        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(base).ok();
    }

    #[test]
    fn completion_names_cover_both_universes() {
        let (_guard, base) = fixture();
        let session = Session::new();

        assert_eq!(installed_app_names(&session), vec!["someapp".to_string()]);
        assert_eq!(
            installed_app_versions(&session),
            vec![("someapp".to_string(), "1.0".to_string())]
        );
        assert_eq!(bucket_names(&session), vec!["fakeb".to_string()]);
        assert_eq!(
            bucket_app_names(&session),
            vec![
                ("otherapp".to_string(), "fakeb".to_string()),
                ("someapp".to_string(), "fakeb".to_string()),
            ]
        );
        assert_eq!(
            all_app_names(&session),
            vec!["otherapp".to_string(), "someapp".to_string()]
        );
        assert_eq!(shim_names(&session), vec!["otherapp".to_string()]);

        cleanup(&base);
    }
}
