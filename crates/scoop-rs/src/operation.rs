//! Operations that can be performed on a Scoop instance.
//!
//! This module contains publicly available operations that can be executed on
//! a Scoop session. Certain operations may read or write Scoop's data, hence
//! a session is required to perform these functions.
//!
//! # Note
//!
//! operations with description ending with `*` alter the config.
//!
//! # Examples
//!
//! ```rust
//! use scoop_rs::{Session, operation};
//! let session = Session::new();
//! let buckets = operation::bucket_list(&session).expect("failed to get buckets");
//! println!("{} bucket(s)", buckets.len());
//! ```
use chrono::{SecondsFormat, Utc};
use futures::{executor::ThreadPool, task::SpawnExt};
use std::{
    collections::HashSet,
    iter::FromIterator,
    sync::{Arc, Mutex},
};
use tracing::{debug, info};

use crate::{
    bucket::{Bucket, BucketUpdateProgressContext},
    cache::CacheFile,
    error::{Error, Fallible},
    event::Event,
    internal, package,
    package::{InstallInfo, Package, QueryOption},
    Session, SyncOption,
};

/// Add a bucket to Scoop.
///
/// # Errors
///
/// This method will return an error if the bucket already exists, or the remote
/// url is not specified when adding a non built-in bucket.
///
/// A git error will be returned if failed to clone the bucket.
pub fn bucket_add(session: &Session, name: &str, remote_url: &str) -> Fallible<()> {
    let config = session.config();
    let mut path = config.root_path().to_owned();
    path.push("buckets");

    internal::fs::ensure_dir(&path)?;

    path.push(name);
    if path.exists() {
        return Err(Error::BucketAlreadyExists(name.to_owned()));
    }

    let proxy = config.proxy();
    let remote_url = match remote_url.is_empty() {
        false => remote_url,
        true => crate::constant::BUILTIN_BUCKET_LIST
            .iter()
            .find(|&&(n, _)| n == name)
            .map(|&(_, remote)| remote)
            .ok_or_else(|| Error::BucketAddRemoteRequired(name.to_owned()))?,
    };

    internal::git::clone_repo(remote_url, path, proxy)
}

/// Get a list of added buckets.
///
/// # Returns
///
/// A list of added buckets sorted by name.Buckets cannot be parsed will be
/// filtered out.
///
/// # Errors
///
/// I/O errors will be returned if the `buckets` directory is not readable.
pub fn bucket_list(session: &Session) -> Fallible<Vec<Bucket>> {
    crate::bucket::bucket_added(session).map(|mut buckets| {
        buckets.sort_by_key(|b| b.name().to_owned());
        buckets
    })
}

/// Get a list of known (built-in) buckets.
///
/// # Returns
///
/// A list of known buckets.
pub fn bucket_list_known() -> Vec<(&'static str, &'static str)> {
    crate::constant::BUILTIN_BUCKET_LIST.to_vec()
}

/// Update all added buckets. *
///
/// # Errors
///
/// I/O errors will be returned if the `buckets` directory is not readable or
/// failed to start up the update threads.
///
/// A [`ConfigInUse`][1] error will be returned if the config is borrowed elsewhere.
///
/// [1]: crate::Error::ConfigInUse
pub fn bucket_update(session: &Session) -> Fallible<()> {
    let buckets = crate::bucket::bucket_added(session)?;

    if buckets.is_empty() {
        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::BucketUpdateDone);
        }

        return Ok(());
    }

    // Doing bucket update will update the last_update timestamp in the config.
    // A mutable reference to the config is borrowed here.
    let mut config = session.config_mut()?;
    let any_bucket_updated = Arc::new(Mutex::new(false));
    let mut tasks = Vec::new();
    let pool = ThreadPool::builder().create()?;
    let proxy = config.proxy().map(|s| s.to_owned());
    let emitter = session.emitter();

    for bucket in buckets.iter() {
        let repo = bucket.path().to_owned();

        // There is no remote url for this bucket, so we just ignore it.
        if bucket.remote_url().is_none() {
            info!("ignored non-updatable bucket '{}'", bucket.name());
            continue;
        }

        let name = bucket.name().to_owned();
        let flag = Arc::clone(&any_bucket_updated);
        let proxy = proxy.clone();
        let emitter = emitter.clone();

        let task = pool
            .spawn_with_handle(async move {
                let mut ctx = BucketUpdateProgressContext::new(name.as_str());

                if let Some(tx) = emitter.clone() {
                    let _ = tx.send(Event::BucketUpdateProgress(ctx.clone()));
                }

                match internal::git::reset_head(repo, proxy) {
                    Ok(_) => {
                        *flag.lock().unwrap() = true;

                        if let Some(tx) = emitter {
                            ctx.set_succeeded();
                            let _ = tx.send(Event::BucketUpdateProgress(ctx));
                        }
                    }
                    Err(err) => {
                        if let Some(tx) = emitter {
                            ctx.set_failed(err.to_string().as_str());
                            let _ = tx.send(Event::BucketUpdateProgress(ctx));
                        }
                    }
                };
            })
            .map_err(|e| Error::Custom(e.to_string()))?;
        tasks.push(task);
    }

    let joined = futures::future::join_all(tasks);
    futures::executor::block_on(joined);

    if *any_bucket_updated.lock().unwrap() {
        let time = Utc::now().to_rfc3339_opts(SecondsFormat::Micros, false);
        config.set("last_update", time.as_str())?;
    }

    if let Some(tx) = emitter {
        let _ = tx.send(Event::BucketUpdateDone);
    }
    Ok(())
}

/// Remove a bucket from Scoop.
///
/// # Errors
///
/// This method will return an error if the bucket does not exist. I/O errors
/// will be returned if the bucket directory is unable to be removed.
pub fn bucket_remove(session: &Session, name: &str) -> Fallible<()> {
    let mut path = session.config().root_path().to_owned();
    path.push("buckets");
    path.push(name);

    if !path.exists() {
        return Err(Error::BucketNotFound(name.to_owned()));
    }

    Ok(remove_dir_all::remove_dir_all(path.as_path())?)
}

