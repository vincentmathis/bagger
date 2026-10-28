use bagger_hash::ChecksumBuilder;
use once_cell::unsync::OnceCell;
use std::collections::HashSet;
use std::io::Read;
use tracing::{debug, info};

use crate::{
    env, error::Fallible, internal, persist, psmodule, shim, shortcut, Error, Event, QueryOption,
    Session,
};

use super::{
    download::{self, DownloadSize},
    is_dated_nightly,
    manifest::{InstallInfo, Manifest},
    nightly_version, query, resolve, InstallState, InstallStateInstalled, Package,
};

/// Options that may be used to tweak behavior of package sync operation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum SyncOption {
    /// Assume YES on all prompts.
    ///
    /// # Note
    ///
    /// This option will also suppress the prompt for package candidate selection.
    /// A built-in candidate selection algorithm will be used to select the
    /// proper candidate. This may not be the desired behavior in some cases.
    ///
    /// Enabling this option will also suppress the calculation of download size.
    AssumeYes,

    /// Download package only.
    ///
    /// # Note
    ///
    /// To sync packages by just downloading and caching them without installing
    /// or upgrading, this option can be used. Transcation will be stopped after
    /// the download is done.
    DownloadOnly,

    /// Force operations on held packages.
    ///
    /// # Note
    ///
    /// Held packages are ignored during the replace, upgrade or uninstall
    /// operations by default. The option can be used to escape the hold and
    /// enforce operations on the held packages.
    ///
    /// Packages will be held again after the replace or upgrade operation.
    EscapeHold,

    /// Ignore local cache and force package download.
    ///
    /// # Note
    ///
    /// This option is not intended to be used with the [`Offline`][1]
    /// option.
    ///
    /// [1]: SyncOption::Offline
    IgnoreCache,

    /// Do not install dependencies.
    ///
    /// # Note
    ///
    /// By default, dependencies of the pending installation package will be
    /// resolved and installed **recursively** if they are not installed yet.
    /// One can opt in this option to disable the default behavior. However,
    /// it is not recommended to do so since it clearly breaks the dependency
    /// relationship, and may stop the dependents from working properly.
    NoDependencies,

    /// Stop checking hash of downloaded packages.
    ///
    /// # Note
    ///
    /// Integrity check helps to ensure the downloaded packages are not corrupted
    /// or tampered. Hash check will be performed by default. In some cases, user
    /// may want to skip the check to force the installation or upgrade of the
    /// packages. By opting in this option, the hash check will be skipped.
    ///
    /// It is highly **NOT** recommended to use this option unless you really
    /// know what you are doing.
    NoHashCheck,

    /// Do not upgrade packages.
    ///
    /// This option is not intended to be used with the [`OnlyUpgrade`][1] option.
    ///
    /// [1]: SyncOption::OnlyUpgrade
    NoUpgrade,

    /// Do not replace packages.
    ///
    /// # Note
    ///
    /// When a package is installed and a same-named package is proposed to be
    /// installed, a replace operation will be performed if the proposed package
    /// is from a different bucket from the installed one.
    ///
    /// By opting in this option, the replace operation will be suppressed.
    NoReplace,

    /// Offline mode.
    ///
    /// # Note
    ///
    /// This option is useful when user wants to install or upgrade packages
    /// with existing local cached packages. By opting in this option and having
    /// valid caches prepared, network access can be avoided to perform the sync
    /// operation. However, the transaction may fail if there is any package file
    /// missing or invalid cache.
    ///
    /// This option is basically the opposite of the [`IgnoreCache`][1] option.
    ///
    /// [1]: SyncOption::IgnoreCache
    Offline,

    /// Upgrade packages only.
    ///
    /// Use this option to specify a sync operation of only upgrading packages.
    ///
    /// This option is not intended to be used with the [`NoUpgrade`][1] option.
    ///
    /// [1]: SyncOption::NoUpgrade
    OnlyUpgrade,

    /// Force reinstall even when the installed version is current.
    ///
    /// Mirrors upstream `update -f`: packages whose bucket version matches
    /// the installed one are reinstalled, with the previous version
    /// directory rotated aside to `_<version>.old`.
    Force,

    /// Uninstall packages.
    ///
    /// Use this option to specify a sync operation of only uninstalling packages.
    Remove,

    /// Purge uninstall.
    ///
    /// # Note
    ///
    /// By enabling this option, persistent data of the pending removal packages
    /// will be removed simultaneously.
    ///
    /// This option only takes effect with the [`Remove`][1] option.
    ///
    /// [1]: SyncOption::Remove
    Purge,

    /// Cascade uninstall.
    ///
    /// # Note
    ///
    /// By opt in this option, dependencies of the pending removal package
    /// will also be removed **recursively** if they are not required by other
    /// installed packages.
    ///
    /// This option only takes effect with the [`Remove`][1] option.
    ///
    /// [1]: SyncOption::Remove
    Cascade,

    /// Disable dependent check.
    ///
    /// # Note
    ///
    /// By default, a reverse dependencies check will be performed on the pending
    /// removal package. If any installed package depends on the pending removal
    /// package, the removal operation will be aborted.
    ///
    /// The default behavior can be modified by opting in this option, however,
    /// it is not recommended to do so since it clearly breaks the dependency
    /// relationship, and may stop the dependents from working properly.
    ///
    /// This option only takes effect with the [`Remove`][1] option.
    ///
    /// [1]: SyncOption::Remove
    NoDependentCheck,
}

/// Transaction of sync operation.
///
/// # Note
///
/// A transaction is a set of packages that will be installed, upgraded, replaced
/// or removed. The transaction is calculated by the sync operation and can be
/// used to prompt the user to confirm the operation.
#[derive(Clone)]
pub struct Transaction {
    /// Packages that will be installed with the transaction.
    install: OnceCell<Vec<Package>>,

    /// Packages that will be upgraded with the transaction.
    upgrade: OnceCell<Vec<Package>>,

    /// Packages that will be replaced with the transaction.
    replace: OnceCell<Vec<Package>>,

    /// Packages that will be removed with the transaction.
    remove: OnceCell<Vec<Package>>,

    /// Total download size of the transaction.
    download_size: OnceCell<DownloadSize>,
}

impl Transaction {
    fn new() -> Transaction {
        Transaction {
            install: OnceCell::new(),
            upgrade: OnceCell::new(),
            replace: OnceCell::new(),
            remove: OnceCell::new(),
            download_size: OnceCell::new(),
        }
    }

    fn set_install(&self, packages: Vec<Package>) {
        let _ = self.install.set(packages);
    }

    fn set_upgrade(&self, packages: Vec<Package>) {
        let _ = self.upgrade.set(packages);
    }

    fn set_replace(&self, packages: Vec<Package>) {
        let _ = self.replace.set(packages);
    }

    fn set_remove(&self, packages: Vec<Package>) {
        let _ = self.remove.set(packages);
    }

    fn set_download_size(&self, download_size: DownloadSize) -> bool {
        self.download_size.set(download_size).is_ok()
    }

    fn add_view(&self) -> Vec<&Package> {
        self.install_view()
            .into_iter()
            .chain(self.upgrade_view())
            .chain(self.replace_view())
            .flatten()
            .collect::<Vec<_>>()
    }

    /// Get packages that will be installed with the transaction.
    ///
    /// # Returns
    ///
    /// A reference to the vector of packages that will be installed or `None`
    /// if no packages will be installed.
    pub fn install_view(&self) -> Option<&Vec<Package>> {
        self.install.get()
    }

    /// Get packages that will be upgraded with the transaction.
    ///
    /// # Returns
    ///
    /// A reference to the vector of packages that will be upgraded or `None`
    /// if no packages will be upgraded.
    pub fn upgrade_view(&self) -> Option<&Vec<Package>> {
        self.upgrade.get()
    }

    /// Get packages that will be replaced with the transaction.
    ///
    /// # Returns
    ///
    /// A reference to the vector of packages that will be replaced or `None`
    /// if no packages will be replaced.
    pub fn replace_view(&self) -> Option<&Vec<Package>> {
        self.replace.get()
    }

    /// Get packages that will be removed with the transaction.
    ///
    /// # Returns
    ///
    /// A reference to the vector of packages that will be removed or `None`
    /// if no packages will be removed.
    pub fn remove_view(&self) -> Option<&Vec<Package>> {
        self.remove.get()
    }

    /// Get the total download size of the transaction.
    ///
    /// # Returns
    ///
    /// A `DownloadSize` reference that contains the total download size of the
    /// transaction.
    pub fn download_size(&self) -> Option<&DownloadSize> {
        self.download_size.get()
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

/// Split packages into committable ones and ones blocked by running
/// processes.
///
/// With `ignore_running_processes` set, nothing is blocked and the input is
/// returned unchanged.
fn partition_runnable(
    session: &Session,
    packages: Vec<Package>,
) -> (Vec<Package>, Vec<(String, String)>) {
    if session.config().ignore_running_processes() {
        return (packages, vec![]);
    }

    let apps_dir = session.config().root_path().join("apps");
    let mut runnable = Vec::with_capacity(packages.len());
    let mut blocked = vec![];
    for pkg in packages {
        match internal::os::running_apps(&apps_dir.join(pkg.name())) {
            Ok(procs) if !procs.is_empty() => {
                blocked.push((pkg.name().to_owned(), procs.join(", ")))
            }
            _ => runnable.push(pkg),
        }
    }
    (runnable, blocked)
}

/// Drop blocked packages from the transaction, notifying the frontend.
///
/// A transaction left with nothing runnable fails with the running-process
/// error (this is also what an explicitly requested single package gets);
/// otherwise the blocked entries are skipped so the rest can proceed.
fn filter_running(session: &Session, packages: Vec<Package>) -> Fallible<Vec<Package>> {
    let (runnable, blocked) = partition_runnable(session, packages);
    for (name, processes) in &blocked {
        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageRunningSkipped {
                name: name.clone(),
                processes: processes.clone(),
            });
        }
    }
    if runnable.is_empty() {
        if let Some((name, processes)) = blocked.into_iter().next() {
            return Err(Error::PackageRunningProcesses(name, processes));
        }
    }
    Ok(runnable)
}

/// Whether a single package is currently blocked by running processes.
///
/// Re-checked at commit time to close the gap between resolving and
/// committing (a process may have started while downloading).
fn running_blocked(session: &Session, package: &Package) -> Option<String> {
    if session.config().ignore_running_processes() {
        return None;
    }
    let app_path = session
        .config()
        .root_path()
        .join("apps")
        .join(package.name());
    match internal::os::running_apps(&app_path) {
        Ok(procs) if !procs.is_empty() => Some(procs.join(", ")),
        _ => None,
    }
}

