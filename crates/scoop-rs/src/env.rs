use crate::{config, error::Fallible, internal, package::Package, Event, Session};

/// Unset all environment variables defined by a given package.
pub fn remove(session: &Session, package: &Package) -> Fallible<()> {
    assert!(package.is_installed());

    let config = session.config();
    let global = config.is_global_scope();
    let mut changed = false;

    // Unset environment variables
    if let Some(env_set) = package.manifest().env_set() {
        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageEnvVarRemoveStart);
        }

        let keys = env_set.keys();
        for key in keys {
            internal::env::set_scoped(key, None, global)?;
            // Upstream also clears the current session (`Remove-Item env:\`).
            std::env::remove_var(key);
        }
        changed = true;

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageEnvVarRemoveDone);
        }
    }

    // Remove environment path. Upstream removes from the default target
    // and the isolated target, covering installs from before isolation
    // was enabled (and vice versa).
    if let Some(env_add_path) = package.manifest().env_add_path() {
        let entries = install_path_entries(session, package);

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageEnvPathRemoveStart);
        }

        for target in path_targets(&config) {
            let mut paths = internal::env::get_path_like_env_scoped(&target, global)?;
            let before = paths.len();
            paths.retain(|p| !entries.contains(p));
            if paths.len() != before {
                let updated = internal::env::join_path_list(&paths)?;
                internal::env::set_scoped(&target, Some(&updated), global)?;
                changed = true;
            }
        }

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageEnvPathRemoveDone);
        }
    }

    if changed {
        internal::env::broadcast_env_change();
    }

    Ok(())
}

/// Set environment variables and path entries for a given package during installation.
pub fn install(session: &Session, package: &Package) -> Fallible<()> {
    let config = session.config();
    let global = config.is_global_scope();
    let mut changed = false;

    // Set environment variables (values expand the full hook scope, like
    // upstream `$ExecutionContext.InvokeCommand.ExpandString`).
    if let Some(env_set) = package.manifest().env_set() {
        for (key, value) in env_set {
            let resolved = resolve_env_value(session, package, value);
            let os_value = std::ffi::OsString::from(resolved);
            internal::env::set_scoped(key, Some(&os_value), global)?;
            // Upstream also sets the current session (`Set-Content env:\`).
            std::env::set_var(key, &os_value);
            changed = true;
        }
    }

    // Add to PATH
    if let Some(env_add_path) = package.manifest().env_add_path() {
        let env_path_name = match config.use_isolated_path() {
            Some(config::IsolatedPath::Named(name)) => name.to_owned(),
            Some(config::IsolatedPath::Boolean(true)) => "SCOOP_PATH".to_owned(),
            _ => "PATH".to_owned(),
        };

        let mut paths = internal::env::get_path_like_env_scoped(&env_path_name, global)?;
        let app_base = install_app_base(session, package);

        for p in env_add_path {
            let path_entry = internal::path::normalize_path(app_base.join(p));
            // Upstream `is_in_dir`: entries escaping the app dir are dropped.
            if !path_within(&path_entry, &app_base) {
                continue;
            }
            if !paths.contains(&path_entry) {
                paths.push(path_entry);
                changed = true;
            }
        }

        if changed {
            let updated = internal::env::join_path_list(&paths)?;
            internal::env::set_scoped(&env_path_name, Some(&updated), global)?;
        }
    }

    if changed {
        internal::env::broadcast_env_change();
    }

    Ok(())
}

/// Remove the install directory (and anything under it) from `PATH` when a
/// custom installer put it there.
///
/// Mirrors upstream `ensure_install_dir_not_in_path`: the scoped PATH is
/// scrubbed and rewritten, while a non-global install that touched the
/// system PATH only earns a warning (removing that needs admin rights).
pub fn scrub_install_dir_from_path(session: &Session, dir: &std::path::Path) -> Fallible<()> {
    let global = session.config().is_global_scope();

    let current = internal::env::get_scoped_opt("PATH", global)?
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (fixed, removed) = split_dir_entries(&current, dir);
    if !removed.is_empty() {
        let fixed_os = std::ffi::OsString::from(fixed);
        internal::env::set_scoped("PATH", Some(&fixed_os), global)?;
        println!(
            "Installer added '{}' to path. Removing.",
            removed.join("; ")
        );
        internal::env::broadcast_env_change();
    }

    if !global {
        // Read-only probe of the system PATH: touching it here would need
        // admin rights, so only warn like upstream does.
        if let Ok(Some(system)) = internal::env::get_scoped_opt("PATH", true) {
            let (_, sys_removed) = split_dir_entries(&system.to_string_lossy(), dir);
            if !sys_removed.is_empty() {
                eprintln!(
                    "warning: installer added '{}' to system path. You might want to remove this manually (requires admin permission).",
                    sys_removed.join("; ")
                );
            }
        }
    }

    Ok(())
}