/// Get a list of downloaded cache files.
///
/// # Returns
///
/// A list of downloaded cache files.
///
/// # Errors
///
/// I/O errors will be returned if the cache directory is not readable.
pub fn cache_list(session: &Session, query: &str) -> Fallible<Vec<CacheFile>> {
    let is_wildcard_query = query.eq("*") || query.is_empty();
    let config = session.config();
    let cache_dir = config.cache_path();
    let mut files = vec![];

    match cache_dir.read_dir() {
        Err(err) => {
            debug!("failed to read cache dir (err: {})", err);
        }
        Ok(entires) => {
            files = entires
                .filter_map(|de| {
                    if let Ok(entry) = de {
                        let is_file = entry.file_type().unwrap().is_file();
                        if is_file {
                            if let Ok(item) = CacheFile::from(entry.path()) {
                                if !is_wildcard_query {
                                    let matched = item
                                        .package_name()
                                        .to_lowercase()
                                        .contains(&query.to_lowercase());
                                    if matched {
                                        return Some(item);
                                    } else {
                                        return None;
                                    }
                                }

                                return Some(item);
                            }
                        }
                    }
                    None
                })
                .collect::<Vec<_>>();
        }
    }

    Ok(files)
}

/// Remove cache files by query.
///
/// # Errors
///
/// I/O errors will be returned if the cache directory is not readable or failed
/// to remove the cache files.
pub fn cache_remove(session: &Session, query: &str) -> Fallible<()> {
    match query {
        "*" => {
            let config = session.config();
            Ok(internal::fs::empty_dir(config.cache_path())?)
        }
        query => {
            let files = cache_list(session, query)?;
            for f in files.into_iter() {
                std::fs::remove_file(f.path())?;
            }
            Ok(())
        }
    }
}

/// Get a list of running process names that belong to a package's app directory.
///
/// # Errors
///
/// I/O errors will be returned if the process enumeration fails.
pub fn running_processes(session: &Session, package: &Package) -> Fallible<Vec<String>> {
    let config = session.config();
    let apps_dir = config.root_path().join("apps");
    let app_path = apps_dir.join(package.name());
    internal::os::running_apps(&app_path)
}

/// Get the installation directory of a package.
///
/// # Returns
///
/// The path to the package's install directory. If `no_junction` is set,
/// this is `<root>/apps/<name>/<version>`. Otherwise, it is
/// `<root>/apps/<name>/current`.
pub fn install_dir(session: &Session, package: &Package) -> std::path::PathBuf {
    let config = session.config();
    let apps_dir = std::path::PathBuf::from(config.root_path()).join("apps");

    if config.no_junction() {
        apps_dir
            .join(package.name())
            .join(package.installed_version().unwrap_or(package.version()))
    } else {
        apps_dir.join(package.name()).join("current")
    }
}

/// Result of a checkver operation.
#[derive(Clone, Debug)]
pub struct CheckverResult {
    pub current_version: Option<String>,
    pub latest_version: Option<String>,
    /// Regex captures from the version match (`0` = whole match, then
    /// numbered and named groups), for `replace` templates and autoupdate
    /// `$match*` variables.
    pub captures: Vec<(String, String)>,
}

impl CheckverResult {
    pub fn is_upgradable(&self) -> bool {
        self.current_version
            .as_ref()
            .zip(self.latest_version.as_ref())
            .map(|(cur, latest)| cur != latest)
            .unwrap_or(false)
    }
}

/// Check the latest version of a package by fetching its checkver URL.
///
/// # Returns
///
/// A [`CheckverResult`] containing the current and latest versions.
///
/// # Errors
///
/// Network errors will be returned if the checkver URL is not fetchable.
pub fn checkver(session: &Session, package: &Package) -> Fallible<CheckverResult> {
    let config = session.config();
    let proxy = config.proxy();

    let manifest = package.manifest();
    let checkver = manifest.effective_checkver();
    let current = package
        .installed_version()
        .unwrap_or(package.version())
        .to_string();

    let reverse = checkver.and_then(|c| c.reverse).unwrap_or(false);
    let replace = checkver.and_then(|c| c.replace.as_deref());
    let regex = checkver.and_then(|c| c.regex.as_deref());

    // `checkver.script` runs a PowerShell snippet expected to print the
    // latest version. It replaces page fetching entirely.
    if let Some(script) = checkver.and_then(|c| c.script.as_ref()) {
        let lines = script.devectorize();
        let fallback_cwd = session.config().cache_path().to_owned();
        let working_dir = manifest.path().parent().unwrap_or(&fallback_cwd);
        let stdout =
            internal::ps::invoke_script_capture(session, package, "checkver", &lines, working_dir)?;

        let (latest_version, captures) = match match_version(&stdout, regex, reverse, replace) {
            Some(m) => (Some(m.text.clone()), m.captures.clone()),
            None => (
                stdout
                    .lines()
                    .map(str::trim)
                    .find(|line| !line.is_empty())
                    .map(|line| line.to_owned()),
                vec![],
            ),
        };

        return Ok(CheckverResult {
            current_version: Some(current),
            latest_version,
            captures,
        });
    }

    let url = checkver
        .and_then(|c| c.url.as_deref())
        .unwrap_or_else(|| manifest.homepage())
        .to_string();

    // `checkver.useragent` wins, then the session user agent.
    let user_agent = checkver
        .and_then(|c| c.useragent.as_deref())
        .or_else(|| session.user_agent.get().map(|s| s.as_str()));
    let content = match user_agent {
        Some(ua) => {
            internal::network::fetch_url_with_headers(&url, proxy, &[("User-Agent", ua)])
                .map(|(_, body)| body)
        }
        None => internal::network::fetch_url(&url, proxy),
    };
    let content = match content {
        Some(c) => c,
        None => {
            return Ok(CheckverResult {
                current_version: Some(current),
                latest_version: None,
                captures: vec![],
            });
        }
    };

    // Per Scoop semantics, `jsonpath`/`xpath` first extract a string that
    // `regex` is then matched against. Without `regex`, the extracted
    // string itself is the version.
    let extracted = checkver
        .and_then(|c| c.jsonpath.as_deref())
        .and_then(|jsonpath| eval_jsonpath(&content, jsonpath))
        .or_else(|| {
            checkver
                .and_then(|c| c.xpath.as_deref())
                .and_then(|xpath| eval_xpath(&content, xpath))
        });

    let haystack = extracted.as_deref().unwrap_or(&content);
    let (latest_version, captures) = match match_version(haystack, regex, reverse, replace) {
        Some(m) => (Some(m.text.clone()), m.captures.clone()),
        // No regex given: the extracted string (if any) is the version.
        None => (extracted, vec![]),
    };

    Ok(CheckverResult {
        current_version: Some(current),
        latest_version,
        captures,
    })
}