/// Sync operation: install and/or upgrade packages.
///
/// Returns whether any package was committed (an empty transaction —
/// already installed, nothing outdated — returns `false` so frontends
/// can report it instead of going silent).
pub fn install(session: &Session, queries: &[&str], options: &[SyncOption]) -> Fallible<bool> {
    let mut packages = vec![];

    let only_upgrade = options.contains(&SyncOption::OnlyUpgrade);
    let escape_hold = options.contains(&SyncOption::EscapeHold);
    let force = options.contains(&SyncOption::Force);
    let update_nightly = session.config().update_nightly();
    // First resolution failure, deferred so the remaining names still
    // resolve and install (one bad name must not sink the batch).
    let mut unresolved: Option<Error> = None;

    if only_upgrade {
        // Exact matching: `upgrade git` must not sweep up `gitui` or
        // `git-lfs` (regex substring matching upgraded the wrong apps).
        packages = query::query_installed(
            session,
            queries,
            &[QueryOption::Upgradable, QueryOption::Explicit],
        )?;

        // Replace the packages with their upgradable references.
        packages = packages
            .into_iter()
            .map(|p| p.upgradable().cloned().unwrap())
            .collect::<Vec<_>>();
    } else {
        let synced = query::query_synced(session, &["*"], &[])?;

        for &query in queries {
            // `app@version` pins resolve to generated/historical manifests
            // before the normal flows run.
            let (base, pinned) = query::split_version_query(query);
            if let Some(version) = pinned {
                // Held pins without hold escape resolve to `None` and are
                // skipped like the other flows.
                match query::resolve_version_pin(session, &base, &version, escape_hold) {
                    Ok(Some(pkg)) => {
                        if !packages.contains(&pkg) {
                            packages.push(pkg);
                        }
                    }
                    Ok(None) => {}
                    Err(e) => defer_unresolved(&mut unresolved, query, e),
                }
                continue;
            }

            // Manifest URLs and local manifest files install as isolated
            // packages without requiring a bucket.
            match query::load_isolated_package(session, query) {
                Ok(Some(p)) => {
                    if p.is_held() && !escape_hold {
                        continue;
                    }

                    if !packages.contains(&p) {
                        packages.push(p);
                    }
                    continue;
                }
                Ok(None) => {}
                Err(e) => {
                    defer_unresolved(&mut unresolved, query, e);
                    continue;
                }
            }

            let mut matched = synced
                .iter()
                .filter(|&p| {
                    let (query_bucket, query_name) = query.split_once('/').unwrap_or(("", query));
                    let bucket_matched = query_bucket.is_empty() || p.bucket() == query_bucket;
                    // App lookup is case-insensitive, like Windows file
                    // names and upstream `-match`.
                    let name_matched = p.name().eq_ignore_ascii_case(query_name);
                    bucket_matched && name_matched
                })
                .cloned()
                .collect::<Vec<_>>();

            match matched.len() {
                0 => defer_unresolved(
                    &mut unresolved,
                    query,
                    Error::PackageNotFound(query.to_owned()),
                ),
                1 => {
                    let p = matched.pop().unwrap();

                    if p.is_held() && !escape_hold {
                        // Skipping held package returns nothing to frontend...
                        continue;
                    }

                    if !packages.contains(&p) {
                        packages.push(p);
                    }
                }
                _ => {
                    let is_held = matched.iter().any(|p| p.is_held());

                    if is_held && !escape_hold {
                        continue;
                    }

                    let assume_yes = options.contains(&SyncOption::AssumeYes);
                    resolve::select_candidate(session, &mut matched, assume_yes)?;
                    let p = matched.pop().unwrap();
                    if !packages.contains(&p) {
                        packages.push(p);
                    }
                }
            }
        }
    };

    // A force reinstall (or nightly redating) may proceed with an empty
    // resolve set (the reinstall candidates are collected below);
    // anything still empty is caught by the post-transaction early return
    // instead.
    if packages.is_empty() && !force && !update_nightly {
        if let Some(e) = unresolved {
            return Err(e);
        }

        return Ok(false);
    }

    let transaction = Transaction::default();

    let no_dependencies = options.contains(&SyncOption::NoDependencies);
    if !no_dependencies {
        let assume_yes = options.contains(&SyncOption::AssumeYes);
        resolve::resolve_dependencies(session, &mut packages, assume_yes)?;
    }

    // Drop packages blocked by running processes before building the
    // transaction, so the confirmation prompt and downloads only cover
    // what will actually be committed.
    let packages = filter_running(session, packages)?;

    // Upstream aborts installs whose manifest version contains characters
    // outside `[\w.\-+_]`.
    for pkg in packages.iter() {
        if !valid_manifest_version(pkg.version()) {
            return Err(Error::Custom(format!(
                "manifest version '{}' of '{}' has unsupported characters (allowed: letters, digits, '.', '-', '+', '_')",
                pkg.version(),
                pkg.name()
            )));
        }
    }

    let (installed, installable): (Vec<_>, Vec<_>) = packages
        .into_iter()
        // Stamp nightly manifests first, so downloads, cache names,
        // staging, and directories all agree on the dated version.
        .map(|p| {
            if p.manifest().version() == "nightly" {
                p.with_version(&nightly_version())
            } else {
                p
            }
        })
        .partition(|p| p.is_installed());

    // Snapshot installed names/versions for the already-installed
    // warnings below (the partition consumes the packages).
    let installed_snapshot: Vec<(String, String)> = installed
        .iter()
        .map(|p| {
            (
                p.name().to_owned(),
                p.installed_version().unwrap_or(p.version()).to_owned(),
            )
        })
        .collect();

    let (upgradable, replaceable): (Vec<_>, Vec<_>) = installed
        .into_iter()
        .partition(|p| p.is_strictly_installed());

    if !only_upgrade && !installable.is_empty() {
        transaction.set_install(installable);
    }

    let mut upgradable: Vec<_> = upgradable
        .into_iter()
        .filter(|p| {
            // Dated nightly builds never upgrade through the version
            // comparison; the gated block below owns them.
            p.upgradable_version().is_some() && !is_dated_nightly(p.version())
        })
        .collect();

    if (force || update_nightly) && only_upgrade {
        // Upstream `update -f` reinstalls the requested apps even when the
        // bucket version is not newer, and nightly apps are redated when a
        // new day dawned (gated by `update_nightly`, like upstream's
        // `Compare-Version`). Holds are still respected.
        let today = nightly_version();
        let current = query::query_installed(session, queries, &[])?;
        let synced = query::query_synced(session, &["*"], &[])?;
        for inst in current {
            if inst.is_held() && !escape_hold {
                continue;
            }
            if upgradable.iter().any(|p| p.name() == inst.name()) {
                continue;
            }
            if !inst.is_strictly_installed() {
                continue;
            }
            let bucket = inst.installed_bucket().unwrap_or_default();
            let origin = synced
                .iter()
                .find(|s| s.name() == inst.name() && s.bucket() == bucket);
            let Some(origin) = origin else {
                continue;
            };
            let wanted = if origin.version() == "nightly" {
                force || (update_nightly && inst.installed_version() != Some(today.as_str()))
            } else {
                force && Some(origin.version()) == inst.installed_version()
            };
            if wanted {
                upgradable.push(origin.clone());
            }
        }
    }

    let no_upgrade = options.contains(&SyncOption::NoUpgrade);
    if !no_upgrade && !upgradable.is_empty() {
        if !escape_hold {
            let (_held, upgradable): (Vec<_>, Vec<_>) =
                upgradable.into_iter().partition(|p| p.is_held());

            if !upgradable.is_empty() {
                transaction.set_upgrade(upgradable);
            }
        } else {
            transaction.set_upgrade(upgradable);
        }
    }

    let no_replace = options.contains(&SyncOption::NoReplace);
    if !no_replace && !replaceable.is_empty() {
        transaction.set_replace(replaceable);
    }

    // Upstream prune warnings: explicitly requested apps that are already
    // installed are skipped, not reinstalled.
    if !only_upgrade {
        if let Some(tx) = session.emitter() {
            let acted: std::collections::HashSet<&str> =
                transaction.add_view().iter().map(|p| p.name()).collect();
            for (name, version) in &installed_snapshot {
                if acted.contains(name.as_str()) {
                    continue;
                }
                // Only explicitly requested apps warn (wildcards, buckets,
                // and dependencies never do), mirroring upstream exactly.
                if queries.contains(&name.as_str()) {
                    let _ = tx.send(Event::PackageAlreadyInstalled {
                        name: name.clone(),
                        version: version.clone(),
                    });
                }
            }
        }
    }

    let reuse_cache = !options.contains(&SyncOption::IgnoreCache);

    let packages = transaction.add_view();
    if packages.is_empty() {
        if let Some(e) = unresolved {
            return Err(e);
        }

        return Ok(false);
    }

    let assume_yes = options.contains(&SyncOption::AssumeYes);
    let offline = options.contains(&SyncOption::Offline);
    let mut should_offline = true;

    if !offline {
        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageDownloadSizingStart);
        }

        // One unreachable host must not abort the whole batch: size each
        // package independently and tolerate failures.
        let mut total = 0u64;
        let mut estimated = false;
        for pkg in packages.iter() {
            let one_pkg: &[&Package] = std::slice::from_ref(pkg);
            let mut one = if let Ok(s) = download::PackageSet::new(session, one_pkg, reuse_cache) {
                s
            } else {
                continue;
            };
            if let Ok(ds) = one.calculate_download_size() {
                total += ds.total;
                estimated |= ds.estimated;
            }
        }
        let download_size = download::DownloadSize { total, estimated };
        // Only skip downloading when every size is known (not estimated)
        // and nothing is missing: servers that omit Content-Length (or
        // non-HTTP URLs like file://) report zero sizes, which must still
        // be downloaded.
        should_offline = download_size.total == 0 && !download_size.estimated;
        transaction.set_download_size(download_size);
    }

    if !assume_yes {
        if let Some(tx) = session.emitter() {
            if tx
                .send(Event::PromptTransactionNeedConfirm(transaction.clone()))
                .is_ok()
            {
                let rx = session.receiver().unwrap();
                let mut confirmed = false;

                while let Ok(event) = rx.recv() {
                    if let Event::PromptTransactionNeedConfirmResult(ret) = event {
                        confirmed = ret;
                        break;
                    }
                }

                if !confirmed {
                    return Ok(false);
                }
            }
        }
    }

    // First failure across the download, integrity-check, and commit phases.
    // It is returned only after the remaining packages had their turn, so a
    // single bad package cannot sink the whole batch.
    let mut deferred: Option<Error> = None;
    // Packages that must not be committed: their download or integrity check
    // already failed, so there is nothing trustworthy to install.
    let mut failed: HashSet<String> = HashSet::new();

    if !should_offline {
        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageDownloadStart);
        }

        // Download packages one at a time so a single unreachable URL
        // doesn't abort the whole batch.
        for pkg in packages.iter() {
            let one_pkg: &[&Package] = std::slice::from_ref(pkg);
            let mut one = match download::PackageSet::new(session, one_pkg, reuse_cache) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!(
                        "warning: failed to download '{}': {e}; continuing with the rest",
                        pkg.name()
                    );
                    failed.insert(pkg.name().to_owned());
                    if deferred.is_none() {
                        deferred = Some(e);
                    }
                    continue;
                }
            };
            if let Err(e) = one.download() {
                eprintln!(
                    "warning: failed to download '{}': {e}; continuing with the rest",
                    pkg.name()
                );
                failed.insert(pkg.name().to_owned());
                if deferred.is_none() {
                    deferred = Some(e);
                }
            }
        }

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageDownloadDone);
        }
    }

    let no_hash_check = options.contains(&SyncOption::NoHashCheck);
    if !no_hash_check {
        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageIntegrityCheckStart);
        }

        let config = session.config();
        let cache_root = config.cache_path();

        let mut buf = [0; 1024 * 64];

        for &pkg in packages.iter() {
            // Its download already failed; there is no file to verify, and
            // committing it would fail too.
            if failed.contains(pkg.name()) {
                continue;
            }

            // Nightly builds skip hash checks (the dated stamp is
            // covered too, since stamping happens before this phase).
            if pkg.version() == "nightly" || is_dated_nightly(pkg.version()) {
                info!("skip hash check for nightly package '{}'", pkg.name());
                continue;
            }

            // One package's verification runs in a closure so a missing
            // cache file or a hash mismatch skips that package only.
            let result = (|| -> Fallible<()> {
                let files = pkg.download_filenames();
                let hashes = pkg.download_hashes();
                if hashes.is_empty() {
                    // Upstream prints the computed SHA256 for hash-less
                    // manifests so authors can fill it in.
                    for filename in files.iter() {
                        let path = cache_root.join(filename);
                        let mut hasher = ChecksumBuilder::new().sha256().build();
                        let mut file = std::fs::File::open(&path)
                            .map_err(|e| Error::invalid_cache_file(&path, &e))?;
                        loop {
                            let len = file.read(&mut buf)?;
                            if len == 0 {
                                break;
                            }
                            hasher.consume(&buf[..len]);
                        }
                        eprintln!(
                            "warning: no hash in manifest for '{}'. SHA256 for '{}' is:\n    {}",
                            pkg.name(),
                            path.file_name().unwrap_or_default().to_string_lossy(),
                            hasher.finalize()
                        );
                    }
                    return Ok(());
                }
                let files_cnt = files.len();

                for (idx, (filename, hash)) in files.into_iter().zip(hashes).enumerate() {
                    let path = cache_root.join(filename);

                    let mut hasher = ChecksumBuilder::new().algo(hash.algorithm())?.build();

                    if let Some(tx) = session.emitter() {
                        let progress = format!("{} ({}/{})", pkg.name(), idx + 1, files_cnt);
                        let _ = tx.send(Event::PackageIntegrityCheckProgress(progress));
                    }

                    let mut file = std::fs::File::open(&path)
                        .map_err(|e| Error::invalid_cache_file(&path, &e))?;
                    loop {
                        let len = file.read(&mut buf)?;
                        if len == 0 {
                            break;
                        }
                        hasher.consume(&buf[..len]);
                    }

                    let actual = hasher.finalize();
                    let expected = hash.value();
                    if actual != expected {
                        let name = pkg.name().to_owned();
                        let url = pkg.download_urls()[idx].to_owned();
                        // Drop the bad file so the next run re-downloads
                        // instead of failing the same integrity check forever.
                        let _ = std::fs::remove_file(&path);
                        eprintln!(
                            "warning: removed corrupt cache file '{}'; it will be re-downloaded",
                            path.display()
                        );
                        let ctx =
                            super::HashMismatchContext::new(name, url, expected.to_owned(), actual);
                        return Err(Error::HashMismatch(ctx));
                    }
                }
                Ok(())
            })();

            if let Err(e) = result {
                eprintln!(
                    "warning: integrity check failed for '{}': {e}; continuing with the rest",
                    pkg.name()
                );
                failed.insert(pkg.name().to_owned());
                if deferred.is_none() {
                    deferred = Some(e);
                }
            }
        }

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageIntegrityCheckDone);
        }
    }

    let download_only = options.contains(&SyncOption::DownloadOnly);
    if !download_only {
        let config = session.config();
        let apps_dir = config.root_path().join("apps");

        // Log packages requiring PowerShell scripts
        let script_pkgs = packages
            .iter()
            .filter(|p| p.has_install_script())
            .map(|p| p.name().to_owned())
            .collect::<Vec<_>>();
        if !script_pkgs.is_empty() {
            info!(
                "packages requiring PowerShell scripts: {}",
                script_pkgs.join(", ")
            );
        }

        for pkg in packages.iter() {
            // Its download or integrity check failed; nothing to install.
            if failed.contains(pkg.name()) {
                continue;
            }

            // Nightly manifests install under a dated version (upstream
            // `nightly_version`), so each day gets a fresh directory. The
            // stamped manifest is saved as-is, making the installed version
            // self-describing like upstream's link-target convention.
            let stamped;
            let pkg = match pkg.manifest().version() == "nightly" {
                true => {
                    stamped = pkg.with_version(&nightly_version());
                    &stamped
                }
                false => pkg,
            };

            // One package's commit runs in a closure so its error can be
            // collected and the rest of the batch can continue instead of
            // aborting the transaction (`?` returns from here, and the
            // skip below returns success for this package only).
            let result = (|| -> Fallible<()> {
                // Re-check at commit time: a process may have started while
                // downloading. Skip it rather than failing the transaction.
                if let Some(processes) = running_blocked(session, pkg) {
                    if let Some(tx) = session.emitter() {
                        let _ = tx.send(Event::PackageRunningSkipped {
                            name: pkg.name().to_owned(),
                            processes,
                        });
                    }
                    return Ok(());
                }

                if let Some(tx) = session.emitter() {
                    let _ = tx.send(Event::PackageCommitStart(pkg.name().to_owned()));
                }

                let working_dir = apps_dir.join(pkg.name()).join(pkg.version());
                // A previous failed attempt leaves a metadata-less directory
                // behind; purge it before staging (upstream `ensure_none_failed`).
                // Mutually exclusive with the force rotation below, which needs
                // the markers to be present.
                if purge_failed_dir(&apps_dir.join(pkg.name()), pkg.version())? {
                    info!(
                        "purged previous failed install of '{}' ({})",
                        pkg.name(),
                        pkg.version()
                    );
                }
                if force && super::has_install_metadata(&working_dir) {
                    // Upstream `update -f`: rotate the previous install aside
                    // instead of merging the reinstall into it.
                    let backup = rotate_version_dir(&apps_dir.join(pkg.name()), pkg.version())?;
                    info!(
                        "moved previous install of '{}' aside to '{}'",
                        pkg.name(),
                        backup.display()
                    );
                }
                internal::fs::ensure_dir(&working_dir)?;

                let filenames = pkg.download_filenames();
                let staged = pkg.download_staged_filenames();

                let cache_root = config.cache_path();

                // Stage downloads under their real URL basenames (upstream
                // `url_filename`; `#/...` fragments coerce the name), so hook
                // scripts, installer files, and shim targets observe the names
                // manifests were written against.
                for (filename, staged_name) in filenames.iter().zip(staged.iter()) {
                    let src = cache_root.join(filename);
                    if !src.exists() {
                        continue;
                    }
                    let dst = working_dir.join(staged_name);
                    let _ = std::fs::remove_file(&dst);
                    std::fs::copy(&src, &dst)?;
                }

                // Standard extraction first, matching upstream Scoop
                // (`Invoke-Extraction` runs before any hook script and simply
                // skips files without a known extractor, leaving those for the
                // installer file/script below).
                extract_package(session, pkg, &working_dir)?;

                // Run pre_install after extraction (upstream order): hook
                // scripts such as `Expand-7zipArchive "$dir\inner.7z"` operate
                // on the extracted tree.
                if let Some(pre_install) = pkg.manifest().pre_install() {
                    internal::ps::invoke_script(
                        session,
                        pkg,
                        "install",
                        &pre_install,
                        &working_dir,
                    )?;
                }

                // Run installer file and/or script if present. Upstream runs
                // the downloaded file (first URL basename) when `args` is
                // given without `file`, so the call is unconditional: it
                // no-ops when the section carries neither.
                if let Some(installer) = pkg.manifest().installer() {
                    run_installer_file(
                        session,
                        pkg,
                        &working_dir,
                        installer.file(),
                        installer.args(),
                        installer.keep(),
                        false,
                    )?;
                    if let Some(script) = installer.script() {
                        internal::ps::invoke_script(
                            session,
                            pkg,
                            "install",
                            &script,
                            &working_dir,
                        )?;
                    }
                }

                // Installers may register the app dir on PATH themselves;
                // upstream scrubs those entries so the manifest stays in
                // control of the environment.
                crate::env::scrub_install_dir_from_path(session, &working_dir)?;

                // Create the 'current' symlink if not using no_junction
                if !config.no_junction() {
                    let current_link = apps_dir.join(pkg.name()).join("current");
                    internal::fs::remove_symlink(&current_link)?;
                    internal::fs::symlink_dir(&working_dir, &current_link)?;
                }

                let install_subdir = if config.no_junction() {
                    pkg.version().to_string()
                } else {
                    "current".to_string()
                };
                let install_base = apps_dir.join(pkg.name()).join(&install_subdir);

                // Save the installed manifest under both names (upstream
                // `scoop-manifest.json`, legacy `manifest.json`).
                let manifest_json = serde_json::to_string_pretty(pkg.manifest().inner())?;
                std::fs::write(install_base.join("manifest.json"), &manifest_json)?;
                std::fs::write(install_base.join("scoop-manifest.json"), &manifest_json)?;

                // Write install info under both names (upstream
                // `scoop-install.json`, legacy `install.json`). Isolated
                // packages record no bucket so they keep the `__isolated__`
                // marker on later queries. The `url` mirrors upstream
                // semantics — the manifest source for isolated installs,
                // absent for bucket installs. It must never be a download
                // URL: upstream `update` mistakes that for a manifest and
                // fails the reinstall.
                let isolated = pkg.bucket() == crate::constant::ISOLATED_PACKAGE_BUCKET;
                let install_bucket = match isolated {
                    true => None,
                    false => Some(pkg.bucket().to_owned()),
                };
                let install_url = match isolated {
                    true => Some(pkg.manifest().path().to_string_lossy().into_owned()),
                    false => None,
                };
                let install_info = InstallInfo::new(
                    crate::operation::resolved_arch(pkg),
                    install_bucket,
                    install_url,
                );
                internal::fs::write_json(install_base.join("install.json"), &install_info)?;
                internal::fs::write_json(install_base.join("scoop-install.json"), &install_info)?;

                // Set up persist directories
                persist::link(session, pkg)?;

                // Upstream `persist_permission`: global installs running
                // elevated grant Users write access to the persist root so
                // non-admin users can use the persisted data.
                if session.config().is_global_scope()
                    && pkg.manifest().persist().is_some()
                    && internal::os::is_elevated()
                {
                    persist::grant_users_write(&session.config().root_path().join("persist"))?;
                }

                // Add shims
                shim::add(session, pkg)?;

                // Add shortcuts
                shortcut::add(session, pkg)?;

                // Import PowerShell modules
                psmodule::add(session, pkg)?;

                // Set environment variables and path
                env::install(session, pkg)?;

                // Run post_install script if present
                if let Some(post_install) = pkg.manifest().post_install() {
                    let post_install_dir = if config.no_junction() {
                        apps_dir.join(pkg.name()).join(pkg.version())
                    } else {
                        apps_dir.join(pkg.name()).join("current")
                    };
                    internal::ps::invoke_script(
                        session,
                        pkg,
                        "install",
                        &post_install,
                        &post_install_dir,
                    )?;
                }

                if let Some(tx) = session.emitter() {
                    // Substitute `$dir`/`$original_dir`/`$persist_dir` in notes,
                    // mirroring upstream `show_notes`.
                    let dir_str = install_base.to_string_lossy();
                    let original_dir = apps_dir.join(pkg.name()).join(pkg.version());
                    let original_str = original_dir.to_string_lossy();
                    let persist_str = config
                        .root_path()
                        .join("persist")
                        .join(pkg.name())
                        .to_string_lossy()
                        .into_owned();
                    let substitute = |s: &str| {
                        s.replace("$original_dir", original_str.as_ref())
                            .replace("$persist_dir", persist_str.as_str())
                            .replace("$dir", dir_str.as_ref())
                    };
                    let notes = pkg
                        .manifest()
                        .notes()
                        .unwrap_or_default()
                        .into_iter()
                        .map(substitute)
                        .collect::<Vec<_>>();
                    let suggest = pkg
                        .manifest()
                        .suggest()
                        .map(|map| {
                            let mut entries = map
                                .iter()
                                .map(|(scope, apps)| {
                                    format!("{scope}: {}", apps.devectorize().join(", "))
                                })
                                .collect::<Vec<_>>();
                            entries.sort();
                            entries
                        })
                        .unwrap_or_default();

                    if !notes.is_empty() || !suggest.is_empty() {
                        let _ = tx.send(Event::PackageInstalledNotes {
                            ident: pkg.ident(),
                            notes,
                            suggest,
                        });
                    }

                    let _ = tx.send(Event::PackageCommitDone(pkg.name().to_owned()));
                }

                // Upgrades finalize after the new version commits: the previous
                // version's uninstall hooks run, integration the new manifest
                // dropped is removed, and the old directory goes away.
                if only_upgrade {
                    finalize_upgrade(session, pkg);
                }
                Ok(())
            })();

            if let Err(e) = result {
                eprintln!(
                    "warning: failed to process '{}': {e}; continuing with the rest",
                    pkg.name()
                );
                if deferred.is_none() {
                    deferred = Some(e);
                }
            } else {
                // Fresh installs are not upgrades; reporting them as such
                // was confusing during batch imports.
                let verb = match installed_snapshot.iter().any(|(n, _)| n == pkg.name()) {
                    true => "upgraded",
                    false => "installed",
                };
                println!("{verb} {}", pkg.name());
            }
        }
    }

    // Download, integrity, and commit failures are reported after every
    // other package had its turn, so the command still exits non-zero.
    if let Some(e) = deferred {
        return Err(e);
    }

    // Names that could not be resolved are reported after every other
    // package had its turn, so the command still exits non-zero.
    if let Some(e) = unresolved {
        return Err(e);
    }

    Ok(true)
}

