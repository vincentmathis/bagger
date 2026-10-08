use std::collections::HashMap;
use std::io::Read;
use std::ops::Deref;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::error::{Error, Fallible};
use crate::internal;

/// Builder pattern for generating [`Config`].
pub struct ConfigBuilder {
    /// Path of the config file.
    ///
    /// default is [`default::config_path()`].
    path: PathBuf,
}

impl ConfigBuilder {
    pub fn new() -> ConfigBuilder {
        Self {
            path: default::config_path(),
        }
    }

    pub fn path<P: AsRef<Path>>(&mut self, path: P) -> ConfigBuilder {
        Self {
            path: path.as_ref().to_owned(),
        }
    }

    /// Load the config file from the config path.
    pub fn load(&self) -> Fallible<Config> {
        let mut buf = vec![];
        let path = self.path.clone();

        std::fs::File::open(&path)?.read_to_end(&mut buf)?;

        let inner = serde_json::from_slice(&buf)?;
        let config = Config {
            path,
            inner,
            root_override: None,
        };
        Ok(config)
    }
}

/// Scoop Configuration representation.
///
/// **NOTE**: Not all fields are supported. For the purpose of not erasing unused
/// fields during serialization, they are implemented to be (de)serializable.
/// However, most of them are set to private and transparent during the whole
/// (de)serialization process.
#[derive(Clone, Debug)]
pub struct Config {
    /// The file path of this [`Config`].
    pub path: PathBuf,

    /// Inner config data.
    inner: ConfigInner,

