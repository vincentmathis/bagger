use rayon::prelude::{ParallelBridge, ParallelIterator};
use regex::{Regex, RegexBuilder};
use tracing::{debug, info};

use crate::{
    bucket::Bucket,
    constant::ISOLATED_PACKAGE_BUCKET,
    error::Fallible,
    internal::{self, compare_versions},
    manifest_cache::{FileFingerprint, ManifestCache},
    package::manifest::{InstallInfo, Manifest},
    Error, Session,
};

use super::{InstallState, InstallStateInstalled, Package};

/// Options that may be used to query Scoop packages.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum QueryOption {
    /// Enable query through package binaries.
    Binary,

    /// Enable query through package description.
    Description,

    /// Explicit mode. Regex is disabled in this mode.
    ///
    /// Query will be performed through the package name only. `Description`
    /// and `Binary` options will be ignored.
    Explicit,

    /// Additionally check if the matched package is upgradable.
    ///
    /// This option only takes effect on querying installed packages.
    Upgradable,
}

/// A trait represents a matcher that can be used to do string matching.
trait Matcher {
    fn is_match(&self, s: &str) -> bool;
}

/// A matcher that does explicit match.
///
/// # Note
///
/// This matcher is case-insensitive.
struct ExplicitMatcher<'a>(&'a str);

/// A matcher that does regex match.
struct RegexMatcher(Regex);

impl Matcher for ExplicitMatcher<'_> {
    fn is_match(&self, s: &str) -> bool {
        self.0.eq_ignore_ascii_case(s)
    }
}

impl Matcher for RegexMatcher {
    fn is_match(&self, s: &str) -> bool {
        self.0.is_match(s)
    }
}