/// Warn about an unresolvable query and remember the first failure.
///
/// A name that matches nothing (or a broken pinned version / manifest
/// reference) must not abort the rest of the batch, so the error is only
/// reported once the remaining packages have been processed.
fn defer_unresolved(deferred: &mut Option<Error>, query: &str, e: Error) {
    eprintln!("warning: could not resolve '{query}': {e}; continuing with the rest");
    if deferred.is_none() {
        *deferred = Some(e);
    }
}

/// Get the architecture string for the current platform.
///
/// Honors the `--arch` override when set, so `install.json` records the
/// architecture that was actually resolved.
/// Whether a manifest version only holds upstream-supported characters
/// (`install_app` aborts otherwise).
fn valid_manifest_version(version: &str) -> bool {
    !version.is_empty()
        && version
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'))
}

/// Remove a previous failed install: a version directory without install
/// metadata. Mirrors the purge half of upstream `ensure_none_failed`.
/// Never touches the directory `current` points at (a pending upgrade is
/// not garbage, it gets repaired in place). Returns whether anything was
/// purged.
fn purge_failed_dir(app_dir: &std::path::Path, version: &str) -> Fallible<bool> {
    let working_dir = app_dir.join(version);
    if !working_dir.is_dir() {
        return Ok(false);
    }
    // Purge only true garbage: no manifest and no install info under
    // either naming scheme. A half-present directory may be another
    // tool's live install — never delete what might be data.
    let any_manifest = super::installed_manifest_path(&working_dir).is_file();
    let any_info = super::install_info_path(&working_dir).is_file();
    if any_manifest || any_info {
        return Ok(false);
    }
    // The live directory (behind `current`) is a pending upgrade, not a
    // failed install — leave it alone.
    let current = app_dir.join("current");
    if current.exists() {
        if let (Ok(link), Ok(dir)) = (current.canonicalize(), working_dir.canonicalize()) {
            if link == dir {
                return Ok(false);
            }
        }
    }
    internal::fs::remove_dir(&working_dir)?;
    Ok(true)
}

