//! Aria2 download manager integration.
//!
//! When the `aria2-enabled` config is set and an `aria2c` binary is available
//! on `PATH`, package downloads are delegated to aria2c for faster,
//! multi-connection downloads. Otherwise downloads fall back to the built-in
//! curl backend.

use std::path::Path;
use std::process::Command;

use crate::error::{Error, Fallible};

/// Options controlling an aria2c download.
#[derive(Clone, Debug)]
pub struct DownloadOptions {
    /// User-Agent header value.
    pub user_agent: String,
    /// Preformatted `Cookie` header value (empty means no cookie).
    pub cookie: String,
    /// Proxy URL passed as `--all-proxy`.
    pub proxy: Option<String>,
    /// Value for `--split`.
    pub split: u32,
    /// Value for `--max-connection-per-server`.
    pub max_connection_per_server: u32,
    /// Value for `--min-split-size` (e.g. `"10M"`).
    pub min_split_size: String,
    /// Value for `--retry-wait` (seconds).
    pub retry_wait: u32,
    /// Extra raw aria2c arguments (whitespace separated).
    pub extra_options: Option<String>,
}

/// Check if an `aria2c` binary is available on the system.
///
/// Both bare `aria2c` and `aria2c.exe` are probed, since the naive `PATH`
/// lookup does not account for Windows' `PATHEXT` resolution.
pub fn is_available() -> bool {
    super::os::is_program_available("aria2c") || super::os::is_program_available("aria2c.exe")
}

/// Download a single file with aria2c.
///
/// The file is written to `dest` (a temporary `.download` path in the cache
/// dir, matching the curl backend's convention). On a non-zero exit status an
/// [`Error::Custom`] carrying aria2c's stderr is returned.
pub fn download_file(url: &str, dest: &Path, opts: &DownloadOptions) -> Fallible<()> {
    let parent = dest.parent().ok_or_else(|| {
        Error::Custom(format!("invalid download destination '{}'", dest.display()))
    })?;
    let filename = dest.file_name().and_then(|n| n.to_str()).ok_or_else(|| {
        Error::Custom(format!("invalid download destination '{}'", dest.display()))
    })?;

    let mut cmd = Command::new("aria2c");
    cmd.arg(format!("--dir={}", parent.display()))
        .arg(format!("--out={filename}"))
        .arg("--allow-overwrite=true")
        .arg("--auto-file-renaming=false")
        .arg(format!("--split={}", opts.split))
        .arg(format!(
            "--max-connection-per-server={}",
            opts.max_connection_per_server
        ))
        .arg(format!("--min-split-size={}", opts.min_split_size))
        .arg(format!("--retry-wait={}", opts.retry_wait))
        .arg(format!("--user-agent={}", opts.user_agent))
        .arg("--quiet=true")
        .arg("--show-console-readout=false")
        .arg("--summary-interval=0");

    if !opts.cookie.is_empty() {
        cmd.arg(format!("--header=Cookie: {}", opts.cookie));
    }

    if let Some(proxy) = opts.proxy.as_deref() {
        cmd.arg(format!("--all-proxy={proxy}"));
    }

    if let Some(extra) = opts.extra_options.as_deref() {
        for opt in extra.split_whitespace() {
            cmd.arg(opt);
        }
    }

    cmd.arg(url);

    let output = cmd
        .output()
        .map_err(|e| Error::Custom(format!("failed to launch aria2c for '{url}' ({e})")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Custom(format!(
            "aria2c failed to download '{url}' ({})",
            stderr.trim()
        )));
    }

    if !dest.exists() {
        return Err(Error::Custom(format!(
            "aria2c reported success but '{}' was not created",
            dest.display()
        )));
    }

    Ok(())
}
