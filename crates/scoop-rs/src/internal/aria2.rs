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

/// Human-readable meaning of an aria2c exit code (upstream `aria_exit_code`).
pub fn exit_code_message(code: i32) -> &'static str {
    match code {
        0 => "All downloads were successful",
        1 => "An unknown error occurred",
        2 => "Timeout",
        3 => "Resource was not found",
        4 => "Aria2 saw the specified number of \"resource not found\" error",
        5 => "Download aborted because download speed was too slow",
        6 => "Network problem occurred",
        7 => "There were unfinished downloads",
        8 => "Remote server did not support resume when resume was required",
        9 => "There was not enough disk space available",
        10 => "Piece length was different from one in .aria2 control file",
        11 => "Aria2 was downloading same file at that moment",
        12 => "Aria2 was downloading same info hash torrent at that moment",
        13 => "File already existed",
        14 => "Renaming file failed",
        15 => "Aria2 could not open existing file",
        16 => "Aria2 could not create new file or truncate existing file",
        17 => "File I/O error occurred",
        18 => "Aria2 could not create directory",
        19 => "Name resolution failed",
        20 => "Aria2 could not parse Metalink document",
        21 => "FTP command failed",
        22 => "HTTP response header was bad or unexpected",
        23 => "Too many redirects occurred",
        24 => "HTTP authorization failed",
        25 => "Aria2 could not parse bencoded file",
        26 => "\".torrent\" file was corrupted or missing information",
        27 => "Magnet URI was bad",
        28 => "Bad/unrecognized option was given",
        29 => "The remote server was unable to handle the request",
        30 => "Aria2 could not parse JSON-RPC request",
        32 => "Checksum validation failed",
        _ => "An unknown error occurred",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_codes_have_meanings() {
        assert_eq!(exit_code_message(0), "All downloads were successful");
        assert_eq!(exit_code_message(3), "Resource was not found");
        assert_eq!(exit_code_message(19), "Name resolution failed");
        assert_eq!(exit_code_message(32), "Checksum validation failed");
        assert_eq!(exit_code_message(99), "An unknown error occurred");
    }
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
        return Err(Error::Aria2 {
            url: url.to_owned(),
            code: output.status.code().unwrap_or(-1),
            stderr: stderr.trim().to_owned(),
        });
    }

    if !dest.exists() {
        return Err(Error::Custom(format!(
            "aria2c reported success but '{}' was not created",
            dest.display()
        )));
    }

    Ok(())
}