/// Search installed packages.
pub(crate) fn query_installed(
    session: &Session,
    queries: &[&str],
    options: &[QueryOption],
) -> Fallible<Vec<Package>> {
    let is_explicit_mode = options.contains(&QueryOption::Explicit);
    let is_wildcard_query = queries.contains(&"*") || queries.is_empty();
    let root_path = session.config().root_path().to_owned();
    let apps_dir = root_path.join("apps");
    // build matchers
    let mut matchers: Vec<(Option<String>, Box<dyn Matcher + Send + Sync>)> = vec![];

    if !is_wildcard_query {
        for query in queries {
            let (bucket_prefix, name) = query
                .split_once('/')
                .map(|(b, n)| (Some(b.to_owned()), n))
                .unwrap_or((None, query));

            if is_explicit_mode {
                matchers.push((bucket_prefix, Box::new(ExplicitMatcher(name))));
            } else {
                let re = RegexBuilder::new(name)
                    .case_insensitive(true)
                    .multi_line(true)
                    .build()?;
                matchers.push((bucket_prefix, Box::new(RegexMatcher(re))));
            }
        }
    }

    let mut ret = vec![];
    match apps_dir.read_dir() {
        Err(err) => {
            debug!("failed to read apps dir (err: {})", err);
        }
        Ok(entries) => {
            ret = entries
                .par_bridge()
                .filter_map(|item| {
                    if let Ok(e) = item {
                        let is_dir = e.file_type().map(|t| t.is_dir()).unwrap_or_default();
                        let filename = e.file_name();
                        let Some(name) = filename.to_str() else {
                            // Skip entries with non-UTF8 names.
                            return None;
                        };
                        // The name `scoop` is reserved for Scoop, ignore it
                        let is_scoop = name == "scoop";
                        let current = e.path().join("current");
                        let manifest_path = super::installed_manifest_path(&current);
                        let install_info_path = super::install_info_path(&current);
                        let is_not_broken = manifest_path.exists() && install_info_path.exists();

                        if !is_dir || is_scoop || !is_not_broken {
                            return None;
                        }

                        // Here we can do some pre-filtering by package name, if there
                        // isn't any wildcard query and no extra query requested on
                        // package description or binaries. This could save some query
                        // time by avoiding parsing manifest and install info files.
                        let extra_query = options.contains(&QueryOption::Binary)
                            || options.contains(&QueryOption::Description);
                        let name_matched = if is_wildcard_query {
                            // name is always matched for wildcard query
                            true
                        } else {
                            matchers.iter().any(|(_, m)| m.is_match(name))
                        };

                        if !is_wildcard_query && !extra_query && !name_matched {
                            return None;
                        }

                        if let Ok(manifest) = Manifest::parse(manifest_path) {
                            if let Ok(install_info) = InstallInfo::parse(install_info_path) {
                                // Noted that packages installed via URLs don't have
                                // bucket info in install info file. We mark them as
                                // isolated packages and use `ISOLATED_PACKAGE_BUCKET`
                                // as bucket name.
                                let bucket =
                                    install_info.bucket().unwrap_or(ISOLATED_PACKAGE_BUCKET);

                                let mut unmatched = true;

                                if is_wildcard_query {
                                    unmatched = false;
                                } else {
                                    let prefixed_name_matched = matchers
                                        .iter()
                                        .filter(|&(_, m)| m.is_match(name))
                                        .any(|(prefix, _)| {
                                            // either no bucket prefix or the bucket
                                            // is also matched.
                                            prefix.is_none() || prefix.as_deref().unwrap() == bucket
                                        });

                                    if prefixed_name_matched {
                                        unmatched = false;
                                    }

                                    if unmatched && !is_explicit_mode {
                                        if options.contains(&QueryOption::Description) {
                                            let description =
                                                manifest.description().unwrap_or_default();
                                            let description_matched = matchers
                                                .iter()
                                                .any(|(_, m)| m.is_match(description));
                                            if description_matched {
                                                unmatched = false;
                                            }
                                        }

                                        if options.contains(&QueryOption::Binary) {
                                            let binaries = manifest.shims().unwrap_or_default();
                                            let binary_matched = matchers.iter().any(|(_, m)| {
                                                binaries.iter().any(|&b| m.is_match(b))
                                            });
                                            if binary_matched {
                                                unmatched = false;
                                            }
                                        }
                                    }
                                }

                                if unmatched {
                                    return None;
                                }

                                let current_version = manifest.version().to_owned();

                                let state = InstallState::Installed(InstallStateInstalled {
                                    version: current_version.clone(),
                                    bucket: install_info.bucket().map(|s| s.to_owned()),
                                    arch: install_info.arch().to_owned(),
                                    held: install_info.is_held(),
                                    url: install_info.url().map(|s| s.to_owned()),
                                });

                                let package = Package::from(name, bucket, manifest);
                                package.fill_install_state(state.clone());

                                // The query has finished, the package has been found
                                // and crafted. We can now apply some extra filters.
                                //
                                // Filter out packages that are not upgradable when
                                // the upgradable option is requested.
                                if options.contains(&QueryOption::Upgradable) {
                                    if bucket == ISOLATED_PACKAGE_BUCKET {
                                        info!("ignored isolated package '{}'", name);
                                        // isolated packages are not upgradable currently,
                                        // we may support it by live checking the origin
                                        // manifest via the path/url in install_info.
                                        return None;
                                    }

                                    let mut bucket_path = root_path.join("buckets");
                                    bucket_path.push(bucket);

                                    let Ok(origin_bucket) = Bucket::from(&bucket_path) else {
                                        // the package is not upgradable because the
                                        // origin bucket is not reachable. This could
                                        // happen when the bucket is removed or renamed.
                                        return None;
                                    };

                                    let Some(origin_manifest_path) =
                                        origin_bucket.path_of_manifest(name)
                                    else {
                                        // the package is not upgradable because
                                        // the origin manifest is not found. This
                                        // could happen when the package is deleted
                                        // or deprecated from the origin bucket.
                                        return None;
                                    };

                                    if let Ok(origin_manifest) =
                                        Manifest::parse(origin_manifest_path)
                                    {
                                        let origin_version = origin_manifest.version();
                                        let is_upgradable =
                                            compare_versions(origin_version, &current_version)
                                                == std::cmp::Ordering::Greater;
                                        if is_upgradable {
                                            let origin_pkg =
                                                Package::from(name, bucket, origin_manifest);
                                            origin_pkg.fill_install_state(state);

                                            package.fill_upgradable(origin_pkg);
                                        } else {
                                            // the package is not upgradable,
                                            // since the upgradable option is
                                            // requested, we should skip it.
                                            return None;
                                        }
                                    }
                                }

                                return Some(package);
                            }
                        }
                    }
                    None
                })
                .collect::<Vec<_>>();
        }
    }

    Ok(ret)
}