/// A regex version match with its captures retained.
#[derive(Clone, Debug)]
pub struct VersionMatch {
    /// The matched version text (group 1 preferred, else whole match,
    /// optionally rewritten by the `replace` template).
    pub text: String,
    /// Captures as `(name, value)`: `0` is the whole match, then numbered
    /// groups (`1`, `2`, …) and named groups (`version`, …).
    pub captures: Vec<(String, String)>,
}

/// Match `regex` against `haystack`, honoring `reverse` (last match wins)
/// and applying the `replace` template when given.
fn match_version(
    haystack: &str,
    regex: Option<&str>,
    reverse: bool,
    replace: Option<&str>,
) -> Option<VersionMatch> {
    let re = regex::Regex::new(regex?).ok()?;
    let caps = match reverse {
        true => re.captures_iter(haystack).last()?,
        false => re.captures(haystack)?,
    };

    let mut captures = vec![];
    if let Some(whole) = caps.get(0) {
        captures.push(("0".to_owned(), whole.as_str().to_owned()));
    }
    for (idx, group) in caps.iter().enumerate().skip(1) {
        if let Some(m) = group {
            captures.push((idx.to_string(), m.as_str().to_owned()));
        }
    }
    for name in re.capture_names().flatten() {
        if let Some(m) = caps.name(name) {
            captures.push((name.to_owned(), m.as_str().to_owned()));
        }
    }

    let text = caps
        .get(1)
        .or_else(|| caps.get(0))
        .map(|m| m.as_str().to_owned())?;

    let text = match replace {
        Some(template) => expand_replace(template, &captures),
        None => text,
    };

    Some(VersionMatch { text, captures })
}

/// Expand a .NET-style replacement template (`$1`, `$name`, `${name}`;
/// `$$` escapes to `$`) using regex captures.
fn expand_replace(template: &str, captures: &[(String, String)]) -> String {
    let lookup = |key: &str| -> &str {
        captures
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
            .unwrap_or("")
    };

    let mut out = String::with_capacity(template.len());
    let mut chars = template.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '$' {
            out.push(ch);
            continue;
        }
        match chars.peek() {
            // `$$` escapes to a literal `$`.
            Some('$') => {
                out.push('$');
                chars.next();
            }
            // `${name}` form.
            Some('{') => {
                chars.next();
                let name: String = chars.by_ref().take_while(|&c| c != '}').collect();
                out.push_str(lookup(&name));
            }
            // `$1` / `$name` forms.
            Some(c) if c.is_ascii_alphanumeric() || *c == '_' => {
                let mut name = String::new();
                while let Some(&c) = chars.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        name.push(c);
                        chars.next();
                    } else {
                        break;
                    }
                }
                out.push_str(lookup(&name));
            }
            // Lone `$` stays literal.
            _ => out.push('$'),
        }
    }
    out
}

/// Expand autoupdate URL templates.
///
/// Supports `$version` plus `$match*` captured variables (`$match1`,
/// `$matchHead`, …) from the checkver regex match. Longer names are
/// substituted first so `$match1` never clobbers `$match10`.
pub fn expand_autoupdate_template(
    template: &str,
    version: &str,
    captures: &[(String, String)],
) -> String {
    let mut vars: Vec<(String, &str)> = vec![("version".to_owned(), version)];
    for (name, value) in captures {
        vars.push((format!("match{name}"), value.as_str()));
    }
    vars.sort_by_key(|a| std::cmp::Reverse(a.0.len()));

    let mut out = template.to_owned();
    for (name, value) in vars {
        out = out.replace(&format!("${name}"), value);
    }
    out
}

/// Outcome of [`autoupdate_apply`].
#[derive(Clone, Debug, Default)]
pub struct AutoupdateResult {
    /// The new version that was applied (when `write`).
    pub version: String,
    /// Expanded URLs that were downloaded and hashed.
    pub rewritten_urls: Vec<String>,
    /// `sha256:…` hashes computed for the rewritten URLs.
    pub rewritten_hashes: Vec<String>,
    /// Scopes left untouched, with reasons.
    pub skipped: Vec<String>,
    /// Whether the manifest file was rewritten.
    pub wrote: bool,
}

