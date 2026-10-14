use std::path::PathBuf;

use crate::error::Fallible;

#[cfg(unix)]
pub use unix::{broadcast_env_change, get_scoped, get_scoped_opt, set_scoped};
#[cfg(windows)]
pub use windows::{broadcast_env_change, get_scoped, get_scoped_opt, set_scoped};

/// Get the value of a path-like environment variable.
pub fn get_path_like_env(name: &str) -> Fallible<Vec<PathBuf>> {
    get_path_like_env_scoped(name, false)
}

/// Get the value of a path-like environment variable from the user or
/// system (global) environment. Missing variables read as empty.
pub fn get_path_like_env_scoped(name: &str, global: bool) -> Fallible<Vec<PathBuf>> {
    let raw = get_scoped_opt(name, global)?;
    let text = raw
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    if text.is_empty() {
        return Ok(Vec::new());
    }
    Ok(std::env::split_paths(&text).collect())
}

/// Join path entries back into a `;`-separated list.
pub fn join_path_list(paths: &[PathBuf]) -> Fallible<std::ffi::OsString> {
    std::env::join_paths(paths).map_err(|e| crate::Error::Custom(e.to_string()))
}

#[cfg(windows)]
mod windows {
    use std::ffi::{OsStr, OsString};
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::path::Path;

    use once_cell::sync::Lazy;
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, REG_EXPAND_SZ};
    use winreg::RegKey;

    use crate::error::Fallible;

    /// `HKEY_CURRENT_USER` registry key handle.
    static HKCU: Lazy<RegKey> = Lazy::new(|| RegKey::predef(HKEY_CURRENT_USER));
    /// `HKEY_LOCAL_MACHINE` registry key handle.
    static HKLM: Lazy<RegKey> = Lazy::new(|| RegKey::predef(HKEY_LOCAL_MACHINE));

    const USER_ENV_SUBKEY: &str = "Environment";
    const MACHINE_ENV_SUBKEY: &str =
        r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment";

    fn open_env_key(global: bool, writable: bool) -> Fallible<RegKey> {
        if global {
            let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
            if writable {
                // Opens the existing key for writing (creating is harmless);
                // fails cleanly without admin rights.
                let (key, _) = hklm.create_subkey(MACHINE_ENV_SUBKEY)?;
                Ok(key)
            } else {
                Ok(hklm.open_subkey(MACHINE_ENV_SUBKEY)?)
            }
        } else if writable {
            let (key, _) = HKCU.create_subkey(Path::new(USER_ENV_SUBKEY))?;
            Ok(key)
        } else {
            Ok(HKCU.open_subkey(Path::new(USER_ENV_SUBKEY))?)
        }
    }

    /// Decode a `REG_SZ`/`REG_EXPAND_SZ` value without expanding embedded
    /// `%VAR%` references (mirrors upstream `Get-EnvVar`, which reads with
    /// `DoNotExpandEnvironmentNames` so round-trips preserve the raw text).
    fn decode_reg_string(bytes: &[u8]) -> OsString {
        let (chunks, _) = bytes.as_chunks::<2>();
        let words: Vec<u16> = chunks.iter().map(|c| u16::from_le_bytes(*c)).collect();
        let end = words.iter().position(|&w| w == 0).unwrap_or(words.len());
        OsString::from_wide(&words[..end])
    }

    fn encode_reg_string(value: &OsStr) -> Vec<u8> {
        let mut words: Vec<u16> = value.encode_wide().collect();
        words.push(0);
        let mut bytes = Vec::with_capacity(words.len() * 2);
        for w in words {
            bytes.extend_from_slice(&w.to_le_bytes());
        }
        bytes
    }

    /// Get the raw value of an environment variable from the user (or, when
    /// `global`, system) environment. Errors when the value is missing.
    pub fn get_scoped(key: &str, global: bool) -> Fallible<OsString> {
        get_scoped_opt(key, global)?
            .ok_or_else(|| crate::Error::Custom(format!("environment variable '{key}' is not set")))
    }

    /// Get the raw value of an environment variable, returning `None` when
    /// it is missing instead of erroring.
    pub fn get_scoped_opt(key: &str, global: bool) -> Fallible<Option<OsString>> {
        let env = match open_env_key(global, false) {
            Ok(env) => env,
            // A missing key (fresh profile) simply means "no value".
            Err(crate::Error::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok(None)
            }
            Err(e) => return Err(e),
        };
        match env.get_raw_value(key) {
            Ok(raw) => Ok(Some(decode_reg_string(&raw.bytes))),
            Err(_) => Ok(None),
        }
    }

    /// Set the value of an environment variable in the user (or, when
    /// `global`, system) environment. A `None` value deletes it.
    ///
    /// Values containing `%` are stored as `REG_EXPAND_SZ` so references
    /// like `%SystemRoot%` keep expanding, mirroring upstream `Set-EnvVar`.
    pub fn set_scoped(key: &str, value: Option<&OsString>, global: bool) -> Fallible<()> {
        let env = open_env_key(global, true)?;
        match value {
            Some(value) if !value.is_empty() => {
                if value.to_string_lossy().contains('%') {
                    env.set_raw_value(
                        key,
                        &winreg::RegValue {
                            bytes: encode_reg_string(value),
                            vtype: REG_EXPAND_SZ,
                        },
                    )?;
                } else {
                    env.set_value(key, value)?;
                }
            }
            _ => {
                // ignore error of deleting non-existent value
                let _ = env.delete_value(key);
            }
        }
        Ok(())
    }

    /// Broadcast `WM_SETTINGCHANGE` so running programs (notably Explorer,
    /// which spawns new terminals) pick up registry environment changes.
    ///
    /// Upstream does this on every `Set-EnvVar`; .NET's
    /// `SetEnvironmentVariable` broadcasts automatically, so a set+delete
    /// round-trip of a throwaway variable is the dependency-free equivalent.
    /// Best-effort: failures are logged, never fatal.
    pub fn broadcast_env_change() {
        let out = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "[Environment]::SetEnvironmentVariable('BAGGER_ENV_SYNC', '1', 'User'); [Environment]::SetEnvironmentVariable('BAGGER_ENV_SYNC', $null, 'User')",
            ])
            .output();
        if let Err(e) = out {
            tracing::debug!("environment change broadcast failed: {e}");
        }
    }
}

#[cfg(unix)]
mod unix {
    use std::ffi::OsString;

    use crate::error::Fallible;

    /// Get the value of an environment variable (process scope on unix).
    pub fn get_scoped(key: &str, _global: bool) -> Fallible<OsString> {
        Ok(std::env::var_os(key).unwrap_or_default())
    }

    /// Get the value of an environment variable, `None` when missing.
    pub fn get_scoped_opt(key: &str, _global: bool) -> Fallible<Option<OsString>> {
        Ok(std::env::var_os(key))
    }

    /// Set the value of an environment variable (process scope on unix).
    pub fn set_scoped(_key: &str, _value: Option<&OsString>, _global: bool) -> Fallible<()> {
        // no-op
        Ok(())
    }

    /// Environment broadcasts are a Windows registry concept; no-op on unix.
    pub fn broadcast_env_change() {}
}