/// Search available packages.
pub(crate) fn query_synced(
    session: &Session,
    queries: &[&str],
    options: &[QueryOption],
) -> Fallible<Vec<Package>> {
    let is_explicit_mode = options.contains(&QueryOption::Explicit);
    let is_wildcard_query = queries.contains(&"*") || queries.is_empty();
    let buckets = crate::bucket::bucket_added(session)?;
    let apps_dir = session.config().root_path().join("apps");

    // Open the SQLite manifest cache when enabled. Failures are
    // non-fatal: queries transparently fall back to parsing from disk.
    let cache_dir = session.config().cache_path().to_owned();
    let cache_enabled = session.config().use_sqlite_cache();
    let cache = if cache_enabled {
        match ManifestCache::open(&cache_dir) {
            Ok(cache) => Some(cache),
            Err(err) => {
                debug!(
                    "failed to open manifest cache (err: {}), querying uncached",
                    err
                );
                None
            }
        }
    } else {
        None
    };
    let cache = cache.as_ref();
    // build matchers
    let mut matchers: Vec<(Option<String>, Box<dyn Matcher + Send + Sync>)> = vec![];

    if !is_wildcard_query {
        for query in queries {
            let (bucket_prefix, name) = query
                .split_once('/')
                .map(|(b, n)| (Some(b.to_owned()), n))
                .unwrap_or((None, query));

            if is_explicit_mode {
                matchers.push((bucket_prefix, Box::new(ExplicitMatcher(name))));
            } else {
                let re = RegexBuilder::new(name)
                    .case_insensitive(true)
                    .multi_line(true)
                    .build()?;
                matchers.push((bucket_prefix, Box::new(RegexMatcher(re))));
            }
        }
    }

    let packages = buckets
        .iter()
        .par_bridge()
        .filter_map(|bucket| {
            if let Ok(manifest_files) = bucket.manifests() {
                let bucket_packages = manifest_files
                    .into_iter()
                    .par_bridge()
                    .filter_map(|entry| {
                        let filename = entry.file_name();
                        let name = filename.to_str().and_then(|n| n.strip_suffix(".json"))?;

                        // Here we can do some pre-filtering by package name, if there
                        // isn't any wildcard query and no extra query requested on
                        // package description or binaries. This could save some query
                        // time by avoiding parsing manifest and install info files.
                        let extra_query = options.contains(&QueryOption::Binary)
                            || options.contains(&QueryOption::Description);
                        let name_matched = if is_wildcard_query {
                            // name is always matched for wildcard query
                            true
                        } else {
                            matchers.iter().any(|(_, m)| m.is_match(name))
                        };

                        if !is_wildcard_query && !extra_query && !name_matched {
                            return None;
                        }

                        if let Ok(manifest) =
                            parse_manifest_cached(cache, bucket.name(), name, &entry.path())
                        {
                            let bucket = bucket.name();

                            let mut unmatched = true;

                            if is_wildcard_query {
                                unmatched = false;
                            } else {
                                let prefixed_name_matched = matchers
                                    .iter()
                                    .filter(|&(_, m)| m.is_match(name))
                                    .any(|(prefix, _)| {
                                        // either no bucket prefix or the bucket
                                        // is also matched.
                                        prefix.is_none() || prefix.as_deref().unwrap() == bucket
                                    });

                                if prefixed_name_matched {
                                    unmatched = false;
                                }

                                if unmatched && !is_explicit_mode {
                                    if options.contains(&QueryOption::Description) {
                                        let description =
                                            manifest.description().unwrap_or_default();
                                        let description_matched =
                                            matchers.iter().any(|(_, m)| m.is_match(description));
                                        if description_matched {
                                            unmatched = false;
                                        }
                                    }

                                    if options.contains(&QueryOption::Binary) {
                                        let binaries = manifest.shims().unwrap_or_default();
                                        let binary_matched = matchers
                                            .iter()
                                            .any(|(_, m)| binaries.iter().any(|&b| m.is_match(b)));
                                        if binary_matched {
                                            unmatched = false;
                                        }
                                    }
                                }
                            }

                            if unmatched {
                                return None;
                            }

                            let package = Package::from(name, bucket, manifest);

                            // The query has finished, the package has been found,
                            // the last step is to check if the package is installed.
                            let current = apps_dir.join(name).join("current");

                            if let Ok(install_info) =
                                InstallInfo::parse(super::install_info_path(&current))
                            {
                                if let Ok(install_manifest) =
                                    Manifest::parse(super::installed_manifest_path(&current))
                                {
                                    let state = InstallState::Installed(InstallStateInstalled {
                                        version: install_manifest.version().to_owned(),
                                        bucket: install_info.bucket().map(|s| s.to_owned()),
                                        arch: install_info.arch().to_owned(),
                                        held: install_info.is_held(),
                                        url: install_info.url().map(|s| s.to_owned()),
                                    });
                                    package.fill_install_state(state);
                                }
                            } else {
                                package.fill_install_state(InstallState::NotInstalled);
                            }

                            return Some(package);
                        }
                        None
                    })
                    .collect::<Vec<_>>();

                return Some(bucket_packages);
            }
            None
        })
        .flatten()
        .collect::<Vec<_>>();

    Ok(packages)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isolated_loader_ignores_plain_names() {
        let session = Session::new();
        let result = load_isolated_package(&session, "7zip").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn version_query_splits_only_version_shaped_suffixes() {
        // Plain and bucket-qualified pins.
        assert_eq!(
            split_version_query("gh@2.7.0"),
            ("gh".to_owned(), Some("2.7.0".to_owned()))
        );
        assert_eq!(
            split_version_query("main/gh@2.7.0"),
            ("main/gh".to_owned(), Some("2.7.0".to_owned()))
        );
        // URL and file pins.
        assert_eq!(
            split_version_query("https://example.com/app.json@1.0"),
            (
                "https://example.com/app.json".to_owned(),
                Some("1.0".to_owned())
            )
        );
        assert_eq!(
            split_version_query("C:\\bucket\\app.json@1.0"),
            ("C:\\bucket\\app.json".to_owned(), Some("1.0".to_owned()))
        );
        // Userinfo `@` never splits (the suffix holds slashes).
        assert_eq!(
            split_version_query("https://user@host/app.json"),
            ("https://user@host/app.json".to_owned(), None)
        );
        // Degenerate shapes never split.
        for plain in ["gh", "gh@", "@2.0", "gh@1.0 beta", "C:\\app.json"] {
            assert_eq!(split_version_query(plain), (plain.to_owned(), None));
        }
    }

    /// Commit a manifest version into a scratch bucket repo.
    fn commit_manifest(repo: &git2::Repository, app: &str, version: &str) {
        let workdir = repo.workdir().unwrap();
        let dir = workdir.join("bucket");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{app}.json")),
            format!(
                r#"{{"version": "{version}", "homepage": "https://example.com", "license": "MIT"}}"#
            ),
        )
        .unwrap();
        let mut index = repo.index().unwrap();
        index
            .add_path(std::path::Path::new(&format!("bucket/{app}.json")))
            .unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = git2::Signature::now("t", "t@t").unwrap();
        let head = repo.head().ok().and_then(|h| h.target());
        let parents: Vec<git2::Commit> = head
            .and_then(|id| repo.find_commit(id).ok())
            .into_iter()
            .collect();
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            &format!("{app} {version}"),
            &tree,
            &parent_refs,
        )
        .unwrap();
    }

    /// Git history yields the manifest that carried the pinned version.
    #[test]
    fn history_finds_pinned_manifest() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-pinhistory");
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("root");
        std::fs::create_dir_all(root.join("buckets/pinbucket")).unwrap();
        std::env::set_var("SCOOP", &root);
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));
        let session = Session::new();

        let repo = git2::Repository::init(root.join("buckets/pinbucket")).unwrap();
        commit_manifest(&repo, "pinapp", "1.0");
        commit_manifest(&repo, "pinapp", "2.0");

        let found = find_version_in_history(&session, "pinbucket", "pinapp", "1.0")
            .unwrap()
            .expect("history should hold 1.0");
        assert_eq!(found.version(), "1.0");
        assert_eq!(found.bucket(), "pinbucket");

        assert!(
            find_version_in_history(&session, "pinbucket", "pinapp", "3.0")
                .unwrap()
                .is_none()
        );
        assert!(
            find_version_in_history(&session, "nobucket", "pinapp", "1.0")
                .unwrap()
                .is_none()
        );

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }

    /// Autoupdate generation expands templates for the pinned version and
    /// resolves its hashes offline.
    #[test]
    fn generate_expands_pinned_version() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-pingen");
        let _ = std::fs::remove_dir_all(&base);
        let files = base.join("files");
        std::fs::create_dir_all(&files).unwrap();
        std::fs::write(files.join("tool-1.5.bin"), b"v1.5-bytes").unwrap();

        // Hash doc for the pinned version only (HEAD is 2.0).
        let hash = {
            use bagger_hash::ChecksumBuilder;
            let mut hasher = ChecksumBuilder::new().sha256().build();
            hasher.consume(b"v1.5-bytes");
            format!("sha256:{}", hasher.finalize())
        };
        std::fs::write(
            files.join("hashes-1.5.txt"),
            format!("{hash}  tool-1.5.bin\n"),
        )
        .unwrap();
        let f = |n: &str| {
            format!(
                "file:///{}/{}",
                files.to_string_lossy().replace('\\', "/"),
                n
            )
        };

        std::env::set_var("SCOOP", base.join("root"));
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));
        let session = Session::new();

        let template = format!(
            r#"{{"version": "2.0", "homepage": "https://example.com", "license": "MIT",
                "url": "{u}",
                "autoupdate": {{"url": "{t}", "hash": {{"url": "{h}"}}}}}}"#,
            u = f("tool-2.0.bin"),
            t = f("tool-$version.bin").replace("$version", "$version"),
            h = f("hashes-$version.txt").replace("$version", "$version"),
        );
        let manifest =
            Manifest::parse_bytes(template.as_bytes(), std::path::Path::new("gen.json")).unwrap();

        let pkg =
            generate_version_manifest(&session, "genapp", "genbucket", &manifest, "1.5").unwrap();
        assert_eq!(pkg.version(), "1.5");
        assert_eq!(pkg.bucket(), "genbucket");
        let urls = pkg.download_urls();
        assert_eq!(urls, vec![f("tool-1.5.bin").as_str()]);
        let hashes: Vec<String> = pkg
            .download_hashes()
            .into_iter()
            .map(|h| h.to_string())
            .collect();
        assert_eq!(hashes, vec![hash]);

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn isolated_loader_parses_local_manifest() {
        let dir = std::env::temp_dir().join("bagger-test-isolated");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("mytool.json");
        std::fs::write(
            &path,
            r#"{
                "version": "1.0",
                "homepage": "https://example.com",
                "license": "MIT",
                "url": "https://example.com/mytool.zip",
                "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            }"#,
        )
        .unwrap();

        let session = Session::new();
        let pkg = load_isolated_package(&session, path.to_str().unwrap())
            .unwrap()
            .expect("local manifest should load");
        assert_eq!(pkg.name(), "mytool");
        assert_eq!(pkg.bucket(), ISOLATED_PACKAGE_BUCKET);
        assert_eq!(pkg.version(), "1.0");

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Reproduction: an installed app recording `"architecture": "32bit"`
    /// must be found by installed queries.
    #[test]
    fn installed_query_finds_32bit_record() {
        let _guard = crate::test_support::env_guard();
        let dir = std::env::temp_dir().join("bagger-test-installed-arch");
        let current = dir.join("apps").join("archt").join("current");
        std::fs::create_dir_all(&current).unwrap();
        std::fs::write(
            current.join("manifest.json"),
            r#"{
                "version": "1.0",
                "homepage": "https://example.com",
                "license": "MIT",
                "url": "https://example.com/a.bin",
                "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            }"#,
        )
        .unwrap();
        std::fs::write(
            current.join("install.json"),
            r#"{"architecture": "32bit", "bucket": "fake"}"#,
        )
        .unwrap();

        std::env::set_var("SCOOP", &dir);
        let session = Session::new();
        let pkgs = query_installed(&session, &["*"], &[]).unwrap();
        std::env::remove_var("SCOOP");
        std::fs::remove_dir_all(&dir).ok();

        assert!(
            pkgs.iter().any(|p| p.name() == "archt"),
            "installed archt should be found, got: {:?}",
            pkgs.iter().map(|p| p.name().to_owned()).collect::<Vec<_>>()
        );
    }

    /// Upstream's `scoop-*.json` names resolve like the legacy ones (and
    /// win when both exist).
    #[test]
    fn installed_query_reads_scoped_filenames() {
        let _guard = crate::test_support::env_guard();
        let dir = std::env::temp_dir().join("bagger-test-installed-scoped");
        let _ = std::fs::remove_dir_all(&dir);
        let current = dir.join("apps").join("newapp").join("current");
        std::fs::create_dir_all(&current).unwrap();
        std::fs::write(
            current.join("scoop-manifest.json"),
            r#"{"version": "1.0", "homepage": "https://example.com", "license": "MIT"}"#,
        )
        .unwrap();
        std::fs::write(
            current.join("scoop-install.json"),
            r#"{"architecture": "64bit", "bucket": "fake"}"#,
        )
        .unwrap();

        std::env::set_var("SCOOP", &dir);
        std::env::set_var("SCOOP_GLOBAL", dir.join("global"));
        std::env::set_var("SCOOP_CACHE", dir.join("cache"));
        let session = Session::new();
        let pkgs = query_installed(&session, &["*"], &[]).unwrap();
        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");

        let pkg = pkgs
            .iter()
            .find(|p| p.name() == "newapp")
            .expect("scoop-named install should be found");
        assert_eq!(pkg.installed_version(), Some("1.0"));

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Build a scratch root with one installed app and, optionally, a
    /// bucket manifest at `bucket_version`. Returns the root path; the
    /// caller holds [`crate::test_support::env_guard`] and points `SCOOP`
    /// at it.
    fn installed_fixture(tag: &str, bucket_version: Option<&str>) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("bagger-test-query-{tag}"));
        let current = dir.join("apps").join("qapp").join("current");
        std::fs::create_dir_all(&current).unwrap();
        std::fs::write(
            current.join("manifest.json"),
            r#"{
                "version": "1.0",
                "homepage": "https://example.com",
                "license": "MIT",
                "url": "https://example.com/a.bin",
                "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            }"#,
        )
        .unwrap();
        std::fs::write(
            current.join("install.json"),
            r#"{"architecture": "64bit", "bucket": "qbuk"}"#,
        )
        .unwrap();

        if let Some(version) = bucket_version {
            let bucket_dir = dir.join("buckets").join("qbuk").join("bucket");
            std::fs::create_dir_all(&bucket_dir).unwrap();
            std::fs::write(
                bucket_dir.join("qapp.json"),
                format!(
                    r#"{{"version": "{version}", "homepage": "https://example.com",
                        "license": "MIT", "url": "https://example.com/a.bin",
                        "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"}}"#
                ),
            )
            .unwrap();
        }

        dir
    }

    #[test]
    fn installed_query_matching() {
        let _guard = crate::test_support::env_guard();
        let dir = installed_fixture("matching", None);
        std::env::set_var("SCOOP", &dir);
        let session = Session::new();

        let all = query_installed(&session, &["*"], &[]).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name(), "qapp");

        let explicit = query_installed(&session, &["qapp"], &[QueryOption::Explicit]).unwrap();
        assert_eq!(explicit.len(), 1);

        let missing = query_installed(&session, &["nope"], &[QueryOption::Explicit]).unwrap();
        assert!(missing.is_empty());

        std::env::remove_var("SCOOP");
        std::fs::remove_dir_all(&dir).ok();
    }
    #[test]
    fn installed_query_upgradable_filter() {
        let _guard = crate::test_support::env_guard();
        let dir = installed_fixture("upgradable", Some("2.0"));
        std::env::set_var("SCOOP", &dir);
        let session = Session::new();

        let upgradable = query_installed(&session, &["*"], &[QueryOption::Upgradable]).unwrap();
        assert_eq!(upgradable.len(), 1);
        assert_eq!(upgradable[0].upgradable_version(), Some("2.0"));

        std::env::remove_var("SCOOP");
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn installed_query_upgradable_current_excluded() {
        // Same version in the bucket: filtered out.
        let _guard = crate::test_support::env_guard();
        let dir = installed_fixture("current", Some("1.0"));
        std::env::set_var("SCOOP", &dir);
        let session = Session::new();

        let current = query_installed(&session, &["*"], &[QueryOption::Upgradable]).unwrap();
        assert!(current.is_empty());

        std::env::remove_var("SCOOP");
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Stray non-manifest files in a bucket directory must be ignored,
    /// never panic query_synced.
    #[test]
    fn synced_query_ignores_stray_files() {
        let _guard = crate::test_support::env_guard();
        let dir = std::env::temp_dir().join("bagger-test-query-stray");
        let bucket_dir = dir.join("buckets").join("qbuk").join("bucket");
        std::fs::create_dir_all(&bucket_dir).unwrap();
        std::fs::write(
            bucket_dir.join("qapp.json"),
            r#"{
                "version": "1.0", "homepage": "https://example.com",
                "license": "MIT", "url": "https://example.com/a.bin",
                "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
            }"#,
        )
        .unwrap();
        // Stray files a real bucket directory may contain.
        std::fs::write(bucket_dir.join("README.md"), b"docs").unwrap();
        std::fs::write(bucket_dir.join("package.json"), b"{}").unwrap();

        std::env::set_var("SCOOP", &dir);
        let session = Session::new();
        let pkgs = query_synced(&session, &["*"], &[]).unwrap();
        std::env::remove_var("SCOOP");
        std::fs::remove_dir_all(&dir).ok();

        assert_eq!(pkgs.len(), 1);
        assert_eq!(pkgs[0].name(), "qapp");
    }
}
/// Parse a bucket manifest, serving it from the SQLite cache on hits.
///
/// On a fingerprint (mtime+size) miss the file is read, parsed, and stored
/// back into the cache. With `cache` set to `None` this is a plain
/// [`Manifest::parse`].
fn parse_manifest_cached(
    cache: Option<&ManifestCache>,
    bucket: &str,
    name: &str,
    path: &std::path::Path,
) -> Fallible<Manifest> {
    let Some(cache) = cache else {
        return Manifest::parse(path);
    };

    let fp = FileFingerprint::of(path)?;
    if let Some(json) = cache.get(bucket, name, fp)? {
        // Cached JSON was valid when stored; a parse failure here means the
        // cache row is corrupt, so fall through and re-parse from disk.
        if let Ok(manifest) = Manifest::parse_bytes(&json, path) {
            return Ok(manifest);
        }
    }

    let bytes = std::fs::read(path)?;
    let manifest = Manifest::parse_bytes(&bytes, path)?;
    // Cache write failures are non-fatal for the query itself.
    let _ = cache.put(bucket, name, fp, &bytes);
    Ok(manifest)
}