/// Rotate an existing version directory aside to `_<version>.old`
/// (`_<version>.old(N)` when taken), mirroring upstream `update -f`.
/// Returns the backup path.
fn rotate_version_dir(app_dir: &std::path::Path, version: &str) -> Fallible<std::path::PathBuf> {
    let mut backup = app_dir.join(format!("_{version}.old"));
    if backup.exists() {
        let mut n = 1;
        loop {
            let candidate = app_dir.join(format!("_{version}.old({n})"));
            if !candidate.exists() {
                backup = candidate;
                break;
            }
            n += 1;
        }
    }
    std::fs::rename(app_dir.join(version), &backup)?;
    Ok(backup)
}

/// Run a manifest `installer.file`/`uninstaller.file` program.
///
/// Mirrors upstream `Invoke-Installer`: the file runs when `file` or `args`
/// is present — `args` alone executes the downloaded file (first URL
/// basename, i.e. the staged filename). The resolved file must live inside
/// the app directory (`is_in_dir`), `args` undergo `$dir`/`$global`/`$version`
/// substitution, `.ps1` files run as hook scripts (and are kept, like
/// upstream) while anything else is executed directly and removed afterwards
/// unless `keep` is set.
#[allow(clippy::too_many_arguments)]
fn run_installer_file(
    session: &Session,
    pkg: &Package,
    working_dir: &std::path::Path,
    file: Option<&str>,
    args: Option<Vec<&str>>,
    keep: bool,
    is_uninstall: bool,
) -> Fallible<()> {
    let kind = if is_uninstall {
        "uninstaller"
    } else {
        "installer"
    };
    let file = file.filter(|f| !f.is_empty());
    let args: Vec<&str> = args
        .unwrap_or_default()
        .into_iter()
        .filter(|a| !a.is_empty())
        .collect();
    if file.is_none() && args.is_empty() {
        return Ok(());
    }

    // Upstream `coalesce $installer.file $Name[0]`: without an explicit
    // file, the downloaded file (staged under its URL basename) runs.
    let name = match file {
        Some(f) => f.to_owned(),
        None => match pkg.download_staged_filenames().into_iter().next() {
            Some(staged) => staged,
            None => {
                return Err(crate::Error::Custom(format!(
                    "{kind} args given for '{}' but no file to run",
                    pkg.name(),
                )));
            }
        },
    };
    let prog = working_dir.join(&name);
    if !prog.is_file() {
        return Err(crate::Error::Custom(format!(
            "{} file '{}' is missing for '{}'",
            kind,
            prog.display(),
            pkg.name(),
        )));
    }
    // The installer file must resolve inside the app directory; a
    // `../` traversal (or a symlink pointing out) aborts the install.
    let canonical_dir = working_dir.canonicalize().map_err(|e| {
        crate::Error::Custom(format!("cannot resolve '{}': {e}", working_dir.display()))
    })?;
    let canonical_prog = prog
        .canonicalize()
        .map_err(|e| crate::Error::Custom(format!("cannot resolve '{}': {e}", prog.display())))?;
    if !canonical_prog.starts_with(&canonical_dir) {
        return Err(crate::Error::Custom(format!(
            "Error in manifest: {kind} {} is outside the app directory",
            prog.display(),
        )));
    }

    let dir_str = working_dir.to_string_lossy();
    let global_str = if session.config().is_global_scope() {
        "True"
    } else {
        "False"
    };
    let substitute = |s: &str| {
        s.replace("$dir", dir_str.as_ref())
            .replace("$global", global_str)
            .replace("$version", pkg.version())
    };
    let fn_args: Vec<String> = args.iter().map(|a| substitute(a)).collect();

    let is_ps1 = prog
        .extension()
        .map(|e| e.eq_ignore_ascii_case("ps1"))
        .unwrap_or(false);
    if is_ps1 {
        // PowerShell files run as hook scripts (with the prelude scope),
        // invoked with the substituted arguments (upstream `& $prog @args`).
        let cmd = if is_uninstall { "uninstall" } else { "install" };
        let mut invocation = format!(
            "& {}",
            crate::internal::ps::ps_quote(&prog.to_string_lossy())
        );
        for arg in &fn_args {
            invocation.push(' ');
            invocation.push_str(&crate::internal::ps::ps_quote(arg));
        }
        let lines = vec![invocation.as_str()];
        internal::ps::invoke_script(session, pkg, cmd, &lines, working_dir)?;
    } else {
        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageCommitStart(format!(
                "running {} file for {}",
                kind,
                pkg.name()
            )));
        }
        let status = std::process::Command::new(&prog)
            .args(&fn_args)
            .current_dir(working_dir)
            .status()?;
        if !status.success() {
            return Err(crate::Error::Custom(format!(
                "{} file '{}' for '{}' exited with {}",
                kind,
                prog.display(),
                pkg.name(),
                status
            )));
        }
        // NOTE: no PackageCommitDone here either. Running the installer
        // file is a mid-transaction step (its start is announced above for
        // binaries); the owning transaction reports completion exactly once.
        // Upstream only removes installer binaries (`.ps1` files are kept);
        // removal is skipped when `keep` is set.
        if !is_ps1 && !keep {
            let _ = std::fs::remove_file(&prog);
        }
    }
    Ok(())
}

