//! Dynamic shell-completion providers for package/bucket/config names.
//!
//! These back `#[arg(add = ArgValueCompleter::new(...))]` annotations and
//! run inside `clap_complete::CompleteEnv`, i.e. on every TAB press in a
//! bare process whose stdout is the candidate protocol. Rules:
//! - never print anything (stdout is candidates, stderr would leak into some
//!   shells — stay silent entirely),
//! - never panic: all failures fold to an empty candidate list,
//! - keep it fast: name walks only, no manifest parsing beyond what the
//!   query layer already does.

use std::ffi::OsStr;

use clap_complete::engine::CompletionCandidate;
use scoop_rs::Session;

fn session() -> Session {
    Session::new()
}

fn candidates(current: &OsStr, names: Vec<(String, Option<String>)>) -> Vec<CompletionCandidate> {
    let current = current.to_string_lossy();
    names
        .into_iter()
        .filter(|(name, _)| name.starts_with(current.as_ref()))
        .map(|(name, help)| {
            let candidate = CompletionCandidate::new(name);
            match help {
                Some(help) => candidate.help(Some(help.into())),
                None => candidate,
            }
        })
        .collect()
}

fn plain(current: &OsStr, names: Vec<String>) -> Vec<CompletionCandidate> {
    candidates(
        current,
        names.into_iter().map(|name| (name, None)).collect(),
    )
}

/// Installed app names, with installed versions as help text.
/// For `uninstall`, `upgrade`, `hold`, `cleanup`, `reset`, …
pub fn installed_apps(current: &OsStr) -> Vec<CompletionCandidate> {
    let pairs = scoop_rs::complete::installed_app_versions(&session());
    candidates(
        current,
        pairs
            .into_iter()
            .map(|(name, version)| {
                let help = (!version.is_empty()).then_some(version);
                (name, help)
            })
            .collect(),
    )
}

/// Installed app names plus `all`. For `checkver`.
pub fn checkver_targets(current: &OsStr) -> Vec<CompletionCandidate> {
    let mut names = scoop_rs::complete::installed_app_names(&session());
    if "all".starts_with(&*current.to_string_lossy()) {
        names.push("all".to_string());
    }
    plain(current, names)
}

/// Every manifest in every added bucket, with the bucket as help text.
/// For `install`, `download`, `search`.
pub fn known_apps(current: &OsStr) -> Vec<CompletionCandidate> {
    let pairs = scoop_rs::complete::bucket_app_names(&session());
    candidates(
        current,
        pairs
            .into_iter()
            .map(|(app, bucket)| (app, Some(bucket)))
            .collect(),
    )
}

/// Union of installed and bucket-known names. For commands that search
/// both universes (`info`, `cat`, `depends`, `home`, `cache`, `virustotal`).
pub fn all_apps(current: &OsStr) -> Vec<CompletionCandidate> {
    plain(current, scoop_rs::complete::all_app_names(&session()))
}

/// Added bucket names. For `bucket rm`.
pub fn buckets(current: &OsStr) -> Vec<CompletionCandidate> {
    plain(current, scoop_rs::complete::bucket_names(&session()))
}

/// Known (installable) bucket names with their repos as help text.
/// For `bucket add`. Static: no session needed.
pub fn known_buckets(current: &OsStr) -> Vec<CompletionCandidate> {
    let pairs = scoop_rs::operation::bucket_list_known();
    candidates(
        current,
        pairs
            .into_iter()
            .map(|(name, repo)| (name.to_string(), Some(repo.to_string())))
            .collect(),
    )
}

/// Settable config keys. Static: mirrors `Config::set` arms in scoop-rs.
pub fn config_keys(current: &OsStr) -> Vec<CompletionCandidate> {
    plain(
        current,
        scoop_rs::complete::config_keys()
            .into_iter()
            .map(str::to_string)
            .collect(),
    )
}

/// Shim basenames. For `which`.
pub fn shims(current: &OsStr) -> Vec<CompletionCandidate> {
    plain(current, scoop_rs::complete::shim_names(&session()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    /// The engine resolves annotated positionals to live candidates:
    /// `uninstall neo` must offer `neovim` from the fabricated root.
    #[test]
    fn engine_completes_installed_app_names() {
        let _guard = scoop_rs_test_guard();
        let base = std::env::temp_dir().join(format!("bagger-test-engine-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("root");
        let version = root.join("apps/neovim/0.11.0");
        std::fs::create_dir_all(&version).unwrap();
        std::fs::write(
            version.join("manifest.json"),
            r#"{"version": "0.11.0", "homepage": "https://example.com", "license": "MIT"}"#,
        )
        .unwrap();
        std::fs::write(
            version.join("install.json"),
            r#"{"architecture": "64bit", "bucket": "main"}"#,
        )
        .unwrap();
        #[cfg(windows)]
        scoop_rs::symlink_dir(&version, root.join("apps/neovim/current")).unwrap();
        std::env::set_var("SCOOP", &root);
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));

        let mut cmd = crate::cmd::Cli::command();
        let args = vec![
            std::ffi::OsString::from("bagger"),
            std::ffi::OsString::from("uninstall"),
            std::ffi::OsString::from("neo"),
        ];
        let got = clap_complete::engine::complete(&mut cmd, args, 2, None).unwrap();
        let values: Vec<String> = got
            .iter()
            .map(|c| c.get_value().to_string_lossy().into_owned())
            .collect();
        assert!(
            values.contains(&"neovim".to_string()),
            "expected neovim in {values:?}"
        );

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }

    /// `scoop_rs::test_support` is crate-private to scoop-rs; serialize on
    /// our own mutex for the env-mutating engine test.
    fn scoop_rs_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static GUARD: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        GUARD
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .unwrap()
    }
}