/// Load an isolated package from a manifest URL or local manifest file.
///
/// Returns `Ok(None)` when `query` is neither an `http(s)://` URL nor an
/// existing local `.json` file, in which case the caller should fall back to
/// regular bucket queries.
///
/// The package name is derived from the URL/file basename (minus the `.json`
/// suffix) and the package is placed in the [`ISOLATED_PACKAGE_BUCKET`]
/// bucket. Dependencies of isolated packages still resolve from buckets.
pub(crate) fn load_isolated_package(session: &Session, query: &str) -> Fallible<Option<Package>> {
    let is_url = query.starts_with("http://") || query.starts_with("https://");
    // Strip a `file://` prefix so users can paste file URLs directly.
    let local_path = query.strip_prefix("file://").unwrap_or(query);
    let is_file = !is_url
        && local_path.to_lowercase().ends_with(".json")
        && std::path::Path::new(local_path).is_file();

    if !is_url && !is_file {
        return Ok(None);
    }

    let (name, origin_display, bytes) = if is_url {
        // Derive the app name from the last URL path segment.
        let without_query = query.split(['?', '#']).next().unwrap_or(query);
        let segment = without_query.rsplit('/').next().unwrap_or("");
        let stem = segment.strip_suffix(".json").or_else(|| {
            segment
                .to_lowercase()
                .strip_suffix(".json")
                .map(|_| &segment[..segment.len() - 5])
        });
        let name = match stem {
            Some(s) if !s.is_empty() => s.to_owned(),
            _ => {
                return Err(Error::Custom(format!(
                    "cannot derive a package name from URL '{query}' (expected it to end with '<name>.json')"
                )));
            }
        };
        if name.contains(['/', '\\']) {
            return Err(Error::Custom(format!(
                "invalid package name derived from URL '{query}'"
            )));
        }

        let proxy = session.config().proxy().map(|s| s.to_owned());
        let content = internal::network::fetch_url(query, proxy.as_deref())
            .ok_or_else(|| Error::Custom(format!("failed to fetch manifest from '{query}'")))?;

        let display = session.config().cache_path().join(format!("{name}.json"));
        (name, display, content.into_bytes())
    } else {
        let path = std::path::Path::new(local_path);
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| Error::Custom(format!("invalid manifest path '{local_path}'")))?;
        let bytes = std::fs::read(path)?;
        (stem.to_owned(), path.to_owned(), bytes)
    };

    let manifest = Manifest::parse_bytes(&bytes, &origin_display)?;
    let package = Package::from(&name, ISOLATED_PACKAGE_BUCKET, manifest);

    // Fill the install state from the apps dir, mirroring `query_synced`.
    let current = session
        .config()
        .root_path()
        .join("apps")
        .join(&name)
        .join("current");

    if let Ok(install_info) = InstallInfo::parse(super::install_info_path(&current)) {
        if let Ok(install_manifest) = Manifest::parse(super::installed_manifest_path(&current)) {
            let state = InstallState::Installed(InstallStateInstalled {
                version: install_manifest.version().to_owned(),
                bucket: install_info.bucket().map(|s| s.to_owned()),
                arch: install_info.arch().to_owned(),
                held: install_info.is_held(),
                url: install_info.url().map(|s| s.to_owned()),
            });
            package.fill_install_state(state);
        }
    } else {
        package.fill_install_state(InstallState::NotInstalled);
    }

    Ok(Some(package))
}

