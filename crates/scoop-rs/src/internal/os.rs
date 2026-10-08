#![allow(dead_code)]
use once_cell::sync::Lazy;
use std::path::Path;
use std::sync::Mutex;
use sysinfo::ProcessExt;
use sysinfo::ProcessRefreshKind;
use sysinfo::System;
use sysinfo::SystemExt;

use crate::error::{Error, Fallible};

static SYSINFO: Lazy<Mutex<System>> = Lazy::new(|| Mutex::new(System::default()));

pub fn os_is_arch64() -> bool {
    match std::mem::size_of::<&char>() {
        4 => false,
        8 => true,
        _ => panic!("unexpected os arch"),
    }
}

/// Check if a given executable is available on the system.
pub fn is_program_available(exe: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for p in path.split(';') {
            let path = Path::new(p).join(exe);
            if std::fs::metadata(path).is_ok() {
                return true;
            }
        }
    }
    false
}

/// Whether the current process runs elevated (admin).
///
/// Backstop for operations upstream only attempts as admin (e.g. global
/// persist ACLs); the CLI already requires admin for global scope.
pub fn is_elevated() -> bool {
    #[cfg(windows)]
    {
        // `net session` succeeds only for elevated processes; output is
        // captured so nothing leaks to the console.
        std::process::Command::new("net")
            .arg("session")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        false
    }
}

pub fn running_apps(path: &Path) -> Fallible<Vec<String>> {
    // static REGEX_APPS_PATH: Lazy<Regex> = Lazy::new(|| {
    //     RegexBuilder::new(r".*?apps[\\/]+(?P<app>[a-zA-Z0-9-_.]+)[\\/]+.*")
    //         .build()
    //         .unwrap()
    // });
    let mut sys = SYSINFO.lock().map_err(|e| Error::Custom(e.to_string()))?;

    sys.refresh_processes_specifics(ProcessRefreshKind::new());

    // Find all running processes of installed Scoop apps.
    let mut proc_names = sys
        .processes()
        .values()
        .filter_map(|p| {
            let exe_path = p.exe();

            if exe_path.starts_with(path) {
                Some(p.name().to_owned())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    proc_names.sort();
    proc_names.dedup();
    Ok(proc_names)
}