/// Extract downloaded archives in the working directory.
///
/// Each archive extracts into its own staging directory so `extract_dir`
/// and `extract_to` pair with URLs by index (upstream semantics): a lone
/// entry broadcasts to every URL (e.g. msival2's single `extract_dir`
/// over 4 URLs), otherwise entry[i] applies to URL[i] (aspell,
/// unxutils). Staged content then merges to the working dir (or the
/// paired `extract_to` subdirectory); unextractable files (e.g. `.exe`
/// app binaries) stay at the top level where shims expect them.
fn extract_package(
    session: &Session,
    pkg: &Package,
    working_dir: &std::path::Path,
) -> Fallible<()> {
    let filenames = pkg.download_staged_filenames();
    let config = session.config();
    let extract_dirs = pkg.manifest().extract_dir().unwrap_or_default();
    let extract_tos = pkg.manifest().extract_to().unwrap_or_default();

    for (index, filename) in filenames.iter().enumerate() {
        let archive_path = working_dir.join(filename);
        if !archive_path.exists() {
            continue;
        }

        // Fresh staging per URL (leftovers from a failed run are dropped).
        let stage = working_dir.join(format!("__url_{index}"));
        let _ = remove_dir_all::remove_dir_all(&stage);
        internal::fs::ensure_dir(&stage)?;

        // MSI packages go through the native MSI extractor (lessmsi or
        // msiexec, mirroring upstream `Expand-MsiArchive`); 7z mangles
        // MSI-internal file names, so it must not handle them.
        let is_msi = archive_path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("msi"))
            .unwrap_or(false);
        if is_msi {
            if let Some(tx) = session.emitter() {
                let _ = tx.send(Event::PackageCommitStart(format!(
                    "extracting archive for {}",
                    pkg.name()
                )));
            }
            let lessmsi = crate::internal::ps::helper_exe(session, "lessmsi", "lessmsi.exe");
            internal::archive::extract_msi(&archive_path, &stage, config.use_lessmsi(), lessmsi)?;
            let _ = std::fs::remove_file(&archive_path);
            merge_staged_content(&stage, working_dir, paired_entry(&extract_tos, index))?;
            continue;
        }

        // Inno Setup executables go through innounp natively (mirroring
        // upstream `Expand-InnoArchive`, which `Invoke-Extraction` selects
        // for `.exe` files with `innosetup` set).
        let is_inno = archive_path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("exe"))
            .unwrap_or(false)
            && pkg.manifest().innosetup();
        if is_inno {
            if let Some(tx) = session.emitter() {
                let _ = tx.send(Event::PackageCommitStart(format!(
                    "extracting archive for {}",
                    pkg.name()
                )));
            }
            let innounp =
                crate::internal::ps::helper_exe(session, "innounp-unicode", "innounp.exe")
                    .or_else(|| crate::internal::ps::helper_exe(session, "innounp", "innounp.exe"))
                    .ok_or_else(|| {
                        crate::Error::Custom(format!(
                            "cannot extract Inno Setup file '{}': innounp app is not installed",
                            archive_path.display()
                        ))
                    })?;
            let status = std::process::Command::new(&innounp)
                .arg("-x")
                .arg(format!("-d{}", stage.to_string_lossy()))
                .arg(&archive_path)
                .arg("-y")
                .arg("-c{app}")
                .status()?;
            if !status.success() {
                return Err(crate::Error::Custom(format!(
                    "innounp failed to extract '{}' ({})",
                    archive_path.display(),
                    status
                )));
            }
            let _ = std::fs::remove_file(&archive_path);
            merge_staged_content(&stage, working_dir, paired_entry(&extract_tos, index))?;
            continue;
        }

        if let Some(format) = internal::archive::Format::detect(&archive_path) {
            if let Some(tx) = session.emitter() {
                let _ = tx.send(Event::PackageCommitStart(format!(
                    "extracting archive for {}",
                    pkg.name()
                )));
            }

            // 7z can handle most formats; for tar.gz/tar.bz2 etc., we need to extract
            // in two steps with 7z (first decompress, then untar)
            let is_tar = filename.to_lowercase().ends_with(".tar");
            let is_compressed_tar = filename.to_lowercase().ends_with(".tar.gz")
                || filename.to_lowercase().ends_with(".tar.bz2")
                || filename.to_lowercase().ends_with(".tgz")
                || filename.to_lowercase().ends_with(".tbz2");

            if is_tar || is_compressed_tar {
                // 7z handles .tar files natively, and .tar.gz etc. automatically
                internal::archive::extract_with_root(
                    &archive_path,
                    &stage,
                    Some(config.root_path()),
                )?;
            } else {
                // Extract to the staging directory first
                internal::archive::extract_with_root(
                    &archive_path,
                    &stage,
                    Some(config.root_path()),
                )?;
            }

            // Clean up the archive file after extraction
            let _ = std::fs::remove_file(&archive_path);

            // This URL's extract_dir promotes within the stage.
            if let Some(sub) = paired_entry(&extract_dirs, index) {
                promote_subdir(&stage, &sub)?;
            }
            merge_staged_content(&stage, working_dir, paired_entry(&extract_tos, index))?;
        }
    }

    Ok(())
}

/// Pick the `extract_dir`/`extract_to` entry for one URL: a lone entry
/// broadcasts to every URL, otherwise entries pair by URL index.
fn paired_entry(entries: &[&str], index: usize) -> Option<String> {
    if entries.len() == 1 {
        entries.first().map(|s| s.to_string())
    } else {
        entries.get(index).map(|s| s.to_string())
    }
}

/// Promote a subdirectory's contents up into its root, pruning the
/// emptied directories deepest-first. A no-op when the subdirectory
/// does not exist (e.g. flat 7z output).
fn promote_subdir(root: &std::path::Path, sub: &str) -> Fallible<()> {
    let src_dir = root.join(sub);
    if !src_dir.is_dir() {
        return Ok(());
    }
    // Move the subdirectory's contents up into the root.
    let entries = std::fs::read_dir(&src_dir)?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let src = entry.path();
        let dst = root.join(&name);
        if dst.exists() {
            if dst.is_dir() {
                continue;
            }
            let _ = std::fs::remove_file(&dst);
        }
        std::fs::rename(&src, &dst)?;
    }
    // Remove the (now empty) promoted directories, deepest first.
    let mut dir = src_dir.as_path();
    while dir.starts_with(root) && dir != root {
        if std::fs::read_dir(dir)?.next().is_some() {
            break;
        }
        std::fs::remove_dir(dir)?;
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    Ok(())
}

/// Merge staged content to its destination: the working dir, or the
/// paired `extract_to` subdirectory of it (created on demand).
/// Unextractable files left at the top level are never relocated.
/// Directories merge recursively (matching direct multi-archive
/// extraction); files replace.
fn merge_staged_content(
    stage: &std::path::Path,
    working_dir: &std::path::Path,
    extract_to: Option<String>,
) -> Fallible<()> {
    let dest = match extract_to {
        Some(sub) => {
            let dir = working_dir.join(sub);
            internal::fs::ensure_dir(&dir)?;
            dir
        }
        None => working_dir.to_owned(),
    };
    merge_dir_contents(stage, &dest)?;
    let _ = std::fs::remove_dir(stage);
    Ok(())
}

/// Move every entry of `src` into `dst`, merging directories recursively
/// and replacing files.
fn merge_dir_contents(src: &std::path::Path, dst: &std::path::Path) -> Fallible<()> {
    let entries = std::fs::read_dir(src)?;
    for entry in entries {
        let entry = entry?;
        let name = entry.file_name();
        let from = entry.path();
        let to = dst.join(&name);
        if from.is_dir() {
            if to.exists() && to.is_dir() {
                merge_dir_contents(&from, &to)?;
                let _ = std::fs::remove_dir(&from);
                continue;
            }
            if to.exists() {
                let _ = std::fs::remove_file(&to);
            }
            std::fs::rename(&from, &to)?;
            continue;
        }
        if to.exists() {
            if to.is_dir() {
                continue;
            }
            let _ = std::fs::remove_file(&to);
        }
        std::fs::rename(&from, &to)?;
    }
    Ok(())
}

/// Finalize an upgrade after the new version committed: run the previous
/// version's uninstall hooks, drop integration the new manifest no longer
/// declares (stale shims/shortcuts/env entries), and remove the old
/// version directory.
///
/// Best-effort convergence, mirroring the install side's tolerance: the
/// upgrade itself already succeeded, so failures here warn instead of
/// failing the transaction. `current` and rotation backups (`_*`) are
/// never touched.
fn finalize_upgrade(session: &Session, pkg: &Package) {
    if let Err(e) = finalize_upgrade_inner(session, pkg) {
        eprintln!(
            "warning: upgrade cleanup for '{}' incomplete: {e}",
            pkg.name()
        );
    }
}