/// PATH variable names to scrub on removal: the default target plus the
/// isolated target when one is configured.
fn path_targets(config: &crate::config::Config) -> Vec<String> {
    let mut targets = vec!["PATH".to_owned()];
    let isolated = match config.use_isolated_path() {
        Some(config::IsolatedPath::Named(name)) => Some(name.to_owned()),
        Some(config::IsolatedPath::Boolean(true)) => Some("SCOOP_PATH".to_owned()),
        _ => None,
    };
    if let Some(name) = isolated {
        if name != "PATH" {
            targets.push(name);
        }
    }
    targets
}

/// The app base directory that `env_add_path` entries resolve against
/// (`apps/<name>/current`, or the version dir under `no_junction`).
fn install_app_base(session: &Session, package: &Package) -> std::path::PathBuf {
    let config = session.config();
    let mut app_path = config.root_path().join("apps");
    app_path.push(package.name());

    let version = if config.no_junction() {
        // Removal observes the installed directory (mirroring the
        // uninstaller's version-dir resolution); fresh installs have no
        // installed version yet and fall back to the manifest version.
        package
            .installed_version()
            .unwrap_or_else(|| package.version())
    } else {
        "current"
    };
    app_path.join(version)
}

/// Normalized `env_add_path` entries for a package, used by removal.
fn install_path_entries(session: &Session, package: &Package) -> Vec<std::path::PathBuf> {
    let app_base = install_app_base(session, package);
    // The installed manifest accessor is arch-resolved like the install side.
    match package.manifest().env_add_path() {
        Some(entries) => entries
            .into_iter()
            .map(|p| internal::path::normalize_path(app_base.join(p)))
            .collect(),
        None => Vec::new(),
    }
}

/// Whether `path` equals `dir` or lives under it (upstream `is_in_dir`,
/// whose `-eq`/`-like` comparisons are case-insensitive on Windows).
fn path_within(path: &std::path::Path, dir: &std::path::Path) -> bool {
    fn trimmed(s: &str) -> String {
        let mut s = s.to_owned();
        while s.ends_with(['/', '\\']) {
            s.pop();
        }
        s
    }

    let mut p = trimmed(&path.to_string_lossy());
    let mut d = trimmed(&dir.to_string_lossy());
    if cfg!(windows) {
        p = p.to_lowercase();
        d = d.to_lowercase();
    }
    p == d || p.starts_with(&format!("{d}/")) || p.starts_with(&format!("{d}\\"))
}

/// Split a `;`-separated path list into entries to keep versus entries that
/// equal `dir` or live under it.
fn split_dir_entries(path_value: &str, dir: &std::path::Path) -> (String, Vec<String>) {
    let mut fixed = Vec::new();
    let mut removed = Vec::new();
    for entry in path_value.split(';') {
        if entry.is_empty() {
            continue;
        }
        if path_within(std::path::Path::new(entry), dir) {
            removed.push(entry.to_owned());
        } else {
            fixed.push(entry);
        }
    }
    (fixed.join(";"), removed)
}

/// Hook-scope variables visible to `env_set` value expansion.
struct EnvScope {
    scoop: String,
    dir: String,
    original_dir: String,
    persist_dir: String,
    fname: String,
    bucketsdir: String,
    bucket: String,
    app: String,
    version: String,
    architecture: String,
    global: bool,
}