/// Expand autoupdate URLs for `latest`, hash the downloads, and optionally
/// rewrite the bucket manifest.
///
/// Only `download`-mode hashes (or a missing `hash` section, which defaults
/// to download mode) are handled; other modes are reported in
/// [`AutoupdateResult::skipped`]. URL/hash shapes must match the manifest
/// (single string vs same-length array) or the scope is skipped rather than
/// risk corrupting the manifest.
///
/// # Errors
///
/// Network errors will be returned if an expanded URL cannot be fetched.
pub fn autoupdate_apply(
    session: &Session,
    package: &Package,
    latest: &str,
    captures: &[(String, String)],
    write: bool,
) -> Fallible<AutoupdateResult> {
    use crate::package::manifest::{HashExtraction, Vectorized};

    let manifest = package.manifest();
    let mut result = AutoupdateResult {
        version: latest.to_owned(),
        ..Default::default()
    };

    let Some(autoupdate) = manifest.autoupdate() else {
        result.skipped.push("no autoupdate section".to_owned());
        return Ok(result);
    };

    let proxy = session.config().proxy().map(|s| s.to_owned());

    // One manifest URL/hash section processed below.
    struct AutoupdateScope<'a> {
        label: &'static str,
        url_path: Vec<&'static str>,
        hash_path: Vec<&'static str>,
        templates: Vec<String>,
        hash_specs: Option<&'a Vectorized<HashExtraction>>,
    }
    let mut scopes: Vec<AutoupdateScope<'_>> = vec![];

    if let Some(urls) = autoupdate.url.as_ref() {
        scopes.push(AutoupdateScope {
            label: "noarch",
            url_path: vec!["url"],
            hash_path: vec!["hash"],
            templates: urls.devectorize().into_iter().map(|s| s.to_owned()).collect(),
            hash_specs: autoupdate.hash.as_ref(),
        });
    }

    if let Some(arch) = autoupdate.architecture.as_ref() {
        for (label, url_key, spec) in [
            ("32bit", "32bit", arch.ia32.as_ref()),
            ("64bit", "64bit", arch.amd64.as_ref()),
            ("arm64", "arm64", arch.aarch64.as_ref()),
        ] {
            if let Some(spec) = spec {
                if let Some(urls) = spec.url.as_ref() {
                    scopes.push(AutoupdateScope {
                        label,
                        url_path: vec!["architecture", url_key, "url"],
                        hash_path: vec!["architecture", url_key, "hash"],
                        templates: urls
                            .devectorize()
                            .into_iter()
                            .map(|s| s.to_owned())
                            .collect(),
                        hash_specs: spec.hash.as_ref().or(autoupdate.hash.as_ref()),
                    });
                }
            }
        }
    }

    if scopes.is_empty() {
        result.skipped.push("autoupdate has no URL templates".to_owned());
    }

    // Load the raw manifest JSON once for shape checks and rewriting.
    let manifest_path = manifest.path().to_owned();
    let mut manifest_json: serde_json::Value = if write {
        let raw = std::fs::read_to_string(&manifest_path)?;
        serde_json::from_str(&raw)?
    } else {
        serde_json::Value::Null
    };

    for scope in &scopes {
        let AutoupdateScope {
            label,
            url_path,
            hash_path,
            templates,
            hash_specs,
        } = scope;
        let mode_is_download = hash_specs
            .and_then(|specs| specs.devectorize().first().cloned())
            .map(|h| {
                matches!(
                    &h.mode,
                    None | Some(crate::package::manifest::HashExtractionMode::Download)
                )
            })
            .unwrap_or(true);

        if !mode_is_download {
            result.skipped.push(format!(
                "{label}: non-download hash mode is not automated"
            ));
            continue;
        }

        let expanded: Vec<String> = templates
            .iter()
            .map(|t| expand_autoupdate_template(t, latest, captures))
            .collect();

        // Shape check against the current manifest (only meaningful — and
        // only performed — when writing).
        if write {
            let current_urls = url_path
                .iter()
                .try_fold(&manifest_json, |v, k| v.get(*k))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let shape_ok = match &current_urls {
                serde_json::Value::String(_) => expanded.len() == 1,
                serde_json::Value::Array(arr) => arr.len() == expanded.len(),
                // Missing URL section: don't invent structure.
                _ => false,
            };
            if !shape_ok {
                result.skipped.push(format!(
                    "{label}: template count ({}) does not match manifest URLs; left untouched",
                    expanded.len()
                ));
                continue;
            }
        }

        let mut hashes = vec![];
        for url in &expanded {
            let bytes = internal::network::fetch_bytes(url, proxy.as_deref()).ok_or_else(|| {
                Error::Custom(format!("failed to download autoupdate URL '{url}'"))
            })?;
            let mut hasher = bagger_hash::ChecksumBuilder::new().sha256().build();
            hasher.consume(&bytes);
            hashes.push(format!("sha256:{}", hasher.finalize()));
        }

        result.rewritten_urls.extend(expanded.iter().cloned());
        result.rewritten_hashes.extend(hashes.iter().cloned());

        if write {
            set_json_path(&mut manifest_json, url_path, string_or_array(&expanded));
            set_json_path(&mut manifest_json, hash_path, string_or_array(&hashes));
        }
    }

    if write {
        manifest_json["version"] = serde_json::Value::String(latest.to_owned());
        std::fs::write(
            &manifest_path,
            serde_json::to_string_pretty(&manifest_json)?,
        )?;
        result.wrote = true;
    }

    Ok(result)
}

/// Build a JSON string or array of strings, mirroring manifest shape.
fn string_or_array(values: &[String]) -> serde_json::Value {
    if values.len() == 1 {
        serde_json::Value::String(values[0].clone())
    } else {
        serde_json::Value::Array(
            values
                .iter()
                .map(|s| serde_json::Value::String(s.clone()))
                .collect(),
        )
    }
}

/// Set a nested value in manifest JSON, creating intermediate objects.
fn set_json_path(root: &mut serde_json::Value, path: &[&str], value: serde_json::Value) {
    let mut current = root;
    for (idx, key) in path.iter().enumerate() {
        if idx + 1 == path.len() {
            current[*key] = value;
            return;
        }
        if !current.get(*key).map(|v| v.is_object()).unwrap_or(false) {
            current[*key] = serde_json::Value::Object(serde_json::Map::new());
        }
        current = &mut current[*key];
    }
}