/// Split `app@version` / `bucket/app@version` / `url@version` /
/// `path.json@version` into base + pinned version.
///
/// The `@` split only applies when the suffix is version-shaped (no
/// slashes), so userinfo URLs (`https://user@host/…`) and mails never
/// split.
pub(crate) fn split_version_query(query: &str) -> (String, Option<String>) {
    let Some((base, version)) = query.rsplit_once('@') else {
        return (query.to_owned(), None);
    };
    if base.is_empty()
        || version.is_empty()
        || !version
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | '+' | '_'))
    {
        return (query.to_owned(), None);
    }
    (base.to_owned(), Some(version.to_owned()))
}

/// Search bucket git history for a manifest with `version`, newest first.
///
/// Best-effort history lookup (mirrors the `use_git_history` step of
/// upstream `generate_user_manifest`): non-git buckets, missing refs, and
/// unparsable blobs yield `None` so callers fall back to autoupdate
/// generation. The walk is capped; the newest matching commit wins.
pub(crate) fn find_version_in_history(
    session: &Session,
    bucket: &str,
    app: &str,
    version: &str,
) -> Fallible<Option<Package>> {
    const MAX_COMMITS: usize = 2000;

    let bucket_dir = session.config().root_path().join("buckets").join(bucket);
    let repo = match git2::Repository::open(&bucket_dir) {
        Ok(repo) => repo,
        Err(_) => return Ok(None),
    };
    let mut walk = match repo.revwalk() {
        Ok(walk) => walk,
        Err(_) => return Ok(None),
    };
    if walk.push_head().is_err() {
        return Ok(None);
    }
    walk.set_sorting(git2::Sort::TIME).ok();

    let rel = format!("bucket/{app}.json");
    let rel_path = std::path::Path::new(&rel);
    for id in walk.take(MAX_COMMITS).flatten() {
        let Ok(commit) = repo.find_commit(id) else {
            continue;
        };
        let Ok(tree) = commit.tree() else {
            continue;
        };
        let Ok(entry) = tree.get_path(rel_path) else {
            continue;
        };
        let Ok(obj) = entry.to_object(&repo) else {
            continue;
        };
        let Some(blob) = obj.as_blob() else {
            continue;
        };
        let Ok(json) = serde_json::from_slice::<serde_json::Value>(blob.content()) else {
            continue;
        };
        if json.get("version").and_then(|v| v.as_str()) != Some(version) {
            continue;
        }
        let Ok(manifest) = Manifest::parse_bytes(blob.content(), &bucket_dir.join(&rel)) else {
            continue;
        };
        return Ok(Some(Package::from(app, bucket, manifest)));
    }
    Ok(None)
}