fn finalize_upgrade_inner(session: &Session, pkg: &Package) -> Fallible<()> {
    use std::collections::HashSet;

    let config = session.config();
    let app_dir = config.root_path().join("apps").join(pkg.name());
    // The version that was just committed: the origin (bucket) version, not
    // the recorded install state. For an upgradable reference
    // `installed_version()` is still the *previous* version, so using it
    // here treated the freshly installed directory as an old one — deleting
    // it and leaving `current` dangling while the old version survived.
    let new_version = pkg.version().to_owned();
    let new_base = if config.no_junction() {
        app_dir.join(&new_version)
    } else {
        app_dir.join("current")
    };
    drop(config);

    // Previous version directories carrying install metadata.
    let mut olds = vec![];
    if let Ok(entries) = std::fs::read_dir(&app_dir) {
        for entry in entries.filter_map(|de| de.ok()) {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == "current" || name == new_version || name.starts_with('_') {
                continue;
            }
            let dir = entry.path();
            if dir.is_dir() && super::has_install_metadata(&dir) {
                olds.push(dir);
            }
        }
    }
    olds.sort();

    let new_manifest = pkg.manifest();
    let new_bins: HashSet<String> = new_manifest
        .bin()
        .unwrap_or_default()
        .iter()
        .map(|def| crate::shim::Shim::name_of(def).to_owned())
        .collect();
    let new_shortcuts: HashSet<String> =
        shortcut::shortcut_names(new_manifest, pkg.name(), &new_base)
            .into_iter()
            .collect();
    let new_paths = env_paths_for(&new_base, new_manifest);
    let new_vars = env_vars_for(new_manifest);

    for old_dir in olds {
        let old_manifest = Manifest::parse(super::installed_manifest_path(&old_dir))?;
        // Hook context for the old version ($dir = old version dir).
        let old_pkg = Package::from(pkg.name(), pkg.bucket(), old_manifest.clone());
        let arch = InstallInfo::parse(super::install_info_path(&old_dir))
            .map(|info| info.arch().to_owned())
            .unwrap_or_default();
        old_pkg.fill_install_state(InstallState::Installed(InstallStateInstalled {
            version: old_manifest.version().to_owned(),
            bucket: None,
            arch,
            held: false,
            url: None,
        }));

        // The old version's uninstall hooks (same semantics as remove).
        if let Some(pre) = old_manifest.pre_uninstall() {
            internal::ps::invoke_script(session, &old_pkg, "uninstall", &pre, &old_dir)?;
        }
        if let Some(uninstaller) = old_manifest.uninstaller() {
            run_installer_file(
                session,
                &old_pkg,
                &old_dir,
                uninstaller.file(),
                uninstaller.args(),
                uninstaller.keep(),
                true,
            )?;
            if let Some(script) = uninstaller.script() {
                internal::ps::invoke_script(session, &old_pkg, "uninstall", &script, &old_dir)?;
            }
        }
        if let Some(post) = old_manifest.post_uninstall() {
            internal::ps::invoke_script(session, &old_pkg, "uninstall", &post, &old_dir)?;
        }

        // Stale shims/shortcuts: declared by the old manifest, gone from
        // the new one. Shared names now point at the new install and are
        // left alone.
        let old_bins: HashSet<String> = old_manifest
            .bin()
            .unwrap_or_default()
            .iter()
            .map(|def| crate::shim::Shim::name_of(def).to_owned())
            .collect();
        let stale_bins: HashSet<String> = old_bins.difference(&new_bins).cloned().collect();
        shim::remove_stale(session, &stale_bins)?;

        let old_shortcuts: HashSet<String> =
            shortcut::shortcut_names(&old_manifest, pkg.name(), &old_dir)
                .into_iter()
                .collect();
        let stale_shortcuts: HashSet<String> =
            old_shortcuts.difference(&new_shortcuts).cloned().collect();
        shortcut::remove_stale(session, &stale_shortcuts)?;

        // Stale env: old PATH entries and vars the new manifest dropped.
        let old_paths = env_paths_for(&old_dir, &old_manifest);
        let old_vars = env_vars_for(&old_manifest);
        env::remove_stale(session, &old_paths, &new_paths, &old_vars, &new_vars)?;

        // Drop the old version directory.
        internal::fs::tolerate_not_found(internal::fs::remove_dir(&old_dir))?;
    }

    Ok(())
}

/// `env_add_path` entries of a manifest resolved against a base dir.
fn env_paths_for(base: &std::path::Path, manifest: &Manifest) -> Vec<std::path::PathBuf> {
    match manifest.env_add_path() {
        Some(entries) => entries
            .into_iter()
            .map(|p| internal::path::normalize_path(base.join(p)))
            .collect(),
        None => Vec::new(),
    }
}

/// `env_set` variable names of a manifest.
fn env_vars_for(manifest: &Manifest) -> Vec<String> {
    manifest
        .env_set()
        .map(|map| map.keys().cloned().collect())
        .unwrap_or_default()
}