/// Evaluate a minimal JSONPath expression against a JSON document.
///
/// Supports the Scoop-flavored subset used by `checkver.jsonpath`: a `$`
/// root followed by dot-separated object keys with optional `[n]` array
/// indices (e.g. `$.tag_name`, `$.releases[0].version`). Only scalar results
/// (strings, numbers, booleans) yield a value.
fn eval_jsonpath(content: &str, path: &str) -> Option<String> {
    let rest = path.trim().strip_prefix('$')?;
    let rest = rest.strip_prefix('.').unwrap_or(rest);

    let json: serde_json::Value = serde_json::from_str(content).ok()?;
    if rest.is_empty() {
        return jsonpath_scalar(&json);
    }

    let mut current = &json;
    for part in rest.split('.') {
        let (key, bracketed) = match part.find('[') {
            Some(idx) => (&part[..idx], &part[idx..]),
            None => (part, ""),
        };

        if !key.is_empty() {
            current = current.get(key)?;
        }

        let mut rest = bracketed;
        while let Some(inner) = rest.strip_prefix('[') {
            let end = inner.find(']')?;
            let index: usize = inner[..end].parse().ok()?;
            current = current.get(index)?;
            rest = &inner[end + 1..];
        }
        if !rest.is_empty() {
            return None;
        }
    }

    jsonpath_scalar(current)
}