/// Generate a manifest for a pinned `@version` from a template manifest's
/// `autoupdate` section, writing it to the cache `usermanifests` dir.
///
/// The generated package keeps the origin bucket attribution so later
/// upgrades work.
pub(crate) fn generate_version_manifest(
    session: &Session,
    name: &str,
    bucket: &str,
    template: &Manifest,
    version: &str,
) -> Fallible<Package> {
    let scopes = crate::operation::autoupdate_expand(
        session,
        &Package::from(name, bucket, template.clone()),
        version,
    )?;
    if scopes.is_empty() {
        return Err(Error::Custom(format!(
            "cannot install '{name}@{version}': manifest has no autoupdate section to generate it from"
        )));
    }
    let mut manifest_json = serde_json::to_value(template.inner())?;
    for scope in &scopes {
        let url_path: Vec<&str> = scope.url_path.iter().map(String::as_str).collect();
        let hash_path: Vec<&str> = scope.hash_path.iter().map(String::as_str).collect();
        crate::operation::set_json_path(
            &mut manifest_json,
            &url_path,
            crate::operation::string_or_array(&scope.urls),
        );
        crate::operation::set_json_path(
            &mut manifest_json,
            &hash_path,
            crate::operation::string_or_array(&scope.hashes),
        );
    }
    manifest_json["version"] = serde_json::Value::String(version.to_owned());

    let dir = session.config().cache_path().join("usermanifests");
    crate::internal::fs::ensure_dir(&dir)?;
    let path = dir.join(format!("{name}@{version}.json"));
    std::fs::write(&path, serde_json::to_string_pretty(&manifest_json)?)?;
    let manifest = Manifest::parse_bytes(&std::fs::read(&path)?, &path)?;
    Ok(Package::from(name, bucket, manifest))
}