/// Resolve environment variable values that contain placeholders.
///
/// Mirrors upstream `$ExecutionContext.InvokeCommand.ExpandString`: the full
/// hook scope (`$dir`, `$version`, `$app`, `$architecture`, `$global`,
/// `$bucket`, `$bucketsdir`, `$fname`, `$original_dir`, `$persist_dir` in
/// `$name` and `${name}` forms) plus `$env:NAME` lookups expand; unknown
/// variables expand to the empty string. Non-variable `$` (e.g. `$5`) is
/// left alone.
fn resolve_env_value(session: &Session, package: &Package, value: &str) -> String {
    let config = session.config();
    let root = config.root_path();
    let mut app_path = root.join("apps");
    app_path.push(package.name());

    let scope = EnvScope {
        scoop: root.to_string_lossy().into_owned(),
        dir: install_app_base(session, package)
            .to_string_lossy()
            .into_owned(),
        original_dir: app_path
            .join(package.version())
            .to_string_lossy()
            .into_owned(),
        persist_dir: root
            .join("persist")
            .join(package.name())
            .to_string_lossy()
            .into_owned(),
        fname: package
            .download_staged_filenames()
            .into_iter()
            .next()
            .unwrap_or_default(),
        bucketsdir: root.join("buckets").to_string_lossy().into_owned(),
        bucket: package.bucket().to_owned(),
        app: package.name().to_owned(),
        version: package.version().to_owned(),
        architecture: crate::operation::resolved_arch(package),
        global: config.is_global_scope(),
    };
    expand_hook_vars(value, &scope)
}

/// Expand `$name`/`${name}` hook variables and `$env:NAME` lookups.
fn expand_hook_vars(value: &str, scope: &EnvScope) -> String {
    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default();

    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '$' {
            out.push(c);
            continue;
        }
        // Braced `${name}` form.
        let braced = chars.peek() == Some(&'{');
        if braced {
            chars.next();
            let mut token = String::new();
            for c in chars.by_ref() {
                if c == '}' {
                    break;
                }
                token.push(c);
            }
            out.push_str(&expand_token(&token, scope, &home));
            continue;
        }
        // Bare `$name` form: only identifier starts open a variable.
        match chars.peek() {
            Some(&n) if n.is_ascii_alphabetic() || n == '_' => {
                let mut token = String::new();
                while let Some(&n) = chars
                    .peek()
                    .filter(|n| n.is_ascii_alphanumeric() || **n == '_' || **n == ':')
                {
                    token.push(n);
                    chars.next();
                }
                out.push_str(&expand_token(&token, scope, &home));
            }
            _ => out.push('$'),
        }
    }
    out
}

