//! System diagnostics for `checkup`, mirroring upstream `lib/diagnostic.ps1`.
//!
//! Each check returns `None` when healthy (or when the state cannot be
//! determined — a diagnostic must never false-alarm) and `Some(SystemCheck)`
//! with remediation steps otherwise.

use crate::Session;

/// A failed system check with remediation steps.
#[derive(Clone, Debug)]
pub struct SystemCheck {
    /// Short title of the problem.
    pub title: String,
    /// Remediation steps, printed one per line.
    pub fix: Vec<String>,
}

impl SystemCheck {
    fn new(title: &str, fix: Vec<&str>) -> SystemCheck {
        SystemCheck {
            title: title.to_owned(),
            fix: fix.into_iter().map(str::to_owned).collect(),
        }
    }
}

/// Run all system diagnostics, collecting the failing checks.
pub fn system_diagnostics(session: &Session) -> Vec<SystemCheck> {
    [
        check_main_bucket(session),
        check_long_paths(),
        check_developer_mode(),
        check_windows_defender(session),
    ]
    .into_iter()
    .flatten()
    .collect()
}

/// The `main` bucket should be present (upstream `check_main_bucket`).
fn check_main_bucket(session: &Session) -> Option<SystemCheck> {
    let buckets = crate::bucket::bucket_added(session).unwrap_or_default();
    let names: Vec<String> = buckets.iter().map(|b| b.name().to_owned()).collect();
    if !has_main_bucket(&names) {
        return Some(SystemCheck::new(
            "Main bucket is not added.",
            vec!["run 'bagger bucket add main'"],
        ));
    }
    None
}

/// Whether any added bucket is `main`.
fn has_main_bucket(names: &[String]) -> bool {
    names.iter().any(|n| n.eq_ignore_ascii_case("main"))
}

/// Long-path support must be enabled (upstream `check_long_paths`).
/// Non-Windows platforms do not need this check.
fn check_long_paths() -> Option<SystemCheck> {
    #[cfg(not(windows))]
    {
        return None;
    }
    #[cfg(windows)]
    {
        if long_paths_enabled() {
            return None;
        }
        Some(SystemCheck::new(
            "LongPaths support is not enabled.",
            vec![
                "You can enable it by running (elevated):",
                "  Set-ItemProperty 'HKLM:\\SYSTEM\\CurrentControlSet\\Control\\FileSystem' -Name 'LongPathsEnabled' -Value 1",
            ],
        ))
    }
}

#[cfg(windows)]
fn read_hklm_dword(path: &str, name: &str) -> Option<u32> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let key = RegKey::predef(HKEY_LOCAL_MACHINE).open_subkey(path).ok()?;
    key.get_value::<u32, _>(name).ok()
}

#[cfg(windows)]
fn long_paths_enabled() -> bool {
    read_hklm_dword(
        r"SYSTEM\CurrentControlSet\Control\FileSystem",
        "LongPathsEnabled",
    )
    .is_some_and(|v| v != 0)
}

/// Developer Mode gates symlink creation without elevation
/// (upstream `Get-WindowsDeveloperModeStatus`). Non-Windows: skip.
fn check_developer_mode() -> Option<SystemCheck> {
    #[cfg(not(windows))]
    {
        return None;
    }
    #[cfg(windows)]
    {
        let enabled = read_hklm_dword(
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\AppModelUnlock",
            "AllowDevelopmentWithoutDevLicense",
        )
        .is_some_and(|v| v == 1);
        if enabled {
            return None;
        }
        Some(SystemCheck::new(
            "Windows Developer Mode is not enabled. Operations relevant to symlinks may fail without proper rights.",
            vec![
                "You may read more about the symlinks support here:",
                "  https://blogs.windows.com/windowsdeveloper/2016/12/02/symlinks-windows-10/",
            ],
        ))
    }
}

/// Realtime Defender scanning without a scoop exclusion slows down or
/// disrupts installs (upstream `check_windows_defender`). Passes when
/// Defender is off/absent, realtime monitoring is disabled, or the scoop
/// dir is excluded. Unreadable state is skipped, never alarmed.
fn check_windows_defender(session: &Session) -> Option<SystemCheck> {
    #[cfg(not(windows))]
    {
        let _ = session;
        return None;
    }
    #[cfg(windows)]
    {
        if !defender_running() {
            return None;
        }
        if realtime_monitoring_disabled() {
            return None;
        }
        let root = session.config().root_path().to_string_lossy().into_owned();
        if dir_excluded(&root) {
            return None;
        }
        Some(SystemCheck::new(
            "Windows Defender may slow down or disrupt installs with realtime scanning.",
            vec![
                "Consider running (elevated):",
                &format!("  Add-MpPreference -ExclusionPath '{root}'"),
            ],
        ))
    }
}

#[cfg(windows)]
fn defender_running() -> bool {
    // `sc query` avoids hosting PowerShell for a service state read.
    std::process::Command::new("sc.exe")
        .args(["query", "WinDefend"])
        .output()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .any(|l| l.contains("STATE") && l.contains("RUNNING"))
        })
        .unwrap_or(false)
}

#[cfg(windows)]
fn realtime_monitoring_disabled() -> bool {
    // `Get-MpPreference.DisableRealtimeMonitoring` without the Security
    // module: the backing `DisableRealtimeMonitoring` value.
    read_hklm_dword(
        r"SOFTWARE\Microsoft\Windows Defender\Real-Time Protection",
        "DisableRealtimeMonitoring",
    )
    .is_some_and(|v| v != 0)
}

#[cfg(windows)]
fn defender_exclusions() -> Option<Vec<String>> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey(r"SOFTWARE\Microsoft\Windows Defender\Exclusions\Paths")
        .ok()?;
    let mut out = Vec::new();
    for entry in key.enum_values().flatten() {
        out.push(entry.0);
    }
    Some(out)
}

/// Whether `dir` (or a parent of it) is excluded. Comparison is
/// case-insensitive with trailing separators trimmed.
fn exclusion_covers(exclusions: &[String], dir: &str) -> bool {
    fn trim(s: &str) -> String {
        s.trim_end_matches(['/', '\\']).to_lowercase()
    }
    let dir = trim(dir);
    exclusions.iter().any(|e| {
        let e = trim(e);
        !e.is_empty() && (e == dir || dir.starts_with(&format!("{e}\\")))
    })
}

#[cfg(windows)]
fn dir_excluded(dir: &str) -> bool {
    defender_exclusions()
        .map(|ex| exclusion_covers(&ex, dir))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn main_bucket_match_is_case_insensitive() {
        assert!(has_main_bucket(&["main".to_owned(), "extras".to_owned()]));
        assert!(has_main_bucket(&["Main".to_owned()]));
        assert!(!has_main_bucket(&["extras".to_owned()]));
        assert!(!has_main_bucket(&[]));
    }

    #[test]
    fn exclusion_matching_covers_parents() {
        let exclusions = vec!["C:\\Users\\v\\scoop".to_owned(), "D:\\tools\\".to_owned()];
        assert!(exclusion_covers(&exclusions, "C:\\Users\\v\\scoop"));
        assert!(exclusion_covers(&exclusions, "C:\\USERS\\V\\SCOOP\\apps"));
        assert!(exclusion_covers(&exclusions, "d:\\tools"));
        assert!(!exclusion_covers(&exclusions, "C:\\Users\\v\\scoop-evil"));
        assert!(!exclusion_covers(&exclusions, "C:\\other"));
        assert!(!exclusion_covers(&[], "C:\\other"));
    }
}