/// Resolve an `app@version` pin to an installable package.
///
/// Returns the bucket manifest itself when versions match, a historical
/// manifest from bucket git history, or an autoupdate-generated manifest.
/// `Ok(None)` means "skip" (a held pin without hold escape, like the other
/// resolve flows); anything unresolvable is an error naming what was tried.
pub(crate) fn resolve_version_pin(
    session: &Session,
    base: &str,
    version: &str,
    escape_hold: bool,
) -> Fallible<Option<Package>> {
    // URL / local-file manifests pin against their own content.
    if let Some(pkg) = load_isolated_package(session, base)? {
        if pkg.is_held() && !escape_hold {
            return Ok(None);
        }
        if pkg.version() == version {
            return Ok(Some(pkg));
        }
        if pkg.manifest().autoupdate().is_some() {
            return generate_version_manifest(
                session,
                pkg.name(),
                ISOLATED_PACKAGE_BUCKET,
                pkg.manifest(),
                version,
            )
            .map(Some);
        }
        return Err(Error::Custom(format!(
            "cannot install '{base}@{version}': manifest version is '{}' with no autoupdate section",
            pkg.version()
        )));
    }

    // Bucket lookup, with optional `bucket/` prefix.
    let (bucket_prefix, name) = base
        .split_once('/')
        .map(|(b, n)| (Some(b), n))
        .unwrap_or((None, base));
    let synced = query_synced(session, &["*"], &[])?;
    let matches: Vec<&Package> = synced
        .iter()
        .filter(|p| {
            p.name().eq_ignore_ascii_case(name)
                && bucket_prefix.map(|b| p.bucket() == b).unwrap_or(true)
        })
        .collect();
    if matches.is_empty() {
        return Err(Error::Custom(format!(
            "cannot install '{base}@{version}': no such package"
        )));
    }
    if matches.len() > 1 {
        let candidates = matches
            .iter()
            .map(|p| p.ident())
            .collect::<Vec<_>>()
            .join(", ");
        return Err(Error::Custom(format!(
            "'{base}' matches multiple buckets ({candidates}); qualify as bucket/app@{version}"
        )));
    }
    let origin = matches[0];

    // Installed + held pins stay held.
    if let Ok(installed) = query_installed(session, &[base], &[]) {
        if installed.iter().any(|p| p.is_held()) && !escape_hold {
            return Ok(None);
        }
    }

    if origin.version() == version {
        return Ok(Some(origin.clone()));
    }
    if session.config().use_git_history() {
        if let Some(pkg) =
            find_version_in_history(session, origin.bucket(), origin.name(), version)?
        {
            return Ok(Some(pkg));
        }
    }
    generate_version_manifest(
        session,
        origin.name(),
        origin.bucket(),
        origin.manifest(),
        version,
    )
    .map(Some)
}