/// Expand one variable token (without the leading `$`).
fn expand_token(token: &str, scope: &EnvScope, home: &str) -> String {
    if token.len() > 4 && token[..4].eq_ignore_ascii_case("env:") {
        let name = &token[4..];
        if name.eq_ignore_ascii_case("SCOOP") {
            // `$env:SCOOP` may not exist in the process block; the
            // session root is authoritative.
            return scope.scoop.clone();
        }
        if name.eq_ignore_ascii_case("HOME") || name.eq_ignore_ascii_case("USERPROFILE") {
            return home.to_owned();
        }
        return std::env::var(name).unwrap_or_default();
    }
    match token {
        "dir" => scope.dir.clone(),
        "original_dir" => scope.original_dir.clone(),
        "persist_dir" => scope.persist_dir.clone(),
        "fname" => scope.fname.clone(),
        "bucketsdir" => scope.bucketsdir.clone(),
        "bucket" => scope.bucket.clone(),
        "app" => scope.app.clone(),
        "version" => scope.version.clone(),
        "architecture" => scope.architecture.clone(),
        "global" => {
            if scope.global {
                "True".to_owned()
            } else {
                "False".to_owned()
            }
        }
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_scope() -> EnvScope {
        EnvScope {
            scoop: "C:/scoop".to_owned(),
            dir: "C:/scoop/apps/jdk/current".to_owned(),
            original_dir: "C:/scoop/apps/jdk/17".to_owned(),
            persist_dir: "C:/scoop/persist/jdk".to_owned(),
            fname: "jdk.zip".to_owned(),
            bucketsdir: "C:/scoop/buckets".to_owned(),
            bucket: "java".to_owned(),
            app: "jdk".to_owned(),
            version: "17".to_owned(),
            architecture: "64bit".to_owned(),
            global: false,
        }
    }

    #[test]
    fn env_values_expand_hook_scope() {
        let scope = test_scope();
        // The classic JDK shape: `$dir`-rooted `JAVA_HOME`.
        assert_eq!(
            expand_hook_vars("$dir", &scope),
            "C:/scoop/apps/jdk/current"
        );
        assert_eq!(
            expand_hook_vars("${dir}\\bin", &scope),
            "C:/scoop/apps/jdk/current\\bin"
        );
        assert_eq!(
            expand_hook_vars("$app-$version-$architecture-$bucket", &scope),
            "jdk-17-64bit-java"
        );
        assert_eq!(expand_hook_vars("$global", &scope), "False");
        assert_eq!(
            expand_hook_vars("$original_dir|$persist_dir|$fname|$bucketsdir", &scope),
            "C:/scoop/apps/jdk/17|C:/scoop/persist/jdk|jdk.zip|C:/scoop/buckets"
        );
    }

    #[test]
    fn env_expansion_handles_env_lookups_and_unknowns() {
        let scope = test_scope();
        std::env::set_var("BAGGER_TEST_PROBE", "probed");
        assert_eq!(
            expand_hook_vars("$env:BAGGER_TEST_PROBE!", &scope),
            "probed!"
        );
        std::env::remove_var("BAGGER_TEST_PROBE");
        // Unknown variables expand to empty (upstream `ExpandString`); a
        // `$` that cannot start a variable stays literal.
        assert_eq!(expand_hook_vars("a$nope_var b", &scope), "a b");
        assert_eq!(expand_hook_vars("price: $5", &scope), "price: $5");
        assert_eq!(expand_hook_vars("$env:BAGGER_TEST_MISSING", &scope), "");
    }

    #[test]
    fn dir_containment_matches_upstream_find_dir_or_subdir() {
        let dir = std::path::Path::new("C:/scoop/apps/foo/1.0");
        assert!(path_within(dir, dir));
        assert!(path_within(
            std::path::Path::new("C:/scoop/apps/foo/1.0/bin"),
            dir
        ));
        // A sibling whose name merely extends the prefix is not inside.
        assert!(!path_within(
            std::path::Path::new("C:/scoop/apps/foo/1.0-evil"),
            dir
        ));
        assert!(!path_within(std::path::Path::new("C:/other"), dir));

        let (fixed, removed) = split_dir_entries(
            "C:\\keep;C:\\scoop\\apps\\foo\\1.0;C:\\scoop\\apps\\foo\\1.0\\bin;;C:\\keep2",
            std::path::Path::new("C:\\scoop\\apps\\foo\\1.0"),
        );
        assert_eq!(fixed, "C:\\keep;C:\\keep2");
        assert_eq!(
            removed,
            vec![
                "C:\\scoop\\apps\\foo\\1.0".to_owned(),
                "C:\\scoop\\apps\\foo\\1.0\\bin".to_owned()
            ]
        );
    }

    /// Restores the user PATH on drop so a panic cannot pollute the registry.
    struct RestoreUserPath(Option<std::ffi::OsString>);
    impl Drop for RestoreUserPath {
        fn drop(&mut self) {
            let value = self.0.take();
            let _ = internal::env::set_scoped("PATH", value.as_ref(), false);
            internal::env::broadcast_env_change();
        }
    }

    /// The scrub removes the install dir (and subdirs) from the user PATH
    /// while keeping everything else, then restores the original value.
    #[test]
    #[cfg(windows)]
    fn scrub_removes_install_dir_from_user_path() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-env-scrub");
        let _ = std::fs::remove_dir_all(&base);
        std::env::set_var("SCOOP", base.join("root"));
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));
        let session = Session::new();

        let original = internal::env::get_scoped_opt("PATH", false)
            .unwrap()
            .unwrap_or_default();
        let _restore = RestoreUserPath(Some(original.clone()));

        let victim = base.join("root/apps/scrubapp/1.0");
        std::fs::create_dir_all(victim.join("bin")).unwrap();
        let mut with_victim = original.to_string_lossy().into_owned();
        with_victim.push_str(&format!(
            ";{};{}",
            victim.display(),
            victim.join("bin").display()
        ));
        internal::env::set_scoped("PATH", Some(&std::ffi::OsString::from(&with_victim)), false)
            .unwrap();

        scrub_install_dir_from_path(&session, &victim).unwrap();

        let after = internal::env::get_scoped_opt("PATH", false)
            .unwrap()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        assert!(!after.contains("scrubapp"), "scrubbed PATH: {after}");
        // Every pre-existing entry survives (empty segments aside, which
        // upstream drops when rewriting the list).
        for entry in original.to_string_lossy().split(';') {
            if entry.is_empty() {
                continue;
            }
            assert!(
                after.split(';').any(|kept| kept == entry),
                "lost PATH entry: {entry}"
            );
        }

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }
}