/// Render a JSON scalar as a version string.
fn jsonpath_scalar(value: &serde_json::Value) -> Option<String> {
    match value {
        serde_json::Value::String(s) => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// A parsed step of the XPath subset supported by [`eval_xpath`].
enum XPathStep<'a> {
    /// Element test with optional position/attribute filters.
    Element {
        /// `true` for `//` (descendant) steps, `false` for `/` (child) steps.
        descendant: bool,
        /// Tag name to match, or `None` for the `*` wildcard.
        tag: Option<&'a str>,
        /// 1-based position filter (`tag[n]`), if any.
        position: Option<usize>,
        /// Attribute predicate (`tag[@attr='value']`), if any.
        attr_filter: Option<(&'a str, &'a str)>,
    },
    /// Trailing `text()` step (element text content).
    Text,
    /// Trailing `@attr` step (attribute value).
    Attribute(&'a str),
}

/// Evaluate a small XPath subset against an XML document.
///
/// Supported steps, joined by `/` (child) or `//` (descendant search):
/// `tag`, `*`, `tag[n]` (1-based), `tag[@attr='value']`, plus a trailing
/// `text()` (element text) or `@attr` (attribute value) step. Returns the
/// first matching string value, if any.
fn eval_xpath(content: &str, path: &str) -> Option<String> {
    let path = path.trim();
    if path.is_empty() || path.starts_with("string(") {
        return None;
    }

    // Split into steps, tracking the `/` vs `//` axis of each.
    let mut steps: Vec<XPathStep<'_>> = vec![];
    let mut rest = path;
    let mut first = true;
    while !rest.is_empty() {
        let (descendant, step) = if let Some(s) = rest.strip_prefix("//") {
            (true, s)
        } else if let Some(s) = rest.strip_prefix('/') {
            (false, s)
        } else if first {
            (false, rest)
        } else {
            return None;
        };
        first = false;

        // A step ends at the next `/` outside brackets and quotes
        // (attribute values such as MIME types may contain `/`).
        let (head, tail) = split_xpath_step(step);
        if head.is_empty() {
            return None;
        }
        steps.push(parse_xpath_step(head, descendant)?);
        rest = tail;
    }
    if steps.is_empty() {
        return None;
    }

    let doc = roxmltree::Document::parse(content).ok()?;

    let mut current: Vec<roxmltree::Node<'_, '_>> = vec![doc.root()];
    for (idx, step) in steps.iter().enumerate() {
        let is_last = idx + 1 == steps.len();
        match step {
            XPathStep::Element {
                descendant,
                tag,
                position,
                attr_filter,
            } => {
                current =
                    apply_xpath_element(&current, *descendant, *tag, *position, *attr_filter);
            }
            XPathStep::Text => {
                if !is_last {
                    return None;
                }
                return first_non_empty(current.iter().filter_map(xpath_text_content));
            }
            XPathStep::Attribute(attr) => {
                if !is_last {
                    return None;
                }
                return first_non_empty(
                    current
                        .iter()
                        .filter_map(|n| n.attribute(*attr))
                        .map(str::trim),
                );
            }
        }
        if current.is_empty() {
            return None;
        }
    }

    // A bare element path yields the first element's text content.
    first_non_empty(current.iter().filter_map(xpath_text_content))
}

/// Apply one element step to a node set.
fn apply_xpath_element<'a, 'input>(
    nodes: &[roxmltree::Node<'a, 'input>],
    descendant: bool,
    tag: Option<&str>,
    position: Option<usize>,
    attr_filter: Option<(&str, &str)>,
) -> Vec<roxmltree::Node<'a, 'input>> {
    let matches = |n: &roxmltree::Node<'_, '_>| {
        tag.map(|t| n.tag_name().name() == t).unwrap_or(true)
            && attr_filter
                .map(|(attr, value)| n.attribute(attr) == Some(value))
                .unwrap_or(true)
    };

    let mut out = vec![];
    if descendant {
        // `//tag`: all matching descendants in document order.
        for node in nodes {
            out.extend(
                node.descendants()
                    .filter(|n| n.is_element())
                    .filter(matches),
            );
        }
        if let Some(n) = position {
            out = out.into_iter().skip(n - 1).take(1).collect();
        }
    } else {
        // `tag`: matching element children, `[n]` per parent.
        for node in nodes {
            let mut children = node
                .children()
                .filter(|n| n.is_element())
                .filter(matches)
                .peekable();
            if children.peek().is_none() {
                continue;
            }
            match position {
                Some(n) => out.extend(children.skip(n - 1).take(1)),
                None => out.extend(children),
            }
        }
    }
    out
}

/// Concatenated text content of an element's descendants.
///
/// Note: `descendants()` includes the node itself, and `text()` on an
/// element returns its first text child, so only `Text` nodes are collected
/// to avoid double counting.
fn xpath_text_content(node: &roxmltree::Node<'_, '_>) -> Option<String> {
    let mut text = String::new();
    for descendant in node.descendants() {
        if descendant.is_text() {
            if let Some(t) = descendant.text() {
                text.push_str(t);
            }
        }
    }
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

/// First non-empty trimmed string from an iterator.
fn first_non_empty(iter: impl Iterator<Item = impl AsRef<str>>) -> Option<String> {
    iter.map(|s| s.as_ref().trim().to_owned())
        .find(|s| !s.is_empty())
}

/// Split the leading XPath step from the remainder.
///
/// The step ends at the next `/` that appears outside `[...]` brackets and
/// outside single/double quotes, since attribute values may contain `/`
/// (e.g. `[@type='application/zip']`).
fn split_xpath_step(step: &str) -> (&str, &str) {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    for (idx, ch) in step.char_indices() {
        match quote {
            Some(q) => {
                if ch == q {
                    quote = None;
                }
            }
            None => match ch {
                '\'' | '"' => quote = Some(ch),
                '[' => depth += 1,
                ']' => depth = depth.saturating_sub(1),
                '/' if depth == 0 => return step.split_at(idx),
                _ => {}
            },
        }
    }
    (step, "")
}

/// Parse one XPath step (without any leading `/`).
fn parse_xpath_step(step: &str, descendant: bool) -> Option<XPathStep<'_>> {
    if step == "text()" {
        return Some(XPathStep::Text);
    }
    if let Some(attr) = step.strip_prefix('@') {
        if !attr.is_empty() && !attr.contains(['[', ']', '/', '\'', '"']) {
            return Some(XPathStep::Attribute(attr));
        }
        return None;
    }

    // Bracketed suffix: `[n]` or `[@attr='value']`.
    let (head, bracket) = match step.find('[') {
        Some(idx) => {
            let (h, b) = step.split_at(idx);
            if !b.ends_with(']') {
                return None;
            }
            (h, Some(&b[1..b.len() - 1]))
        }
        None => (step, None),
    };

    let mut tag: Option<&str> = None;
    let mut position: Option<usize> = None;
    let mut attr_filter: Option<(&str, &str)> = None;

    match head {
        "" | "*" => {}
        tag_name => tag = Some(tag_name),
    }

    if let Some(pred) = bracket {
        if let Ok(num) = pred.parse::<usize>() {
            if num == 0 {
                return None;
            }
            position = Some(num);
        } else {
            let inner = pred.strip_prefix('@')?;
            let (attr, value) = inner.split_once('=')?;
            let value = value.trim();
            let unquoted = value
                .strip_prefix('\'')
                .and_then(|v| v.strip_suffix('\''))
                .or_else(|| {
                    value
                        .strip_prefix('"')
                        .and_then(|v| v.strip_suffix('"'))
                })?;
            attr_filter = Some((attr.trim(), unquoted));
        }
    }

    Some(XPathStep::Element {
        descendant,
        tag,
        position,
        attr_filter,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        eval_jsonpath, eval_xpath, expand_autoupdate_template, expand_replace, match_version,
    };
    use crate::package::manifest::Manifest;
    use crate::package::Package;
    use crate::Session;

    #[test]
    fn jsonpath_object_keys() {
        let doc = r#"{"tag_name": "v1.2.3"}"#;
        assert_eq!(eval_jsonpath(doc, "$.tag_name").as_deref(), Some("v1.2.3"));
    }

    #[test]
    fn jsonpath_nested_with_index() {
        let doc = r#"{"releases": [{"version": "2.0"}, {"version": "1.0"}]}"#;
        assert_eq!(
            eval_jsonpath(doc, "$.releases[0].version").as_deref(),
            Some("2.0")
        );
    }

    #[test]
    fn jsonpath_scalars_and_misses() {
        let doc = r#"{"build": 42, "stable": true, "items": [1, 2]}"#;
        assert_eq!(eval_jsonpath(doc, "$.build").as_deref(), Some("42"));
        assert_eq!(eval_jsonpath(doc, "$.stable").as_deref(), Some("true"));
        assert_eq!(eval_jsonpath(doc, "$.missing"), None);
        assert_eq!(eval_jsonpath(doc, "$.items"), None);
        assert_eq!(eval_jsonpath(doc, "tag_name"), None);
        assert_eq!(eval_jsonpath(doc, "$.items[9]"), None);
    }

    const FEED: &str = r#"<?xml version="1.0"?>
        <rss version="2.0">
          <channel>
            <title>Example</title>
            <item><title>Release 2.0</title><enclosure url="https://example.com/a-2.0.zip" type="application/zip"/></item>
            <item><title>Release 1.0</title><enclosure url="https://example.com/a-1.0.zip" type="application/zip"/></item>
          </channel>
        </rss>"#;

    #[test]
    fn xpath_absolute_and_descendant() {
        assert_eq!(
            eval_xpath(FEED, "/rss/channel/title").as_deref(),
            Some("Example")
        );
        assert_eq!(
            eval_xpath(FEED, "//item/title").as_deref(),
            Some("Release 2.0")
        );
    }

    #[test]
    fn xpath_position_attribute_and_text() {
        assert_eq!(
            eval_xpath(FEED, "//item[2]/title/text()").as_deref(),
            Some("Release 1.0")
        );
        assert_eq!(
            eval_xpath(FEED, "//enclosure[@type='application/zip']/@url").as_deref(),
            Some("https://example.com/a-2.0.zip")
        );
    }

    #[test]
    fn xpath_misses() {
        assert_eq!(eval_xpath(FEED, "//missing"), None);
        assert_eq!(eval_xpath(FEED, "//item[9]/title"), None);
        assert_eq!(eval_xpath(FEED, "//item[0]/title"), None);
        assert_eq!(eval_xpath(FEED, "not a path !!!"), None);
        assert_eq!(eval_xpath("not xml at all", "//item"), None);
    }

    #[test]
    fn version_match_first_and_reverse() {
        let page = "dl v1.0 dl v2.0";
        let first = match_version(page, Some("v([\\d.]+)"), false, None).unwrap();
        assert_eq!(first.text, "1.0");
        let last = match_version(page, Some("v([\\d.]+)"), true, None).unwrap();
        assert_eq!(last.text, "2.0");
        assert!(match_version(page, None, false, None).is_none());
    }

    #[test]
    fn version_replace_templates() {
        let page = "sysinternals suite 2024-06-01";
        let m = match_version(
            page,
            Some("suite (?<year>\\d{4})-(?<rest>\\d{2}-\\d{2})"),
            false,
            Some("$year.$rest"),
        )
        .unwrap();
        assert_eq!(m.text, "2024.06-01");

        // Numbered groups, ${} form and $$ escaping.
        assert_eq!(
            expand_replace("${1}-x", &[("1".to_owned(), "2.0".to_owned())]),
            "2.0-x"
        );
        assert_eq!(
            expand_replace("$$1 $9", &[("1".to_owned(), "2.0".to_owned())]),
            "$1 "
        );
    }

        #[test]
    fn autoupdate_template_expansion() {        let captures = vec![
            ("0".to_owned(), "v2.0".to_owned()),
            ("1".to_owned(), "2.0".to_owned()),
            ("tag".to_owned(), "v2.0".to_owned()),
        ];
        assert_eq!(
            expand_autoupdate_template(
                "https://example.com/$version/app-$match1-$matchtag.zip",
                "2.0",
                &captures
            ),
            "https://example.com/2.0/app-2.0-v2.0.zip"
        );
        // `$match1` must not clobber `$match10`.
        let captures = vec![
            ("1".to_owned(), "a".to_owned()),
            ("10".to_owned(), "b".to_owned()),
        ];
        assert_eq!(
            expand_autoupdate_template("$match10/$match1", "9.9", &captures),
            "b/a"
        );
    }

    /// `checkver.useragent` must be honored when fetching.
    ///
    /// Uses a local `file://` document so no network is needed; custom
        /// headers are simply ignored by the file protocol.
    #[test]
    fn checkver_honors_useragent() {        let dir = std::env::temp_dir().join("bagger-test-checkver-ua");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("ver.json"), r#"{"tag": "9.9.9"}"#).unwrap();
        let url = format!(
            "file:///{}/ver.json",
            dir.to_string_lossy().replace('\\', "/")
        );

        let json = format!(
            r#"{{"version": "1.0", "homepage": "https://example.com",
                "license": "MIT",
                "checkver": {{"url": "{url}", "jsonpath": "$.tag",
                               "useragent": "BaggerTest/1.0"}}}}"#
        );
        let manifest = Manifest::parse_bytes(json.as_bytes(), &dir.join("ua.json")).unwrap();
        let pkg = Package::from("ua-pkg", "main", manifest);
        let session = Session::new();

        let result = super::checkver(&session, &pkg).unwrap();
        assert_eq!(result.latest_version.as_deref(), Some("9.9.9"));

        std::fs::remove_dir_all(&dir).ok();
    }

    /// `autoupdate_apply` rewrites version, URLs and download-mode hashes.
    ///
    /// Fully offline via `file://` payloads.
    #[test]
    fn autoupdate_rewrites_download_hashes() {
        use bagger_hash::ChecksumBuilder;

        let dir = std::env::temp_dir().join("bagger-test-autoupdate");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("app-2.0.bin"), b"new-bytes").unwrap();

        let payload_url = format!(
            "file:///{}/app-$version.bin",
            dir.to_string_lossy().replace('\\', "/")
        );
        let json = format!(
            r#"{{"version": "1.0", "homepage": "https://example.com",
                "license": "MIT", "url": "file:///nonexistent/app-1.0.bin",
                "hash": "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
                "autoupdate": {{"url": "{payload_url}"}}}}"#
        );
        let manifest_path = dir.join("app.json");
        std::fs::write(&manifest_path, &json).unwrap();
        let manifest = Manifest::parse(&manifest_path).unwrap();
        let pkg = Package::from("app", "main", manifest);
        let session = Session::new();

        // Dry run rewrites nothing.
        let dry = super::autoupdate_apply(&session, &pkg, "2.0", &[], false).unwrap();
        assert!(!dry.wrote);
        assert_eq!(dry.rewritten_urls.len(), 1);
        assert!(dry.rewritten_urls[0].ends_with("app-2.0.bin"));

        // Write run persists version, URL and hash.
        let done = super::autoupdate_apply(&session, &pkg, "2.0", &[], true).unwrap();
        assert!(done.wrote);

        let mut hasher = ChecksumBuilder::new().sha256().build();
        hasher.consume(b"new-bytes");
        let expected = format!("sha256:{}", hasher.finalize());

        let rewritten: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        assert_eq!(rewritten["version"], "2.0");
        assert_eq!(done.rewritten_hashes, vec![expected.clone()]);
        assert_eq!(rewritten["hash"], expected);
        assert!(rewritten["url"]
            .as_str()
            .unwrap()
            .ends_with("app-2.0.bin"));

        std::fs::remove_dir_all(&dir).ok();
    }
}