/// Sync operation: remove packages.
pub fn remove(session: &Session, queries: &[&str], options: &[SyncOption]) -> Fallible<bool> {
    let mut packages = vec![];

    let installed = query::query_installed(session, &["*"], &[])?;
    let escape_hold = options.contains(&SyncOption::EscapeHold);

    for &name in queries {
        let mut matched = installed
            .iter()
            .filter(|&p| p.name().eq_ignore_ascii_case(name))
            .cloned()
            .collect::<Vec<_>>();

        if matched.is_empty() {
            return Err(Error::PackageNotFound(name.to_string()));
        }

        // It's impossible to have more than one installed packages for the same
        // package name.
        assert_eq!(matched.len(), 1);

        let pkg = matched.pop().unwrap();

        if pkg.is_held() && !escape_hold {
            continue;
        }

        packages.push(pkg);
    }

    let no_dependent_check = options.contains(&SyncOption::NoDependentCheck);
    if !no_dependent_check {
        let mut dependents = vec![];

        for pkg in packages.iter() {
            let mut result = installed
                .iter()
                .filter_map(|p| {
                    if packages.contains(p) {
                        return None;
                    }

                    let dep_names = p
                        .dependencies()
                        .into_iter()
                        .map(super::extract_name)
                        .collect::<Vec<_>>();

                    if dep_names.contains(&pkg.name().to_owned()) {
                        // p depends on pkg
                        Some((p.name().to_owned(), pkg.name().to_owned()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();

            if result.is_empty() {
                continue;
            }

            dependents.append(&mut result);
        }

        if !dependents.is_empty() {
            return Err(Error::PackageDependentFound(dependents));
        }
    }

    let is_cascade = options.contains(&SyncOption::Cascade);
    if is_cascade {
        resolve::resolve_cascade(session, &mut packages, escape_hold)?;
    }

    // Drop packages blocked by running processes before confirming, so the
    // prompt only covers what will actually be removed.
    let packages = filter_running(session, packages)?;

    if let Some(tx) = session.emitter() {
        let _ = tx.send(Event::PackageResolveDone);
    }

    let transaction = Transaction::default();

    transaction.set_remove(packages);

    let assume_yes = options.contains(&SyncOption::AssumeYes);
    if !assume_yes {
        if let Some(tx) = session.emitter() {
            if tx
                .send(Event::PromptTransactionNeedConfirm(transaction.clone()))
                .is_ok()
            {
                let rx = session.receiver().unwrap();
                let mut confirmed = false;

                while let Ok(event) = rx.recv() {
                    if let Event::PromptTransactionNeedConfirmResult(ret) = event {
                        confirmed = ret;
                        break;
                    }
                }

                if !confirmed {
                    return Ok(false);
                }
            }
        }
    }

    let did_work = transaction
        .remove_view()
        .is_some_and(|packages| !packages.is_empty());

    if let Some(packages) = transaction.remove_view() {
        let purge = options.contains(&SyncOption::Purge);
        let config = session.config();
        let root_dir = config.root_path();

        for package in packages.iter() {
            // Re-check at commit time: a process may have started while
            // confirming. Skip it rather than failing the transaction.
            if let Some(processes) = running_blocked(session, package) {
                if let Some(tx) = session.emitter() {
                    let _ = tx.send(Event::PackageRunningSkipped {
                        name: package.name().to_owned(),
                        processes,
                    });
                }
                continue;
            }

            if let Some(tx) = session.emitter() {
                let _ = tx.send(Event::PackageCommitStart(package.name().to_owned()));
            }

            let app_dir = root_dir.join("apps").join(package.name());

            // Hook scripts observe the real version directory as `$dir`,
            // matching upstream Scoop (which never points hooks at the
            // `current` junction).
            let uninstall_dir =
                app_dir.join(package.installed_version().unwrap_or(package.version()));
            if let Some(pre_uninstall) = package.manifest().pre_uninstall() {
                internal::ps::invoke_script(
                    session,
                    package,
                    "uninstall",
                    &pre_uninstall,
                    &uninstall_dir,
                )?;
            }

            // Run uninstaller file and/or script if present (same
            // `file`-or-`args` trigger as the installer side).
            if let Some(uninstaller) = package.manifest().uninstaller() {
                run_installer_file(
                    session,
                    package,
                    &uninstall_dir,
                    uninstaller.file(),
                    uninstaller.args(),
                    uninstaller.keep(),
                    true,
                )?;
                if let Some(script) = uninstaller.script() {
                    internal::ps::invoke_script(
                        session,
                        package,
                        "uninstall",
                        &script,
                        &uninstall_dir,
                    )?;
                }
            }

            shim::remove(session, package)?;
            shortcut::remove(session, package)?;
            psmodule::remove(session, package)?;
            env::remove(session, package)?;
            persist::unlink(session, package)?;

            let current_lnk = app_dir.join("current");
            internal::fs::remove_symlink(current_lnk)?;

            // Run post_uninstall script if present
            if let Some(post_uninstall) = package.manifest().post_uninstall() {
                internal::ps::invoke_script(
                    session,
                    package,
                    "uninstall",
                    &post_uninstall,
                    &uninstall_dir,
                )?;
            }

            // Remove the app directory (already-absent is fine: a retried
            // uninstall after a partial failure must converge, not abort).
            internal::fs::tolerate_not_found(internal::fs::remove_dir(&app_dir))?;

            if purge {
                if let Some(tx) = session.emitter() {
                    let _ = tx.send(Event::PackagePersistPurgeStart);
                }

                let persist_dir = config.root_path().join("persist").join(package.name());
                internal::fs::tolerate_not_found(internal::fs::remove_dir(&persist_dir))?;

                if let Some(tx) = session.emitter() {
                    let _ = tx.send(Event::PackagePersistPurgeDone);
                }
            }

            if let Some(tx) = session.emitter() {
                let _ = tx.send(Event::PackageCommitDone(package.name().to_owned()));
            }
        }
    }

    Ok(did_work)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::manifest::Manifest;

    /// Build an installed-looking package with the given name/version.
    fn test_package(name: &str) -> Package {
        let manifest = Manifest::parse_bytes(
            br#"{"version": "1.0", "homepage": "https://example.com", "license": "MIT"}"#,
            std::path::Path::new("running.json"),
        )
        .expect("fixture manifest should parse");
        Package::from(name, "main", manifest)
    }

    /// A busy app blocks the transaction while an idle app stays runnable.
    ///
    /// Spawns a real process from a copy of powershell.exe placed inside the
    /// fake app directory, mirroring e.g. FanControl running from its
    /// version dir during `bagger upgrade`.
    #[test]
    #[cfg(windows)]
    fn filter_running_skips_busy_apps() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-running-skip");
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("root");
        let busy_dir = root.join("apps").join("busyapp").join("1.0");
        std::fs::create_dir_all(&busy_dir).unwrap();
        std::env::set_var("SCOOP", &root);
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));

        // A sleeper exe living inside the app dir, like FanControl.exe.
        let system_ps = std::path::PathBuf::from(
            std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string()),
        )
        .join("System32/WindowsPowerShell/v1.0/powershell.exe");
        let sleeper = busy_dir.join("sleeper.exe");
        std::fs::copy(&system_ps, &sleeper).expect("powershell.exe should be copyable");

        let session = Session::new();
        let mut child = std::process::Command::new(&sleeper)
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep 30",
            ])
            .spawn()
            .expect("sleeper should spawn");

        // The process needs a moment to appear in the sysinfo snapshot.
        let mut seen = false;
        for _ in 0..20 {
            let (_, blocked) = partition_runnable(
                &session,
                vec![test_package("busyapp"), test_package("idleapp")],
            );
            if blocked.iter().any(|(n, _)| n == "busyapp") {
                seen = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        assert!(seen, "busyapp should be detected as running");

        let (runnable, blocked) = partition_runnable(
            &session,
            vec![test_package("busyapp"), test_package("idleapp")],
        );
        assert_eq!(runnable.len(), 1);
        assert_eq!(runnable[0].name(), "idleapp");
        assert_eq!(blocked.len(), 1);
        assert_eq!(blocked[0].0, "busyapp");
        assert!(!blocked[0].1.is_empty());

        // An explicitly requested single package still fails loudly.
        let err = filter_running(&session, vec![test_package("busyapp")]).unwrap_err();
        assert!(err.to_string().contains("busyapp"));

        let _ = child.kill();
        let _ = child.wait();

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn filter_running_passes_everything_when_ignored() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-running-ignore");
        let _ = std::fs::remove_dir_all(&base);
        std::env::set_var("SCOOP", base.join("root"));
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));

        let session = Session::new();
        // `ignore_running_processes` cannot be set without a config file;
        // without it, idle apps (no processes) always pass through.
        let runnable = filter_running(&session, vec![test_package("idleapp")]).unwrap();
        assert_eq!(runnable.len(), 1);

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }

    /// Staged filenames mirror upstream `url_filename`: real basenames with
    /// `#/...` fragments coerced, so hook scripts and shim targets resolve.
    #[test]
    fn staged_filenames_use_url_basenames() {
        let manifest = Manifest::parse_bytes(
            br#"{"version": "1.0", "homepage": "https://example.com", "license": "MIT",
                "architecture": {"64bit": {"url": ["https://example.com/Obsidian-1.14.4.exe#/dl.7z", "https://example.com/app.zip?v=2"]}}}"#,
            std::path::Path::new("staged.json"),
        )
        .unwrap();
        let pkg = Package::from("staged", "main", manifest);
        assert_eq!(
            pkg.download_staged_filenames(),
            vec!["dl.7z".to_string(), "app.zip".to_string()]
        );
        // Cache names stay hashed and distinct.
        let cached = pkg.download_filenames();
        assert_eq!(cached.len(), 2);
        assert!(cached[0].starts_with("staged#1.0#"));
        assert_ne!(cached[0], cached[1]);
    }

    /// Force reinstalls rotate the previous version aside (`_<v>.old`,
    /// then `_<v>.old(N)`), mirroring upstream `update -f`.
    #[test]
    fn rotate_version_dir_keeps_history() {
        let base = std::env::temp_dir().join("bagger-test-rotate");
        let _ = std::fs::remove_dir_all(&base);
        let app = base.join("myapp");
        std::fs::create_dir_all(app.join("1.0")).unwrap();
        std::fs::write(app.join("1.0/marker.txt"), "old").unwrap();

        let first = rotate_version_dir(&app, "1.0").unwrap();
        assert_eq!(first, app.join("_1.0.old"));
        assert_eq!(
            std::fs::read_to_string(first.join("marker.txt")).unwrap(),
            "old"
        );
        assert!(!app.join("1.0").exists());

        std::fs::create_dir_all(app.join("1.0")).unwrap();
        let second = rotate_version_dir(&app, "1.0").unwrap();
        assert_eq!(second, app.join("_1.0.old(1)"));

        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn manifest_versions_reject_upstream_unsupported_chars() {
        for good in ["1.0", "2.7.0", "nightly", "1.0-beta+2", "2024.01_rc1"] {
            assert!(valid_manifest_version(good), "{good}");
        }
        for bad in ["", "1.0 beta", "v1/0", "1.0?", "a:b", "x$y"] {
            assert!(!valid_manifest_version(bad), "{bad}");
        }
    }

    /// Stale version dirs (no install metadata) are purged, but healthy
    /// installs and the `current`-live directory are never touched.
    #[test]
    fn purge_failed_dir_only_purges_stale_dirs() {
        let base = std::env::temp_dir().join("bagger-test-purge");
        let _ = std::fs::remove_dir_all(&base);
        let app = base.join("myapp");
        // Stale: purged.
        std::fs::create_dir_all(app.join("1.0")).unwrap();
        std::fs::write(app.join("1.0/partial.bin"), b"x").unwrap();
        assert!(purge_failed_dir(&app, "1.0").unwrap());
        assert!(!app.join("1.0").exists());
        // Missing: nothing to do.
        assert!(!purge_failed_dir(&app, "9.9").unwrap());
        // Healthy (markers present): kept.
        std::fs::create_dir_all(app.join("2.0")).unwrap();
        std::fs::write(app.join("2.0/install.json"), "{}").unwrap();
        assert!(!purge_failed_dir(&app, "2.0").unwrap());
        assert!(app.join("2.0/install.json").exists());

        std::fs::remove_dir_all(&base).ok();
    }

    /// The `current`-live directory is a pending upgrade, not garbage —
    /// even without markers it must survive the purge.
    #[test]
    #[cfg(windows)]
    fn purge_failed_dir_spares_live_current() {
        let base = std::env::temp_dir().join("bagger-test-purge-live");
        let _ = std::fs::remove_dir_all(&base);
        let app = base.join("myapp");
        std::fs::create_dir_all(app.join("3.0")).unwrap();
        crate::internal::fs::symlink_dir(&app.join("3.0"), &app.join("current")).unwrap();

        assert!(!purge_failed_dir(&app, "3.0").unwrap());
        assert!(app.join("3.0").is_dir());

        std::fs::remove_dir_all(&base).ok();
    }

    /// Nightly stamps are dated (`nightly-yyyyMMdd`) and `with_version`
    /// patches only the version, preserving the rest of the manifest.
    #[test]
    fn nightly_stamp_format_and_surgery() {
        let stamp = nightly_version();
        assert!(stamp.starts_with("nightly-"), "{stamp}");
        assert_eq!(stamp.len(), "nightly-YYYYMMDD".len());
        assert!(stamp[8..].bytes().all(|b| b.is_ascii_digit()));

        let manifest = Manifest::parse_bytes(
            br#"{"version": "nightly", "homepage": "https://example.com", "license": "MIT",
                "url": "https://example.com/nightly.zip"}"#,
            std::path::Path::new("nightly.json"),
        )
        .unwrap();
        let pkg = Package::from("nightapp", "main", manifest);
        let stamped = pkg.with_version(&stamp);
        assert_eq!(stamped.version(), stamp);
        assert_eq!(stamped.name(), "nightapp");
        assert_eq!(stamped.bucket(), "main");
        assert_eq!(pkg.version(), "nightly");
    }

    /// Build a package whose manifest stages `setup.cmd` and declares an
    /// `installer` section from JSON.
    fn installer_package(installer_json: &str) -> Package {
        let manifest_json = format!(
            r#"{{"version": "1.0", "homepage": "https://example.com", "license": "MIT",
                "architecture": {{"64bit": {{"url": "https://example.com/files/setup.cmd"}}}},
                "installer": {installer_json}}}"#
        );
        let manifest = Manifest::parse_bytes(
            manifest_json.as_bytes(),
            std::path::Path::new("installer.json"),
        )
        .unwrap();
        Package::from("instapp", "main", manifest)
    }

    fn installer_session(base: &std::path::Path) -> Session {
        let _ = std::fs::remove_dir_all(base);
        std::env::set_var("SCOOP", base.join("root"));
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));
        Session::new()
    }

    fn cleanup_installer_env(base: &std::path::Path) {
        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }

    /// `extract_dir`/`extract_to` pair with URLs by index; a lone entry
    /// broadcasts. Merges recurse, files replace, promotions prune.
    #[test]
    fn extract_pairing_merge_and_promote() {
        assert_eq!(paired_entry(&["a"], 0), Some("a".to_string()));
        assert_eq!(paired_entry(&["a"], 7), Some("a".to_string()));
        assert_eq!(paired_entry(&["a", "b"], 1), Some("b".to_string()));
        assert_eq!(paired_entry(&["a", "b"], 5), None);
        assert_eq!(paired_entry(&[], 0), None);

        let base = std::env::temp_dir().join("bagger-test-extract-pair");
        let _ = std::fs::remove_dir_all(&base);
        let stage = base.join("stage");
        let dest = base.join("dest");
        std::fs::create_dir_all(stage.join("sub/deep")).unwrap();
        std::fs::write(stage.join("sub/deep/f.txt"), "f").unwrap();
        std::fs::write(stage.join("top.txt"), "top").unwrap();
        std::fs::create_dir_all(dest.join("sub")).unwrap();
        std::fs::write(dest.join("sub/old.txt"), "old").unwrap();

        promote_subdir(&stage, "sub").unwrap();
        assert_eq!(
            std::fs::read_to_string(stage.join("deep/f.txt")).unwrap(),
            "f"
        );
        assert!(!stage.join("sub").exists());

        merge_staged_content(&stage, &dest, None).unwrap();
        assert_eq!(
            std::fs::read_to_string(dest.join("deep/f.txt")).unwrap(),
            "f"
        );
        assert_eq!(
            std::fs::read_to_string(dest.join("top.txt")).unwrap(),
            "top"
        );
        // Recursive merge kept the pre-existing file.
        assert_eq!(
            std::fs::read_to_string(dest.join("sub/old.txt")).unwrap(),
            "old"
        );

        // extract_to relocates staged content into the subdirectory.
        let stage2 = base.join("stage2");
        std::fs::create_dir_all(&stage2).unwrap();
        std::fs::write(stage2.join("moved.txt"), "m").unwrap();
        merge_staged_content(&stage2, &dest, Some("sub2".to_string())).unwrap();
        assert_eq!(
            std::fs::read_to_string(dest.join("sub2/moved.txt")).unwrap(),
            "m"
        );

        std::fs::remove_dir_all(&base).ok();
    }

    /// `args` without `file` runs the staged download (upstream `coalesce`
    /// to the URL basename), with `$dir`/`$version` substituted.
    #[test]
    #[cfg(windows)]
    fn installer_args_without_file_runs_staged_download() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-installer-args");
        let _session = installer_session(&base);
        let pkg = installer_package(r#"{"args": ["--root=$dir", "--ver=$version"]}"#);
        let dir = base.join("root/apps/instapp/1.0");
        std::fs::create_dir_all(&dir).unwrap();
        // A batch installer logging its argv (like NSIS `/D=$dir` flows).
        std::fs::write(
            dir.join("setup.cmd"),
            "@echo off\r\nsetlocal\r\n(for %%A in (%*) do @echo/%%~A) > \"%~dp0argv.log\"\r\n",
        )
        .unwrap();

        run_installer_file(
            &_session,
            &pkg,
            &dir,
            None,
            pkg.manifest().installer().unwrap().args(),
            false,
            false,
        )
        .expect("args-only installer should run the staged file");

        let logged = std::fs::read_to_string(dir.join("argv.log")).unwrap();
        assert!(
            logged.contains(&format!("--root={}", dir.display())),
            "unexpected argv log: {logged}"
        );
        assert!(
            logged.contains("--ver=1.0"),
            "unexpected argv log: {logged}"
        );
        // Non-ps1 installers are removed afterwards unless `keep` is set.
        assert!(!dir.join("setup.cmd").exists());

        cleanup_installer_env(&base);
    }

    /// `keep: true` preserves installer binaries; `.ps1` installers are
    /// always kept (upstream only removes binaries).
    #[test]
    #[cfg(windows)]
    fn installer_keep_and_ps1_retention() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-installer-keep");
        let _session = installer_session(&base);
        let dir = base.join("root/apps/instapp/1.0");
        std::fs::create_dir_all(&dir).unwrap();

        // Binary with keep=true survives.
        let pkg = installer_package(r#"{"file": "setup.cmd", "keep": true}"#);
        std::fs::write(dir.join("setup.cmd"), "@echo off\r\nexit /b 0\r\n").unwrap();
        run_installer_file(
            &_session,
            &pkg,
            &dir,
            pkg.manifest().installer().unwrap().file(),
            None,
            true,
            false,
        )
        .unwrap();
        assert!(dir.join("setup.cmd").exists());

        // `.ps1` survives even without keep (executed via the hook scope).
        let pkg = installer_package(r#"{"file": "setup.ps1"}"#);
        std::fs::write(
            dir.join("setup.ps1"),
            "Set-Content -LiteralPath \"$dir/ps1-marker.txt\" -Value 'ran'",
        )
        .unwrap();
        run_installer_file(
            &_session,
            &pkg,
            &dir,
            pkg.manifest().installer().unwrap().file(),
            None,
            false,
            false,
        )
        .unwrap();
        assert!(dir.join("setup.ps1").exists());
        assert!(dir.join("ps1-marker.txt").exists());

        cleanup_installer_env(&base);
    }

    /// Installer files escaping the app dir abort (`is_in_dir`), as do
    /// missing files.
    #[test]
    #[cfg(windows)]
    fn installer_outside_dir_and_missing_abort() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-installer-guard");
        let _session = installer_session(&base);
        let pkg = installer_package(r#"{"file": "setup.cmd"}"#);
        let dir = base.join("root/apps/instapp/1.0");
        std::fs::create_dir_all(&dir).unwrap();
        // A real file next to (not inside) the version dir.
        std::fs::write(base.join("root/apps/instapp/evil.cmd"), "@echo off\r\n").unwrap();

        let err = run_installer_file(
            &_session,
            &pkg,
            &dir,
            Some("../evil.cmd"),
            None,
            false,
            false,
        )
        .unwrap_err();
        assert!(err.to_string().contains("outside the app directory"));

        let err = run_installer_file(&_session, &pkg, &dir, Some("setup.cmd"), None, false, false)
            .unwrap_err();
        assert!(err.to_string().contains("missing"));

        cleanup_installer_env(&base);
    }

    /// Running installer files/scripts emits no `PackageCommitDone`: they
    /// are mid-transaction steps, and each extra send printed another green
    /// "'x' was uninstalled." line on top of the real commit event.
    #[test]
    #[cfg(windows)]
    fn installer_file_sends_no_commit_done() {
        use crate::Event;

        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-installer-events");
        let session = installer_session(&base);
        let rx = session.event_bus().receiver();

        // Binary installer.
        let pkg = installer_package(r#"{"file": "setup.cmd", "args": "/S"}"#);
        let dir = base.join("root/apps/instapp/1.0");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("setup.cmd"), "@echo off\r\n").unwrap();
        run_installer_file(
            &session,
            &pkg,
            &dir,
            Some("setup.cmd"),
            pkg.manifest().installer().unwrap().args(),
            true,
            false,
        )
        .unwrap();

        // PowerShell installer (runs through the hook-script path).
        std::fs::write(dir.join("setup.ps1"), "\"ok\" | Out-Null\r\n").unwrap();
        run_installer_file(&session, &pkg, &dir, Some("setup.ps1"), None, true, false).unwrap();

        // Hook script path directly.
        crate::internal::ps::invoke_script(&session, &pkg, "install", &["\"ok\""], &dir).unwrap();

        let mut commits = 0;
        while let Ok(event) = rx.try_recv() {
            if matches!(event, Event::PackageCommitDone(_)) {
                commits += 1;
            }
        }
        assert_eq!(
            commits, 0,
            "installer steps must not send PackageCommitDone"
        );

        cleanup_installer_env(&base);
    }

    /// Uninstalling after a partial failure (shims already removed, app dir
    /// still present) converges instead of aborting with os error 2.
    #[test]
    fn remove_converges_after_partial_failure() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-remove-retry");
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("root");
        let version = root.join("apps/halfapp/1.0");
        std::fs::create_dir_all(&version).unwrap();
        std::fs::write(
            version.join("manifest.json"),
            r#"{"version": "1.0", "homepage": "https://example.com", "license": "MIT",
                "bin": "half.exe"}"#,
        )
        .unwrap();
        std::fs::write(
            version.join("install.json"),
            r#"{"architecture": "64bit", "bucket": "fake"}"#,
        )
        .unwrap();
        crate::internal::fs::symlink_dir(&version, root.join("apps/halfapp/current")).unwrap();
        // Shims dir exists but the shim is already gone (first attempt
        // removed it before dying further down the tail).
        std::fs::create_dir_all(root.join("shims")).unwrap();

        std::env::set_var("SCOOP", &root);
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));
        let session = Session::new();
        let did_work = remove(&session, &["halfapp"], &[]).unwrap();
        assert!(did_work);
        assert!(!root.join("apps/halfapp").exists());

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }

    /// Upgrade finalization runs the old version's uninstall hooks, drops
    /// stale shims the new manifest no longer declares, and removes the
    /// old version directory — while keeping shared shims and the new
    /// install intact.
    #[test]
    #[cfg(windows)]
    fn finalize_upgrade_cleans_old_version() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-finalize-upgrade");
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("root");
        let old = root.join("apps/finapp/1.0");
        let new = root.join("apps/finapp/2.0");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(
            old.join("manifest.json"),
            r#"{"version": "1.0", "homepage": "https://example.com", "license": "MIT",
                "bin": ["oldbin.exe"],
                "post_uninstall": ["Set-Content -Path \"$dir/../hook-marker.txt\" -Value hooked"]}"#,
        )
        .unwrap();
        std::fs::write(
            old.join("install.json"),
            r#"{"architecture": "64bit", "bucket": "fake"}"#,
        )
        .unwrap();
        std::fs::write(
            new.join("manifest.json"),
            r#"{"version": "2.0", "homepage": "https://example.com", "license": "MIT",
                "bin": ["newbin.exe"]}"#,
        )
        .unwrap();
        std::fs::write(
            new.join("install.json"),
            r#"{"architecture": "64bit", "bucket": "fake"}"#,
        )
        .unwrap();
        crate::internal::fs::symlink_dir(&new, root.join("apps/finapp/current")).unwrap();
        let shims = root.join("shims");
        std::fs::create_dir_all(&shims).unwrap();
        std::fs::write(shims.join("oldbin.cmd"), "@echo off\n").unwrap();
        std::fs::write(shims.join("newbin.cmd"), "@echo off\n").unwrap();

        std::env::set_var("SCOOP", &root);
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));
        let session = Session::new();
        let pkgs = query::query_installed(&session, &["finapp"], &[QueryOption::Explicit]).unwrap();
        assert_eq!(pkgs.len(), 1);

        finalize_upgrade(&session, &pkgs[0]);

        // Old hook ran ($dir = old version dir, marker beside it).
        assert_eq!(
            std::fs::read_to_string(root.join("apps/finapp/hook-marker.txt")).unwrap(),
            "hooked\r\n"
        );
        // Stale shim gone, shared/new shim kept, old dir dropped.
        assert!(!shims.join("oldbin.cmd").exists());
        assert!(shims.join("newbin.cmd").is_file());
        assert!(!old.exists());
        assert!(new.is_dir());

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }

    /// The package passed to `finalize_upgrade` during an upgrade is the
    /// origin reference: `version()` is the version just committed while
    /// `installed_version()` is still the previous one. Finalization must key
    /// off the committed version — keying off the install state deleted the
    /// fresh install and left a dangling `current` behind (every real
    /// upgrade hit this).
    #[test]
    #[cfg(windows)]
    fn finalize_upgrade_keeps_new_version_of_upgradable_ref() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-finalize-upgradable");
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("root");
        let old = root.join("apps/finapp/1.0");
        let new = root.join("apps/finapp/2.0");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&new).unwrap();
        std::fs::write(
            old.join("manifest.json"),
            r#"{"version": "1.0", "homepage": "https://example.com", "license": "MIT"}"#,
        )
        .unwrap();
        std::fs::write(
            old.join("install.json"),
            r#"{"architecture": "64bit", "bucket": "fake"}"#,
        )
        .unwrap();
        std::fs::write(
            new.join("manifest.json"),
            r#"{"version": "2.0", "homepage": "https://example.com", "license": "MIT"}"#,
        )
        .unwrap();
        std::fs::write(
            new.join("install.json"),
            r#"{"architecture": "64bit", "bucket": "fake"}"#,
        )
        .unwrap();
        crate::internal::fs::symlink_dir(&new, root.join("apps/finapp/current")).unwrap();

        std::env::set_var("SCOOP", &root);
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));
        let session = Session::new();

        // Shape of an upgradable reference: origin manifest (2.0) carrying
        // the recorded install state of the previous version (1.0).
        let manifest = Manifest::parse(old.join("manifest.json"));
        let manifest = manifest.expect("parse fixture");
        let pkg = Package::from("finapp", "fake", manifest).with_version("2.0");
        pkg.fill_install_state(crate::package::InstallState::Installed(
            crate::package::InstallStateInstalled {
                version: "1.0".to_owned(),
                bucket: Some("fake".to_owned()),
                arch: "64bit".to_owned(),
                held: false,
                url: None,
            },
        ));
        assert_eq!(pkg.version(), "2.0");
        assert_eq!(pkg.installed_version(), Some("1.0"));

        finalize_upgrade(&session, &pkg);

        assert!(new.is_dir(), "the freshly committed version must survive");
        assert!(!old.exists(), "the previous version directory is removed");
        assert!(
            root.join("apps/finapp/current").is_dir(),
            "current must still resolve"
        );

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }
}