    /// Runtime root directory override (e.g. for `--global` installs).
    ///
    /// This is never serialized; it only affects [`Config::root_path`].
    root_override: Option<PathBuf>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ConfigInner {
    #[serde(skip_serializing_if = "Option::is_none")]
    alias: Option<HashMap<String, String>>,

    #[serde(alias = "aria2_enabled")]
    #[serde(rename = "aria2-enabled")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_enabled: Option<bool>,

    #[serde(alias = "aria2_max_connection_per_server")]
    #[serde(rename = "aria2-max-connection-per-server")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_max_connection_per_server: Option<u32>,

    #[serde(alias = "aria2_min_split_size")]
    #[serde(rename = "aria2-min-split-size")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_min_split_size: Option<String>,

    #[serde(alias = "aria2_options")]
    #[serde(rename = "aria2-options")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_options: Option<String>,

    #[serde(alias = "aria2_retry_wait")]
    #[serde(rename = "aria2-retry-wait")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_retry_wait: Option<u32>,

    #[serde(alias = "aria2_split")]
    #[serde(rename = "aria2-split")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_split: Option<u32>,

    #[serde(alias = "aria2_warning_enabled")]
    #[serde(rename = "aria2-warning-enabled")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_warning_enabled: Option<bool>,

    #[serde(alias = "aria2_fallback_enabled")]
    #[serde(rename = "aria2-fallback-enabled")]
    #[serde(skip_serializing_if = "Option::is_none")]
    aria2_fallback_enabled: Option<bool>,

    #[serde(alias = "cachePath")]
    #[serde(default = "default::cache_path")]
    #[serde(skip_serializing_if = "default::is_default_cache_path")]
    cache_path: PathBuf,

    #[serde(skip_serializing_if = "Option::is_none")]
    cat_style: Option<String>,

    #[serde(alias = "deafult_architecture")]
    #[serde(skip_serializing_if = "Option::is_none")]
    default_architecture: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    debug: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    force_update: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    gh_token: Option<String>,

    #[serde(alias = "globalPath")]
    #[serde(default = "default::global_path")]
    #[serde(skip_serializing_if = "default::is_default_global_path")]
    global_path: PathBuf,

    #[serde(skip_serializing_if = "Option::is_none")]
    ignore_running_processes: Option<bool>,

    #[serde(alias = "lastupdate")]
    #[serde(skip_serializing_if = "Option::is_none")]
    last_update: Option<String>,

    #[serde(alias = "manifest_review")]
    #[serde(skip_serializing_if = "Option::is_none")]
    show_manifest: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    use_isolated_path: Option<IsolatedPath>,

    #[serde(alias = "msiextract_use_lessmsi")]
    #[serde(skip_serializing_if = "Option::is_none")]
    use_lessmsi: Option<bool>,

    /// Use SQLite to cache manifests.
    ///
    /// This config was introduced in Scoop v0.5.0 (Jul, 2024)
    #[serde(skip_serializing_if = "Option::is_none")]
    use_sqlite_cache: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    use_git_history: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    update_nightly: Option<bool>,

    /// Disable `current` version junction creation.
    ///
    /// The 'current' version alias will not be used. Shims and shortcuts will
    /// point to specific version instead.
    ///
    /// This config was introduced in Jan, 2017 with the name `NO_JUNCTIONS`:
    /// https://github.com/ScoopInstaller/Scoop/commit/a14ffdb5
    ///
    /// It was renamed to `no_junction` in Aug, 2022 (later in release v0.3.0):
    /// https://github.com/ScoopInstaller/Scoop/pull/5116
    #[serde(alias = "no_junctions")]
    #[serde(skip_serializing_if = "Option::is_none")]
    no_junction: Option<bool>,

    /// A list of private hosts.
    ///
    /// # Note
    ///
    /// Array of private hosts that need additional authentication. For example,
    /// if you want to access a private GitHub repository, you need to add the
    /// host to this list with 'match' and 'headers' strings.
    ///
    /// This config was introduced in Feb, 2021:
    /// https://github.com/ScoopInstaller/Scoop/pull/4254
    #[serde(skip_serializing_if = "Option::is_none")]
    private_hosts: Option<Vec<PrivateHosts>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    proxy: Option<String>,

    #[serde(alias = "rootPath")]
    #[serde(default = "default::root_path")]
    #[serde(skip_serializing_if = "default::is_default_root_path")]
    root_path: PathBuf,

    #[serde(skip_serializing_if = "Option::is_none")]
    scoop_branch: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    scoop_repo: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    shim: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    show_update_log: Option<bool>,

    #[serde(alias = "7zipextract_use_external")]
    #[serde(skip_serializing_if = "Option::is_none")]
    use_external_7zip: Option<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PrivateHosts {
    /// A string defining the host to match.
    #[serde(rename = "match")]
    match_: String,

    /// A string defining HTTP headers.
    headers: String,
}

impl PrivateHosts {
    /// The URL pattern (upstream `$url -match`, i.e. regex) selecting
    /// requests that carry these headers.
    #[inline]
    pub fn matcher(&self) -> &str {
        &self.match_
    }

    /// Extra headers in PowerShell `StringData` form (`Name=Value` lines).
    #[inline]
    pub fn headers(&self) -> &str {
        &self.headers
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum IsolatedPath {
    /// boolean type of `use_isolated_path`
    Boolean(bool),

    /// string type of `use_isolated_path` indicating the environment variable name
    Named(String),
}

impl FromStr for IsolatedPath {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.to_ascii_lowercase();

        // `=` is not a valid character in environment variable names
        // ref: https://learn.microsoft.com/en-us/windows/win32/procthread/environment-variables
        if s.contains('=') {
            return Err(Error::ConfigValueInvalid(s));
        }

        match s.as_str() {
            "true" => Ok(IsolatedPath::Boolean(true)),
            "false" => Ok(IsolatedPath::Boolean(false)),
            _ => Ok(IsolatedPath::Named(s)),
        }
    }
}

impl Config {
    /// Initialize the config with default values.
    ///
    /// This function will try to write the default config to the default path,
    /// located in the XDG_CONFIG_HOME directory.
    pub(crate) fn init() -> Config {
        let config = Config::default();
        // try to write the default config to the default path, error is ignored
        let _ = internal::fs::write_json(default::config_path(), &config.inner);
        config
    }

    /// Get the `cache` directory of Scoop.
    #[inline]
    pub fn cache_path(&self) -> &Path {
        self.cache_path.as_path()
    }

    /// Get the root directory of Scoop.
    ///
    /// This is the root directory of a Scoop installation, by default the value
    /// is `$HOME/scoop`. It may be changed by setting the `SCOOP` environment
    /// variable.
    ///
    /// When a runtime root override is set (e.g. via `--global` installs),
    /// the override is returned instead.
    #[inline]
    pub fn root_path(&self) -> &Path {
        self.root_override
            .as_deref()
            .unwrap_or_else(|| self.root_path.as_path())
    }

    /// Set or clear the runtime root directory override.
    ///
    /// This is crate-internal state used to scope an operation (such as a
    /// `--global` install) at the global root. It is never persisted to the
    /// config file.
    pub(crate) fn set_root_override<P: AsRef<Path>>(&mut self, path: Option<P>) {
        self.root_override = path.map(|p| p.as_ref().to_owned());
    }

    /// Get the global Scoop installation directory.
    #[inline]
    pub fn global_path(&self) -> &Path {
        self.inner.global_path.as_path()
    }

    /// Get the `no_junction` config.
    #[inline]
    pub fn no_junction(&self) -> bool {
        self.no_junction.unwrap_or_default()
    }

    /// Whether operations are currently scoped to the global root.
    ///
    /// True when a runtime root override points at the global path
    /// (e.g. after `Session::set_global(true)` for `--global` installs).
    /// Hook scripts observe this as `$global`, mirroring upstream Scoop.
    #[inline]
    pub fn is_global_scope(&self) -> bool {
        self.root_path() == self.global_path()
    }

    /// Get the `use_external_7zip` config.
    ///
    /// When enabled, `Expand-7zipArchive` resolves `7z` from `PATH`
    /// instead of the Scoop `7zip` app.
    #[inline]
    pub fn use_external_7zip(&self) -> bool {
        self.use_external_7zip.unwrap_or_default()
    }

    /// Get the `use_lessmsi` config.
    ///
    /// When enabled, `Expand-MsiArchive` extracts with the Scoop `lessmsi`
    /// app; otherwise the system `msiexec.exe` is used.
    #[inline]
    pub fn use_lessmsi(&self) -> bool {
        self.use_lessmsi.unwrap_or_default()
    }

    /// Get the `proxy` config.
    #[inline]
    pub fn proxy(&self) -> Option<&str> {
        self.proxy.as_deref()
    }

    /// Get the `cat_style` config.
    #[inline]
    pub fn cat_style(&self) -> &str {
        self.cat_style.as_deref().unwrap_or_default()
    }

    /// Get the `use_isoloated_path` config.
    #[inline]
    pub fn use_isolated_path(&self) -> Option<&IsolatedPath> {
        self.use_isolated_path.as_ref()
    }

    /// Get the `show_manifest` (manifest_review) config.
    ///
    /// When enabled, the manifest content of each package will be displayed
    /// in the transaction confirmation prompt before installation/upgrade.
    #[inline]
    pub fn show_manifest(&self) -> bool {
        self.show_manifest.unwrap_or_default()
    }

    /// Get the `aria2_enabled` config (upstream default is on; the curl
    /// backend remains as the missing-binary and failure fallback).
    #[inline]
    pub fn aria2_enabled(&self) -> bool {
        self.aria2_enabled.unwrap_or(true)
    }

    /// Get the `aria2_warning_enabled` config.
    ///
    /// When disabled, aria2 availability warnings will be suppressed.
    #[inline]
    pub fn aria2_warning_enabled(&self) -> bool {
        self.aria2_warning_enabled.unwrap_or_default()
    }

    /// Get the `aria2_split` config.
    #[inline]
    pub fn aria2_split(&self) -> u32 {
        self.aria2_split.unwrap_or(5)
    }

    /// Get the `aria2_max_connection_per_server` config.
    #[inline]
    pub fn aria2_max_connection_per_server(&self) -> u32 {
        self.aria2_max_connection_per_server.unwrap_or(8)
    }

    /// Get the `aria2_min_split_size` config.
    #[inline]
    pub fn aria2_min_split_size(&self) -> &str {
        self.aria2_min_split_size.as_deref().unwrap_or("10M")
    }

    /// Get the `aria2_retry_wait` config.
    #[inline]
    pub fn aria2_retry_wait(&self) -> u32 {
        self.aria2_retry_wait.unwrap_or(5)
    }

    /// Get the `aria2_options` config as a string.
    #[inline]
    pub fn aria2_options(&self) -> Option<&str> {
        self.aria2_options.as_deref()
    }

    /// Get the `ignore_running_processes` config.
    #[inline]
    pub fn ignore_running_processes(&self) -> bool {
        self.ignore_running_processes.unwrap_or_default()
    }

    /// Get the `use_sqlite_cache` config.
    ///
    /// When enabled, bucket manifests are cached in a SQLite database to
    /// speed up queries.
    #[inline]
    pub fn use_sqlite_cache(&self) -> bool {
        self.use_sqlite_cache.unwrap_or_default()
    }

    /// Get the `update_nightly` config (nightly apps update when a new day
    /// dawned; otherwise only `--force` reinstalls them).
    #[inline]
    pub fn update_nightly(&self) -> bool {
        self.update_nightly.unwrap_or_default()
    }

    /// Get the `use_git_history` config (search bucket git history for
    /// pinned `@version` manifests; upstream default is on).
    #[inline]
    pub fn use_git_history(&self) -> bool {
        self.use_git_history.unwrap_or(true)
    }

    /// Get the `gh_token` config (GitHub API token for authenticated
    /// requests, easing rate limits and private-repo access).
    #[inline]
    pub fn gh_token(&self) -> Option<&str> {
        self.gh_token.as_deref()
    }

    /// Get the `private_hosts` config (per-host match/headers for
    /// additional download authentication).
    #[inline]
    pub fn private_hosts(&self) -> Option<&[PrivateHosts]> {
        self.private_hosts.as_deref()
    }

    /// Get the `force_update` config (upgrade behaves as `--force`).
    #[inline]
    pub fn force_update(&self) -> bool {
        self.force_update.unwrap_or_default()
    }

    /// Get the `show_update_log` config (display bucket commit logs on
    /// update; upstream default is shown).
    #[inline]
    pub fn show_update_log(&self) -> bool {
        self.show_update_log.unwrap_or(true)
    }

    /// Get the `last_update` config (timestamp of the last bucket update).
    #[inline]
    pub fn last_update(&self) -> Option<&str> {
        self.last_update.as_deref()
    }

    /// Get the `debug` config (additional detailed output).
    #[inline]
    pub fn debug(&self) -> bool {
        self.debug.unwrap_or_default()
    }

    /// Get the `default_architecture` config (preferred install
    /// architecture when no `--arch`/`SCOOP_ARCH` override is given).
    #[inline]
    pub fn default_architecture(&self) -> Option<&str> {
        self.default_architecture.as_deref()
    }

    /// Get the `aria2-fallback-enabled` config (fall back to the default
    /// downloader when an aria2c download fails; upstream default is on).
    #[inline]
    pub fn aria2_fallback_enabled(&self) -> bool {
        self.aria2_fallback_enabled.unwrap_or(true)
    }

    /// Update config key with new value.
    pub(crate) fn set(&mut self, key: &str, value: &str) -> Fallible<()> {
        let is_unset = value.is_empty();
        match key {
            "use_external_7zip" | "7zipextract_use_external" => match is_unset {
                true => self.inner.use_external_7zip = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.use_external_7zip = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "manifest_review" | "show_manifest" => match is_unset {
                true => self.inner.show_manifest = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.show_manifest = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "ignore_running_processes" => match is_unset {
                true => self.inner.ignore_running_processes = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.ignore_running_processes = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "aria2_enabled" | "aria2-enabled" => match is_unset {
                true => self.inner.aria2_enabled = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.aria2_enabled = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "aria2_warning_enabled" | "aria2-warning-enabled" => match is_unset {
                true => self.inner.aria2_warning_enabled = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.aria2_warning_enabled = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "cat_style" => {
                self.inner.cat_style = match is_unset {
                    true => None,
                    false => Some(value.to_string()),
                }
            }
            "gh_token" => {
                self.inner.gh_token = match is_unset {
                    true => None,
                    false => Some(value.to_string()),
                }
            }
            "alias" => match is_unset {
                true => self.inner.alias = None,
                false => match serde_json::from_str::<HashMap<String, String>>(value) {
                    Ok(value) => self.inner.alias = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "last_update" => {
                self.inner.last_update = match is_unset {
                    true => None,
                    false => Some(value.to_string()),
                }
            }
            "use_isolated_path" => match is_unset {
                true => self.inner.use_isolated_path = None,
                false => match value.parse::<IsolatedPath>() {
                    Ok(value) => self.inner.use_isolated_path = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "use_lessmsi" => match is_unset {
                true => self.inner.use_lessmsi = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.use_lessmsi = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "use_sqlite_cache" => match is_unset {
                true => self.inner.use_sqlite_cache = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.use_sqlite_cache = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "update_nightly" => match is_unset {
                true => self.inner.update_nightly = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.update_nightly = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "use_git_history" => match is_unset {
                true => self.inner.use_git_history = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.use_git_history = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "proxy" => match value {
                "" | "none" => self.inner.proxy = None,
                _ => self.inner.proxy = Some(value.to_string()),
            },
            "debug" => match is_unset {
                true => self.inner.debug = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.debug = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "force_update" => match is_unset {
                true => self.inner.force_update = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.force_update = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "show_update_log" => match is_unset {
                true => self.inner.show_update_log = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.show_update_log = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "private_hosts" => match is_unset {
                true => self.inner.private_hosts = None,
                false => match serde_json::from_str::<Vec<PrivateHosts>>(value) {
                    Ok(value) => self.inner.private_hosts = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            "default_architecture" => {
                self.inner.default_architecture = match is_unset {
                    true => None,
                    false => Some(value.to_string()),
                }
            }
            "aria2_fallback_enabled" | "aria2-fallback-enabled" => match is_unset {
                true => self.inner.aria2_fallback_enabled = None,
                false => match value.parse::<bool>() {
                    Ok(value) => self.inner.aria2_fallback_enabled = Some(value),
                    Err(_) => return Err(Error::ConfigValueInvalid(value.to_owned())),
                },
            },
            key => return Err(Error::ConfigKeyInvalid(key.to_owned())),
        }

        self.commit()
    }

    /// Commit config changes and save to the config file
    pub(crate) fn commit(&self) -> Fallible<()> {
        internal::fs::write_json(&self.path, &self.inner)
    }

    /// Pretty print the config
    pub(crate) fn pretty(&self) -> Fallible<String> {
        Ok(serde_json::to_string_pretty(&self.inner)?)
    }
}

impl Default for Config {
    fn default() -> Self {
        let inner = ConfigInner {
            alias: Default::default(),
            aria2_enabled: Default::default(),
            aria2_max_connection_per_server: Default::default(),
            aria2_min_split_size: Default::default(),
            aria2_options: Default::default(),
            aria2_retry_wait: Default::default(),
            aria2_split: Default::default(),
            aria2_warning_enabled: Default::default(),
            aria2_fallback_enabled: Default::default(),
            // default_cache_path: default::cache_path(),
            cache_path: default::cache_path(),
            cat_style: Default::default(),
            default_architecture: Default::default(),
            debug: Default::default(),
            force_update: Default::default(),
            gh_token: Default::default(),
            // default_global_path: default::global_path(),
            global_path: default::global_path(),
            ignore_running_processes: Default::default(),
            last_update: Default::default(),
            show_manifest: Default::default(),
            use_isolated_path: Default::default(),
            use_lessmsi: Default::default(),
            use_sqlite_cache: Default::default(),
            use_git_history: Default::default(),
            update_nightly: Default::default(),
            no_junction: Default::default(),
            private_hosts: Default::default(),
            proxy: Default::default(),
            // default_root_path: default::root_path(),
            root_path: default::root_path(),
            scoop_branch: Default::default(),
            scoop_repo: Default::default(),
            shim: Default::default(),
            show_update_log: Default::default(),
            use_external_7zip: Default::default(),
        };
        Config {
            path: default::config_path(),
            inner,
            root_override: None,
        }
    }
}

impl Deref for Config {
    type Target = ConfigInner;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Get a list of possible config paths.
///
/// There are 3 possible locations for the `config.json` file:
///   1) Side-by-side with the real executable (symlink resolved);
///   2) Located in the `root` directory of Scoop;
///   3) Located in the XDG_CONFIG_HOME directory.
pub(crate) fn possible_config_paths() -> Vec<PathBuf> {
    let mut ret = vec![];

    if let Ok(exe_path) = std::env::current_exe() {
        if let Ok(metadata) = std::fs::symlink_metadata(&exe_path) {
            let is_symlink = metadata.is_symlink();
            let mut path = exe_path.clone();

            if is_symlink {
                // since the executable is a symlink, we can use `read_link`
                // to get the real path of the executable
                if let Ok(real_path) = std::fs::read_link(&exe_path) {
                    path = real_path;
                }
            }

            path.pop();
            path.push("config.json");

            // 1) config.json side-by-side with the real executable
            ret.push(path.clone());

            // this pop is ok, it removes `config.json` we just pushed
            // <app>\<current>\<app_name>\apps\<root> (in theory)
            //       ^^^^^^^^^
            path.pop();
            // <app>\<current>\<app_name>\apps\<root> (in theory)
            //                 ^^^^^^^^^^
            if path.pop() {
                // <app>\<current>\<app_name>\apps\<root> (in theory)
                //                            ^^^^
                if path.pop() {
                    let check = internal::path::leaf(&path)
                        .map(|n| n == "apps")
                        .unwrap_or_default();
                    // <app>\<current>\<app_name>\apps\<root> (in theory)
                    //                                 ^^^^^
                    if check && path.pop() {
                        path.push("config.json");

                        // 2) config.json located in the `root` directory of
                        // Scoop, i.e., the portable config.json
                        ret.push(path);
                    }
                }
            }
        }
    }

    // 3) config.json located in the XDG_CONFIG_HOME directory, i.e.,
    // `~/.config/scoop/config.json`
    ret.push(default::config_path());

    ret
}

/// This private module contains functions of constructing default paths used
/// to create the default Scoop `Config`, with system's environment variables.
mod default {
    use std::path::{Path, PathBuf};

    use crate::internal::path::normalize_path;

    /// Join the given `path` to `$HOME` and return a new [`PathBuf`].
    #[inline]
    fn home_join<P: AsRef<Path>>(path: P) -> PathBuf {
        dirs::home_dir().map(|p| p.join(path.as_ref())).unwrap()
    }

    /// Get the default Scoop config path: `$HOME/.config/scoop/config.json`.
    #[inline]
    pub(super) fn config_path() -> PathBuf {
        normalize_path(home_join(".config/scoop/config.json"))
    }

    /// Get the default Scoop root path.
    #[inline]
    pub(super) fn root_path() -> PathBuf {
        let path = if let Some(path) = std::env::var_os("SCOOP") {
            PathBuf::from(path)
        } else {
            home_join("scoop")
        };

        normalize_path(path)
    }

    /// Get the default Scoop cache path.
    #[inline]
    pub(super) fn cache_path() -> PathBuf {
        let path = if let Some(path) = std::env::var_os("SCOOP_CACHE") {
            PathBuf::from(path)
        } else {
            root_path().join("cache")
        };

        normalize_path(path)
    }

    /// Get the default Scoop global path.
    #[inline]
    pub(super) fn global_path() -> PathBuf {
        let path = if let Some(path) = std::env::var_os("SCOOP_GLOBAL") {
            return PathBuf::from(path);
        } else {
            std::env::var_os("ProgramData")
                .map(PathBuf::from)
                .map(|p| p.join("scoop"))
                .unwrap_or(PathBuf::from("C:/ProgramData/scoop"))
        };

        normalize_path(path)
    }

    /// Check if the given `path` is equal to the `default` one.
    #[inline]
    fn is_default(default: &Path, path: &Path) -> bool {
        path.eq(default)
    }

    /// Check if the given `path` is equal to the `default` Scoop root path.
    #[inline]
    pub(super) fn is_default_root_path<P: AsRef<Path>>(path: P) -> bool {
        is_default(root_path().as_path(), path.as_ref())
    }

    /// Check if the given `path` is equal to the `default` Scoop cache path.
    #[inline]
    pub(super) fn is_default_cache_path<P: AsRef<Path>>(path: P) -> bool {
        is_default(cache_path().as_path(), path.as_ref())
    }

    /// Check if the given `path` is equal to the `default` Scoop global path.
    #[inline]
    pub(super) fn is_default_global_path<P: AsRef<Path>>(path: P) -> bool {
        is_default(global_path().as_path(), path.as_ref())
    }
}