/// Get the configuation list.
///
/// # Returns
///
/// A string of the configuation list in pretty-printed JSON format.
///
/// # Errors
///
/// Serde errors will be returned if the config cannot be serialized.
pub fn config_list(session: &Session) -> Fallible<String> {
    let config = session.config();
    config.pretty()
}

/// Set a configuation key. *
///
/// # Errors
///
/// A [`ConfigInUse`][1] error will be returned if the config is borrowed
/// elsewhere.
///
/// A [`ConfigKeyInvalid`][2] error will be returned if the key is invalid.
///
/// A [`ConfigValueInvalid`][3] error will be returned if the value is invalid.
///
/// [1]: crate::Error::ConfigInUse
/// [2]: crate::Error::ConfigKeyInvalid
/// [3]: crate::Error::ConfigValueInvalid
pub fn config_set(session: &Session, key: &str, value: &str) -> Fallible<()> {
    session.config_mut()?.set(key, value)
}

/// Hold or unhold a package.
///
/// # Errors
///
/// This method will return an error if the package is not installed.
///
/// A [`PackageHoldBrokenInstall`][1] error will be returned if the install is
/// broken (`install.json` is missing or broken).
///
/// I/O errors will be returned if failed to write the `install.json` file.
/// Serde errors will be returned if the install info cannot be serialized.
///
/// [1]: crate::Error::PackageHoldBrokenInstall
pub fn package_hold(session: &Session, name: &str, flag: bool) -> Fallible<()> {
    let mut path = session.config().root_path().to_owned();
    path.push("apps");
    path.push(name);

    if !path.exists() {
        return Err(Error::PackageHoldNotInstalled(name.to_owned()));
    }

    path.push("current");
    path.push("install.json");

    if let Ok(mut install_info) = InstallInfo::parse(&path) {
        install_info.set_held(flag);
        internal::fs::write_json(path, install_info)
    } else {
        Err(Error::PackageHoldBrokenInstall(name.to_owned()))
    }
}

/// Query packages.
///
/// # Note
/// Set `installed` to `true` to query installed packages. The returned list
/// will be sorted by package name.
///
/// # Returns
///
/// A list of packages that match the query.
///
/// # Errors
///
/// I/O errors will be returned if the `apps`/`buckets` directory is not readable.
///
/// A [`Regex`][1] error will be returned if the given query is not a valid regex.
///
/// [1]: crate::Error::Regex
pub fn package_query(
    session: &Session,
    queries: Vec<&str>,
    options: Vec<QueryOption>,
    installed: bool,
) -> Fallible<Vec<Package>> {
    // remove possible duplicates
    let mut queries = HashSet::<&str>::from_iter(queries)
        .into_iter()
        .collect::<Vec<_>>();

    if queries.is_empty() {
        queries.push("*");
    }

    let mut packages = if installed {
        package::query::query_installed(session, &queries, &options)?
    } else {
        package::query::query_synced(session, &queries, &options)?
    };

    packages.sort_by_key(|p| p.name().to_owned());

    Ok(packages)
}

/// Sync packages.
///
/// # Note
/// The meaning of `sync` packages is to download, (un)install and/or upgrade
/// packages.
///
/// # Errors
///
/// I/O errors will be returned if the `apps`/`buckets` directory is not readable.
///
/// A [`PackageNotFound`][1] error will be returned if no package is found for
/// the given query.
///
/// A [`PackageMultipleCandidates`][2] error will be returned if multiple
/// candidates are found for the given query and not able to ask for a selection.
///
/// [1]: crate::Error::PackageNotFound
/// [2]: crate::Error::PackageMultipleCandidates
pub fn package_sync(
    session: &Session,
    queries: Vec<&str>,
    options: Vec<SyncOption>,
) -> Fallible<()> {
    // remove possible duplicates
    let queries = HashSet::<&str>::from_iter(queries)
        .into_iter()
        .collect::<Vec<_>>();

    if let Some(tx) = session.emitter() {
        let _ = tx.send(Event::PackageResolveStart);
    }

    let is_op_remove = options.contains(&SyncOption::Remove);
    if is_op_remove {
        package::sync::remove(session, &queries, &options)?;
    } else {
        package::sync::install(session, &queries, &options)?;
    }

    if let Some(tx) = session.emitter() {
        let _ = tx.send(Event::PackageSyncDone);
    }

    Ok(())
}

/// Result of a VirusTotal file lookup.
#[derive(Clone, Debug)]
pub struct VirustotalReport {
    /// SHA256 of the scanned file.
    pub sha256: String,
    /// Whether VirusTotal already knows this file.
    pub found: bool,
    /// Number of engines flagging the file as malicious.
    pub malicious: u64,
    /// Number of engines flagging the file as suspicious.
    pub suspicious: u64,
    /// Number of engines reporting the file as harmless.
    pub harmless: u64,
    /// Number of engines with no verdict.
    pub undetected: u64,
}

impl VirustotalReport {
    /// Whether any engine flagged the file.
    pub fn is_flagged(&self) -> bool {
        self.malicious > 0 || self.suspicious > 0
    }
}

/// Look up a local file on VirusTotal by its SHA256 hash.
///
/// Computes the SHA256 of `path`, then queries the VirusTotal v3 file API
/// (`GET /files/{sha256}`). A missing report (`404`) is not an error: the
/// returned [`VirustotalReport`] simply has `found` set to `false`.
///
/// # Errors
///
/// I/O errors will be returned if the file cannot be read. Network errors
/// will be returned if the API request fails.
pub fn virustotal_file_report(
    session: &Session,
    path: &std::path::Path,
    api_key: &str,
) -> Fallible<VirustotalReport> {
    use bagger_hash::ChecksumBuilder;
    use std::io::Read;

    let mut hasher = ChecksumBuilder::new().sha256().build();
    let mut file = std::fs::File::open(path)?;
    let mut buf = [0u8; 1024 * 64];
    loop {
        let len = file.read(&mut buf)?;
        if len == 0 {
            break;
        }
        hasher.consume(&buf[..len]);
    }
    let sha256 = hasher.finalize();

    let url = format!("https://www.virustotal.com/api/v3/files/{sha256}");
    let proxy = session.config().proxy().map(|s| s.to_owned());
    let (code, body) = internal::network::fetch_url_with_headers(
        &url,
        proxy.as_deref(),
        &[("x-apikey", api_key), ("Accept", "application/json")],
    )
    .ok_or_else(|| Error::Custom("failed to reach the VirusTotal API".to_owned()))?;

    if code == 404 {
        return Ok(VirustotalReport {
            sha256,
            found: false,
            malicious: 0,
            suspicious: 0,
            harmless: 0,
            undetected: 0,
        });
    }

    if code != 200 {
        return Err(Error::Custom(format!(
            "VirusTotal API returned HTTP {code}"
        )));
    }

    let json: serde_json::Value = serde_json::from_str(&body)?;
    let stats = &json["data"]["attributes"]["last_analysis_stats"];

    Ok(VirustotalReport {
        sha256,
        found: true,
        malicious: stats["malicious"].as_u64().unwrap_or(0),
        suspicious: stats["suspicious"].as_u64().unwrap_or(0),
        harmless: stats["harmless"].as_u64().unwrap_or(0),
        undetected: stats["undetected"].as_u64().unwrap_or(0),
    })
}
