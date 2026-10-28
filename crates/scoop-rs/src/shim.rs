#![allow(dead_code)]
use std::path::{Path, PathBuf};

use crate::{error::Fallible, internal, package::Package, Event, Session};

#[derive(Debug)]
pub struct Shim<'a> {
    name: &'a str,
    real_name: &'a str,
    ty: ShimType,
    args: Option<Vec<&'a str>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ShimType {
    /// Bash script
    ///
    /// A shim will be treated as a Bash script if it does not have a file
    /// extension.
    Bash,

    /// Batch script
    ///
    /// A shim will be treated as a Batch script if it has a `.bat`/`.cmd` file
    /// extension.
    Batch,

    /// Executable
    ///
    /// A shim will be treated as an executable if it has a `.exe`/`.com` file
    /// extension.
    Exe,

    /// Java JAR
    ///
    /// A shim will be treated as a Java JAR if it has a `.jar` file extension.
    Java,

    /// PowerShell script
    ///
    /// A shim will be treated as a PowerShell script if it has a `.ps1` file
    /// extension.
    PowerShell,

    /// Python script
    ///
    /// A shim will be treated as a Python script if it has a `.py` file
    /// extension.
    Python,
}

impl<'a> Shim<'a> {
    /// Display name for a bin definition, mirroring [`Shim::new`] without
    /// constructing one: lone entries resolve to the target's basename
    /// without extension (`bin/tool.exe` → `tool`), pairs use the alias.
    pub(crate) fn name_of(def: &[&'a str]) -> &'a str {
        if def.len() == 1 {
            internal::path::leaf_base(def[0]).unwrap_or(def[0])
        } else {
            def[1]
        }
    }

    pub fn new(def: Vec<&'a str>) -> Shim<'a> {
        let length = def.len();
        assert_ne!(length, 0);

        let real_name = def[0];
        let name = Self::name_of(&def);

        let args = if length < 2 {
            None
        } else {
            Some(def[2..].to_vec())
        };

        let ty = Path::new(real_name)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| match ext.to_lowercase().as_str() {
                "bat" | "cmd" => ShimType::Batch,
                "exe" | "com" => ShimType::Exe,
                "jar" => ShimType::Java,
                "ps1" => ShimType::PowerShell,
                "py" => ShimType::Python,
                _ => ShimType::Bash,
            })
            .unwrap_or(ShimType::Bash);

        Shim {
            name,
            real_name,
            ty,
            args,
        }
    }
}

/// Add shims for a package.
///
/// Creates shim files in the Scoop `shims` directory for each binary defined
/// in the package's `bin` field.
pub fn add(session: &Session, package: &Package) -> Fallible<()> {
    let config = session.config();
    let shims_dir = config.root_path().join("shims");
    internal::fs::ensure_dir(&shims_dir)?;

    if let Some(bins) = package.manifest().bin() {
        let pkg_name = package.name();
        let version = if config.no_junction() {
            package
                .installed_version()
                .unwrap_or_else(|| package.version())
                .to_owned()
        } else {
            "current".to_string()
        };

        for shim_def in bins {
            let shim = Shim::new(shim_def);

            // Determine the target binary path
            let apps_dir = config.root_path().join("apps");
            let bin_dir = apps_dir.join(pkg_name).join(&version);
            let target = with_path_fallback(shim.real_name, bin_dir.join(shim.real_name));

            // Older bagger versions wrote batch content into `{name}.exe`,
            // which shadows the working `.cmd` and fails on execution.
            // Remove the poison, but never touch a native binary.
            if shim.ty == ShimType::Exe {
                let poisoned = shims_dir.join(format!("{}.exe", shim.name));
                if is_poisoned_exe(&poisoned) {
                    let _ = std::fs::remove_file(&poisoned);
                }
            }

            // A shim pointing at a missing target is always broken (the
            // install never materialized it); say so loudly instead of
            // succeeding silently, mirroring upstream's abort.
            if !target.exists() {
                warn_missing_target(pkg_name, &shim, &target);
            }

            for (filename, content_kind) in shim_files(&shim) {
                create_shim(
                    &shims_dir.join(&filename),
                    &target,
                    &shim,
                    content_kind,
                    pkg_name,
                )?;
            }
        }
    }

    Ok(())
}

/// Warn about a shim whose target was never materialized.
///
/// Upstream aborts the install here; bagger warns and continues so one bad
/// `bin` entry cannot wedge an otherwise good install, but the breakage is
/// impossible to miss.
fn warn_missing_target(pkg_name: &str, shim: &Shim, target: &Path) {
    eprintln!(
        "warning: shim '{}' for '{}' points at missing target '{}'",
        shim.name,
        pkg_name,
        target.display()
    );
}

/// Shim files created for one `bin` entry.
///
/// Every name produced here must stay within the universe that `remove()`
/// cleans and `which`/`shim ls` understand: bare `{name}` plus standard
/// executable extensions.
///
/// NOTE: `{name}.exe` is deliberately never written. Upstream Scoop places
/// the native shim binary there; a script with an `.exe` extension cannot
/// execute (Windows reports os error 216 for it) and would shadow both the
/// native binary and `{name}.cmd` in `PATH` resolution. Console launches
/// resolve `{name}.cmd` fine.
fn shim_files(shim: &Shim) -> Vec<(String, ShimContent)> {
    match shim.ty {
        ShimType::Exe => vec![(format!("{}.cmd", shim.name), ShimContent::Batch)],
        ShimType::PowerShell => vec![
            (format!("{}.cmd", shim.name), ShimContent::PowerShellInvoke),
            (format!("{}.ps1", shim.name), ShimContent::PowerShell),
        ],
        // Extensionless (Bash) shims live at the bare name.
        ShimType::Bash => vec![(shim.name.to_owned(), ShimContent::Batch)],
        _ => vec![(format!("{}.cmd", shim.name), ShimContent::Batch)],
    }
}

/// Whether a `{name}.exe` file is bagger-poison: script content wearing an
/// executable extension.
///
/// Such files were written by older bagger versions and can neither execute
/// nor be shadowed — `PATH` resolution finds them first and fails. Native
/// (MZ-header) binaries are always left alone.
fn is_poisoned_exe(path: &Path) -> bool {
    let Ok(content) = std::fs::read(path) else {
        return false;
    };
    if content.len() >= 2 && content[0] == b'M' && content[1] == b'Z' {
        return false;
    }
    content.starts_with(b"@echo off") || content.starts_with(b"@rem ")
}

/// The script flavor written into a shim file.
#[derive(Clone, Copy)]
enum ShimContent {
    /// Batch wrapper (`@echo off`, direct or `java`/`python` launch).
    Batch,
    /// Batch wrapper invoking a PowerShell script target.
    PowerShellInvoke,
    /// Native PowerShell script.
    PowerShell,
}

/// Create a shim file that forwards execution to the target binary.
///
/// On collision with a *different* shim, the existing file is displaced
/// aside (suffixed with its owner's name, discovered from its target)
/// instead of overwritten: that is what the restore pass in [`remove`]
/// revives on uninstall. Identical content just overwrites (reinstalls
/// and junction-targeted upgrades must not spam backups).
fn create_shim(
    shim_path: &Path,
    target: &Path,
    shim: &Shim,
    content: ShimContent,
    pkg_name: &str,
) -> Fallible<()> {
    let content = create_shim_content(target, shim, content)?;
    // Never displace another package's shim: two packages declaring the same
    // bin name (e.g. `opencode` and `opencode2` both shipping `opencode.exe`)
    // must not clobber each other — the last installer wins, which breaks the
    // first package's shim (e.g. `opencode` launching `opencode2`'s binary).
    // Same-package reinstalls are unaffected because `existing == content`
    // short-circuits `displace_on_collision` below.
    if let Ok(existing) = std::fs::read_to_string(shim_path) {
        if existing != content {
            if let Some(existing_owner) = shim_target_owner(shim_path) {
                if existing_owner != pkg_name {
                    eprintln!(
                        "warning: shim '{}' for '{}' already owned by '{}'; skipping",
                        shim.name, pkg_name, existing_owner
                    );
                    return Ok(());
                }
            }
        }
    }
    displace_on_collision(shim_path, &content, pkg_name)?;
    // Clear any leftover (e.g. readonly) file before writing.
    let _ = std::fs::remove_file(shim_path);
    // On Windows, create a batch file as a simple shim
    // A proper implementation would embed the shim.exe binary
    #[cfg(windows)]
    {
        std::fs::write(shim_path, &content)?;
    }

    #[cfg(unix)]
    {
        std::fs::write(shim_path, &content)?;
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(shim_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(shim_path, perms)?;
    }

    Ok(())
}

/// Rename an existing, differing shim aside so [`remove`] can restore it.
///
/// Returns the backup path, if one was made.
fn displace_on_collision(shim_path: &Path, content: &str, pkg_name: &str) -> Fallible<()> {
    let existing = match std::fs::read_to_string(shim_path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    if existing == content {
        return Ok(());
    }
    // The backup is suffixed with the displaced shim's owner so removal
    // deletes exactly its own remnants and restores the right file.
    let owner = shim_target_owner(shim_path).unwrap_or_else(|| pkg_name.to_owned());
    let backup = shim_path.with_file_name(format!(
        "{}.{}",
        shim_path.file_name().unwrap().to_string_lossy(),
        owner
    ));
    // Never stack backups: an existing one is the older generation.
    if !backup.exists() {
        std::fs::rename(shim_path, &backup)?;
    }
    Ok(())
}

/// Best-effort owner of an existing shim: the app directory its target
/// points into (`…/apps/<name>/…`). Foreign shims yield `None`.
fn shim_target_owner(shim_path: &Path) -> Option<String> {
    let target = target_of(shim_path)?;
    let mut parts = target.components().peekable();
    while let Some(part) = parts.next() {
        if part.as_os_str() == "apps" {
            if let Some(name) = parts.next() {
                return name.as_os_str().to_str().map(|s| s.to_owned());
            }
        }
    }
    None
}

/// Generate shim script content for the given target.
fn create_shim_content(target: &Path, shim: &Shim, content: ShimContent) -> Fallible<String> {
    let target_str = target.to_string_lossy();

    let args_str = if let Some(args) = &shim.args {
        args.join(" ")
    } else {
        String::new()
    };

    if let ShimContent::PowerShell = content {
        return Ok(format!("& \"{}\" {} @args\n", target_str, args_str));
    }

    if let ShimContent::PowerShellInvoke = content {
        return Ok(format!(
            "@echo off\npowershell -NoProfile -ExecutionPolicy Bypass -File \"{}\" {} %*\n",
            target_str, args_str
        ));
    }

    #[cfg(windows)]
    {
        if shim.ty == ShimType::Exe || shim.ty == ShimType::Bash {
            return Ok(format!("@echo off\n\"{}\" {} %*\n", target_str, args_str));
        }
        if shim.ty == ShimType::Java {
            return Ok(format!(
                "@echo off\njava -jar \"{}\" {} %*\n",
                target_str, args_str
            ));
        }
        if shim.ty == ShimType::Python {
            return Ok(format!(
                "@echo off\npython \"{}\" {} %*\n",
                target_str, args_str
            ));
        }
    }

    #[cfg(unix)]
    {
        if shim.ty == ShimType::Exe || shim.ty == ShimType::Bash {
            return Ok(format!(
                "#!/bin/sh\n\"{}\" {} \"$@\"\n",
                target_str, args_str
            ));
        }
        if shim.ty == ShimType::Java {
            return Ok(format!(
                "#!/bin/sh\njava -jar \"{}\" {} \"$@\"\n",
                target_str, args_str
            ));
        }
        if shim.ty == ShimType::Python {
            return Ok(format!(
                "#!/bin/sh\npython \"{}\" {} \"$@\"\n",
                target_str, args_str
            ));
        }
    }

    Ok(format!(
        "#!/bin/sh\n\"{}\" {} \"$@\"\n",
        target_str, args_str
    ))
}

/// Extract the target path referenced by a shim file, if any.
///
/// Handles the upstream shim flavors:
/// - quoted targets (`"C:\…\app.exe"`, `& "…"`, `path = "…"`);
/// - unquoted sh-style comments (`# <path>` in bare files,
///   `@rem <path>` in `.cmd` files), preferred over quoted strings
///   further down such as `"$(wslpath …)"` wrappers;
/// - relative targets (`..\apps\…`), resolved against the shim directory.
///
/// Returns `None` for binary or otherwise unparseable shims.
pub fn target_of(shim_file: &Path) -> Option<PathBuf> {
    let content = std::fs::read_to_string(shim_file).ok()?;
    let parent = shim_file.parent();
    let mut fallback: Option<PathBuf> = None;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("@echo off") || line.starts_with("#!") {
            continue;
        }
        for marker in ["@rem ", "# "] {
            if let Some(rest) = line.strip_prefix(marker) {
                let rest = rest.trim().trim_matches('"');
                if looks_like_path(rest) {
                    return Some(resolve_target(parent, rest));
                }
            }
        }
        if fallback.is_none() {
            if let Some(start) = line.find('"') {
                if let Some(end) = line[start + 1..].find('"') {
                    fallback = Some(resolve_target(parent, &line[start + 1..start + 1 + end]));
                }
            }
        }
    }
    fallback
}

/// Heuristic: does this comment remainder look like a file path?
fn looks_like_path(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let bytes = s.as_bytes();
    s.contains(['\\', '/']) || (bytes.len() >= 2 && bytes[1] == b':')
}

/// Join a possibly-relative shim target onto the shim directory.
fn resolve_target(parent: Option<&Path>, target: &str) -> PathBuf {
    let path = PathBuf::from(target);
    if path.is_absolute() {
        path
    } else if let Some(dir) = parent {
        dir.join(path)
    } else {
        path
    }
}

/// Apply upstream's `(Get-Command $target).Source` fallback: a bare shim
/// target missing from the app dir may still resolve through `PATH` (e.g. a
/// system executable a manifest shims by name). Anything unresolvable is
/// returned as-is so the caller can warn like before.
fn with_path_fallback(real_name: &str, target: PathBuf) -> PathBuf {
    if target.exists() || real_name.contains(['/', '\\']) {
        return target;
    }
    search_path(real_name).unwrap_or(target)
}

/// Search process `PATH` for `file_name`, with `PATHEXT` probing on Windows
/// for extensionless names (mirroring `Get-Command`).
fn search_path(file_name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(file_name);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        if Path::new(file_name).extension().is_none() {
            if let Some(found) = probe_pathext(&dir, file_name) {
                return Some(found);
            }
        }
    }
    None
}

#[cfg(windows)]
fn probe_pathext(dir: &Path, file_name: &str) -> Option<PathBuf> {
    let pathext = std::env::var_os("PATHEXT")?;
    for ext in pathext.to_string_lossy().split(';') {
        let ext = ext.trim().trim_start_matches('.');
        if ext.is_empty() {
            continue;
        }
        let candidate = dir.join(format!("{file_name}.{ext}"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Repair executable shims for all installed apps.
///
/// Rewrites missing or outdated `{name}.cmd` wrappers for `Exe` bins and
/// removes poisoned (batch-content) `{name}.exe` files left by older bagger
/// versions. Native binaries are never touched. Returns the names of
/// packages that were repaired.
pub fn refresh(session: &Session) -> Fallible<Vec<String>> {
    let installed = crate::package::query::query_installed(session, &["*"], &[])?;
    let shims_dir = session.config().root_path().join("shims");

    let mut repaired = vec![];
    for pkg in installed.iter() {
        let mut needs_repair = false;
        if let Some(bins) = pkg.manifest().bin() {
            for shim_def in bins {
                let shim = Shim::new(shim_def);
                if shim.ty != ShimType::Exe {
                    continue;
                }
                let cmd = shims_dir.join(format!("{}.cmd", shim.name));
                let exe = shims_dir.join(format!("{}.exe", shim.name));
                // Outdated content counts as broken: regenerate the expected
                // wrapper in-memory and compare with what is on disk.
                let target = with_path_fallback(
                    shim.real_name,
                    shim_target_path(session, pkg.name(), shim.real_name),
                );
                let expected = create_shim_content(&target, &shim, ShimContent::Batch).ok();
                let actual = std::fs::read_to_string(&cmd).ok();
                if actual != expected || is_poisoned_exe(&exe) {
                    needs_repair = true;
                    break;
                }
            }
        }
        if needs_repair {
            add(session, pkg)?;
            repaired.push(pkg.name().to_owned());
        }
    }
    Ok(repaired)
}

/// Resolve the app-relative target of a shim entry to an absolute path.
///
/// Uses the same version/current layout as [`add`].
fn shim_target_path(session: &Session, pkg_name: &str, real_name: &str) -> PathBuf {
    let config = session.config();
    let apps_dir = config.root_path().join("apps");
    let version = if config.no_junction() {
        // `installed_version` is unavailable here; fall back to the newest
        // version directory, mirroring link resolution.
        apps_dir
            .join(pkg_name)
            .read_dir()
            .ok()
            .and_then(|entries| {
                entries
                    .flatten()
                    .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                    .filter(|e| e.file_name() != "current")
                    .max_by(|a, b| a.file_name().cmp(&b.file_name()))
                    .map(|e| e.file_name().to_string_lossy().into_owned())
            })
            .unwrap_or_else(|| "current".to_string())
    } else {
        "current".to_string()
    };
    apps_dir.join(pkg_name).join(version).join(real_name)
}

/// Remove shims by name (upgrade finalization): the stale set a new
/// manifest no longer declares. Matches the plain file plus anything
/// suffixed under it (`name.cmd.other`), with the same dot boundary the
/// restore scan uses, so `git` never takes `gitui.cmd` with it. Native
/// `.exe` files are never ours: only batch-content poison (our old bug)
/// is removed, real binaries are left alone.
pub(crate) fn remove_stale(
    session: &Session,
    stale: &std::collections::HashSet<String>,
) -> Fallible<()> {
    if stale.is_empty() {
        return Ok(());
    }
    let shims_dir = session.config().root_path().join("shims");
    let entries = match shims_dir.read_dir() {
        Ok(entries) => entries,
        Err(_) => return Ok(()),
    };
    for entry in entries.filter_map(|de| de.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        let hit = stale
            .iter()
            .any(|s| name == *s || name.starts_with(&format!("{s}.")));
        if !hit {
            continue;
        }
        if path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("exe"))
            && !is_poisoned_exe(&path)
        {
            continue;
        }
        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageShimRemoveProgress(name.clone()));
        }
        let _ = std::fs::remove_file(&path);
    }
    Ok(())
}

/// Remove shims for a package.
pub fn remove(session: &Session, package: &Package) -> Fallible<()> {
    assert!(package.is_installed());

    let config = session.config();
    let shims_dir = config.root_path().join("shims");

    if let Some(bins) = package.manifest().bin() {
        let pkg_name = package.name();
        let shims_dir_entries = shims_dir
            .read_dir()?
            .filter_map(Result::ok)
            .collect::<Vec<_>>();

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageShimRemoveStart);
        }

        for shim in bins.into_iter().map(Shim::new) {
            let exts = match shim.ty {
                ShimType::Exe => vec!["cmd", "exe", "shim"],
                ShimType::PowerShell => vec!["cmd", "ps1", ""],
                _ => vec!["cmd", ""],
            };

            // File names are built by appending, never `set_extension`:
            // that truncates dotted shim names (`python3.12` would address
            // `python3.cmd` — possibly another app's shim).
            let plain_name = |ext: &str| {
                if ext.is_empty() {
                    shim.name.to_owned()
                } else {
                    format!("{}.{ext}", shim.name)
                }
            };
            let suffixed_name = |ext: &str| {
                if ext.is_empty() {
                    format!("{}.{pkg_name}", shim.name)
                } else {
                    format!("{}.{ext}.{pkg_name}", shim.name)
                }
            };

            for ext in exts.into_iter() {
                let suffixed = shims_dir.join(suffixed_name(ext));

                if suffixed.exists() {
                    if let Some(tx) = session.emitter() {
                        let shim_name = suffixed.file_name().unwrap().to_string_lossy().to_string();
                        let _ = tx.send(Event::PackageShimRemoveProgress(shim_name));
                    }

                    // The file can vanish between the check and the removal
                    // (concurrent uninstall, AV quarantine); missing is fine.
                    internal::fs::tolerate_not_found(std::fs::remove_file(&suffixed))?;
                } else {
                    let plain = shims_dir.join(plain_name(ext));

                    // Only announce files that actually exist.
                    if plain.exists() {
                        if let Some(tx) = session.emitter() {
                            let shim_name =
                                plain.file_name().unwrap().to_string_lossy().to_string();
                            let _ = tx.send(Event::PackageShimRemoveProgress(shim_name));
                        }

                        let _ = std::fs::remove_file(&plain);
                    }

                    // restore alter shim
                    let fname = plain.file_name().unwrap().to_str().unwrap();
                    let mut alt_shims = shims_dir_entries
                        .iter()
                        .flat_map(|entry| {
                            let path = entry.path();
                            let name = path.file_name().unwrap().to_str().unwrap();

                            if name.starts_with(fname) && name != fname {
                                Some(entry)
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>();

                    if alt_shims.is_empty() {
                        continue;
                    }

                    // sort by modified time, so the latest one will be used
                    // when there are multiple alter shims for the same shim.
                    // Unreadable metadata sorts oldest instead of panicking.
                    if alt_shims.len() > 1 {
                        alt_shims.sort_by_key(|de| {
                            std::cmp::Reverse(
                                de.metadata()
                                    .and_then(|m| m.modified())
                                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                            )
                        });
                    }

                    let alt_shim = alt_shims.first().unwrap();
                    let alt_path = alt_shim.path();
                    let alt_path_new = alt_path.with_file_name(fname);
                    // A stale alter shim must not wedge the uninstall; warn
                    // loudly and continue instead.
                    if let Err(e) = std::fs::rename(&alt_path, &alt_path_new) {
                        eprintln!(
                            "warning: could not restore alternate shim '{}' for '{}': {e}",
                            alt_path.display(),
                            pkg_name,
                        );
                    }
                }
            }
        }

        if let Some(tx) = session.emitter() {
            let _ = tx.send(Event::PackageShimRemoveDone);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shim_of<'a>(def: &[&'a str]) -> Shim<'a> {
        Shim::new(def.to_vec())
    }

    #[test]
    fn created_files_stay_in_removable_universe() {
        // Every file add() creates must be discoverable by which/shim ls
        // and removable by remove(): bare name or standard exe extensions.
        // NOTE: `{name}.exe` is never written — a script wearing an `.exe`
        // extension cannot execute (os error 216) and shadows the working
        // `.cmd`; console launches resolve the `.cmd` fine.
        let cases = [
            (vec!["app.exe"], vec!["app.cmd"]),
            (vec!["tool.ps1"], vec!["tool.cmd", "tool.ps1"]),
            (vec!["run"], vec!["run"]),
            (vec!["prog.jar"], vec!["prog.cmd"]),
            (vec!["setup.bat"], vec!["setup.cmd"]),
        ];
        for (def, expected) in cases {
            let shim = shim_of(&def);
            let files: Vec<String> = shim_files(&shim)
                .into_iter()
                .map(|(name, _)| name)
                .collect();
            assert_eq!(files, expected, "shim files for {def:?}");
        }
    }

    #[test]
    fn shim_content_forwards_caller_arguments() {
        // Every shim flavor must pass caller arguments through to the
        // target (upstream `%*` / `"$@"` / `@args`); without this, e.g.
        // `7z x archive.7z` runs bare `7z`.
        let exe = shim_of(&["app.exe"]);
        let batch = create_shim_content(
            std::path::Path::new("C:\\x\\app.exe"),
            &exe,
            ShimContent::Batch,
        )
        .unwrap();
        assert!(batch.trim_end().ends_with("%*"), "exe batch: {batch:?}");

        let ps = shim_of(&["tool.ps1", "tool"]);
        let invoke = create_shim_content(
            std::path::Path::new("C:\\x\\tool.ps1"),
            &ps,
            ShimContent::PowerShellInvoke,
        )
        .unwrap();
        assert!(invoke.trim_end().ends_with("%*"), "ps1 invoke: {invoke:?}");
        let script = create_shim_content(
            std::path::Path::new("C:\\x\\tool.ps1"),
            &ps,
            ShimContent::PowerShell,
        )
        .unwrap();
        assert!(
            script.trim_end().ends_with("@args"),
            "ps1 script: {script:?}"
        );
    }

    #[test]
    fn poison_detection_keeps_native_binaries() {
        let dir = std::env::temp_dir().join("bagger-test-shim-poison");
        std::fs::create_dir_all(&dir).unwrap();

        // Batch content with an .exe extension: poison.
        let poisoned = dir.join("tool.exe");
        std::fs::write(&poisoned, "@echo off\n\"C:\\apps\\x\\tool.exe\"\n").unwrap();
        assert!(is_poisoned_exe(&poisoned));

        // Native binary: never poison, even with an .exe extension.
        let native = dir.join("real.exe");
        std::fs::write(&native, b"MZ-binary-junk").unwrap();
        assert!(!is_poisoned_exe(&native));

        // Missing file: not poison.
        assert!(!is_poisoned_exe(&dir.join("ghost.exe")));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn add_writes_cmd_and_heals_poison() {
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-shim-add");
        let _ = std::fs::remove_dir_all(&base);
        std::env::set_var("SCOOP", base.join("root"));
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));

        let manifest = crate::package::manifest::Manifest::parse_bytes(
            br#"{"version": "1.0", "homepage": "https://example.com", "license": "MIT", "bin": ["tool.exe"]}"#,
            std::path::Path::new("shim.json"),
        )
        .unwrap();
        let pkg = crate::package::Package::from("shimapp", "main", manifest);
        let session = Session::new();
        let shims = base.join("root/shims");

        // Poisoned .exe gets removed, working .cmd gets written.
        std::fs::create_dir_all(&shims).unwrap();
        std::fs::write(shims.join("tool.exe"), "@echo off\n\"C:\\x\\tool.exe\"\n").unwrap();
        add(&session, &pkg).unwrap();
        assert!(!shims.join("tool.exe").exists());
        let cmd = std::fs::read_to_string(shims.join("tool.cmd")).unwrap();
        assert!(cmd.contains("tool.exe"));

        // Native .exe is preserved, .cmd still written alongside.
        std::fs::write(shims.join("tool.exe"), b"MZ-native").unwrap();
        add(&session, &pkg).unwrap();
        assert_eq!(&std::fs::read(shims.join("tool.exe")).unwrap()[..2], b"MZ");
        assert!(shims.join("tool.cmd").is_file());

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn remove_tolerates_missing_shims_and_restores_alters() {
        // Regression test: uninstalling after a partial failure (shims
        // already gone) must converge, not abort with os error 2.
        let _guard = crate::test_support::env_guard();
        let base = std::env::temp_dir().join("bagger-test-shim-remove");
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("root");
        let current = root.join("apps/unshim/current");
        std::fs::create_dir_all(&current).unwrap();
        std::fs::write(
            current.join("manifest.json"),
            r#"{"version": "1.0", "homepage": "https://example.com", "license": "MIT",
                "bin": [["unshim.exe"], ["ghost.exe"], ["altapp.exe"]]}"#,
        )
        .unwrap();
        std::fs::write(
            current.join("install.json"),
            r#"{"architecture": "64bit", "bucket": "fake"}"#,
        )
        .unwrap();
        let shims = root.join("shims");
        std::fs::create_dir_all(&shims).unwrap();
        std::fs::write(shims.join("unshim.cmd"), "@echo off\n").unwrap();
        // ghost.cmd deliberately absent (vanished between installs).
        std::fs::write(shims.join("altapp.cmd"), "old\n").unwrap();
        std::fs::write(shims.join("altapp.cmd.other"), "restored\n").unwrap();

        std::env::set_var("SCOOP", &root);
        std::env::set_var("SCOOP_GLOBAL", base.join("global"));
        std::env::set_var("SCOOP_CACHE", base.join("cache"));
        let session = Session::new();
        let pkgs = crate::package::query::query_installed(&session, &["*"], &[]).unwrap();
        let pkg = pkgs.iter().find(|p| p.name() == "unshim").unwrap();
        remove(&session, pkg).unwrap();

        assert!(!shims.join("unshim.cmd").exists());
        // The overlapped alternate shim is restored under its plain name.
        assert_eq!(
            std::fs::read_to_string(shims.join("altapp.cmd")).unwrap(),
            "restored\n"
        );

        std::env::remove_var("SCOOP");
        std::env::remove_var("SCOOP_GLOBAL");
        std::env::remove_var("SCOOP_CACHE");
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn collision_displaces_for_restore() {
        // A colliding install renames the existing shim aside (suffixed
        // with its discovered owner) instead of overwriting it; identical
        // content is left alone.
        let dir =
            std::env::temp_dir().join(format!("bagger-test-shim-collide-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let shim = dir.join("tool.cmd");
        std::fs::write(
            &shim,
            "@echo off\n\"C:\\apps\\oldapp\\current\\tool.exe\" %*\n",
        )
        .unwrap();

        displace_on_collision(
            &shim,
            "@echo off\n\"C:\\apps\\newapp\\current\\tool.exe\" %*\n",
            "newapp",
        )
        .unwrap();
        assert!(!shim.exists());
        assert_eq!(
            std::fs::read_to_string(dir.join("tool.cmd.oldapp")).unwrap(),
            "@echo off\n\"C:\\apps\\oldapp\\current\\tool.exe\" %*\n"
        );

        // Same content: no backup, file untouched.
        std::fs::write(&shim, "same\n").unwrap();
        displace_on_collision(&shim, "same\n", "newapp").unwrap();
        assert!(std::fs::read_to_string(&shim).unwrap() == "same\n");
        assert!(!dir.join("tool.cmd.newapp").exists());

        // Foreign target (no apps/ segment): suffix the installing package.
        std::fs::write(&shim, "foreign\n").unwrap();
        displace_on_collision(&shim, "different\n", "newapp").unwrap();
        assert!(dir.join("tool.cmd.newapp").is_file());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn target_of_parses_shim_flavors() {
        let dir = std::env::temp_dir().join("bagger-test-shim-target");
        std::fs::create_dir_all(&dir).unwrap();

        let batch = dir.join("a.cmd");
        std::fs::write(&batch, "@echo off\n\"C:\\apps\\x\\app.exe\" --flag\n").unwrap();
        assert_eq!(
            target_of(&batch).as_deref(),
            Some(std::path::Path::new("C:\\apps\\x\\app.exe"))
        );

        let invoke = dir.join("b.cmd");
        std::fs::write(
            &invoke,
            "@echo off\npowershell -NoProfile -ExecutionPolicy Bypass -File \"C:\\apps\\x\\t.ps1\"\n",
        )
        .unwrap();
        assert_eq!(
            target_of(&invoke).as_deref(),
            Some(std::path::Path::new("C:\\apps\\x\\t.ps1"))
        );

        let ps1 = dir.join("c.ps1");
        std::fs::write(&ps1, "& \"C:\\apps\\x\\t.ps1\" @args\n").unwrap();
        assert_eq!(
            target_of(&ps1).as_deref(),
            Some(std::path::Path::new("C:\\apps\\x\\t.ps1"))
        );

        let binary = dir.join("d.exe");
        std::fs::write(&binary, b"\x7fELF-binary-junk").unwrap();
        assert_eq!(target_of(&binary), None);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn target_of_prefers_sh_style_comments() {
        // Upstream sh-style shims record the target as an unquoted comment;
        // quoted `"$(wslpath …)"` wrappers further down must not win.
        let dir = std::env::temp_dir().join("bagger-test-shim-comments");
        std::fs::create_dir_all(&dir).unwrap();

        let bare = dir.join("tool");
        std::fs::write(
            &bare,
            "#!/bin/sh\n# C:\\apps\\tool\\current\\tool\nif [ $WSL_INTEROP ]\nthen\n  \"$(wslpath -u 'C:\\apps\\tool\\current\\tool')\"  \"$@\"\nfi\n",
        )
        .unwrap();
        assert_eq!(
            target_of(&bare).as_deref(),
            Some(std::path::Path::new("C:\\apps\\tool\\current\\tool"))
        );

        let cmd = dir.join("tool.cmd");
        std::fs::write(
            &cmd,
            "@rem C:\\apps\\tool\\current\\tool\n@echo off\nbash \"$(wslpath -u 'C:\\apps\\tool\\current\\tool')\"  %*\n",
        )
        .unwrap();
        assert_eq!(
            target_of(&cmd).as_deref(),
            Some(std::path::Path::new("C:\\apps\\tool\\current\\tool"))
        );

        // A comment without a path falls back to quoted strings.
        let odd = dir.join("odd.cmd");
        std::fs::write(
            &odd,
            "@rem just a note\n@echo off\n\"C:\\apps\\x\\t.exe\" %*\n",
        )
        .unwrap();
        assert_eq!(
            target_of(&odd).as_deref(),
            Some(std::path::Path::new("C:\\apps\\x\\t.exe"))
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn target_of_resolves_relative_targets() {
        // Upstream alias shims (scoop.ps1, sudo.ps1) use relative targets.
        let dir = std::env::temp_dir().join("bagger-test-shim-relative");
        let sub = dir.join("shims");
        std::fs::create_dir_all(&sub).unwrap();

        let alias = sub.join("scoop.ps1");
        std::fs::write(
            &alias,
            "# alias\n& \"..\\apps\\scoop\\current\\bin\\scoop.ps1\" @args\n",
        )
        .unwrap();
        assert_eq!(
            target_of(&alias),
            Some(sub.join("..\\apps\\scoop\\current\\bin\\scoop.ps1"))
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    /// Bare shim targets missing from the app dir resolve through `PATH`
    /// (upstream `(Get-Command $target).Source`); anything else passes
    /// through untouched for the missing-target warning.
    #[test]
    fn path_fallback_resolves_bare_names_only() {
        let _guard = crate::test_support::env_guard();
        let dir = std::env::temp_dir().join("bagger-test-shim-pathfind");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("sys-tool.exe"), "dummy").unwrap();

        let sep = if cfg!(windows) { ";" } else { ":" };
        let original = std::env::var_os("PATH").unwrap_or_default();
        let mut with_tmp = dir.to_string_lossy().into_owned();
        with_tmp.push_str(sep);
        with_tmp.push_str(&original.to_string_lossy());
        std::env::set_var("PATH", &with_tmp);

        // Bare name resolves; subdir-qualified names never hit PATH.
        assert_eq!(
            with_path_fallback("sys-tool.exe", PathBuf::from("sys-tool.exe")),
            dir.join("sys-tool.exe")
        );
        assert_eq!(
            with_path_fallback("bin/sys-tool.exe", PathBuf::from("bin/sys-tool.exe")),
            PathBuf::from("bin/sys-tool.exe")
        );
        // An existing target wins over PATH.
        assert_eq!(
            with_path_fallback("sys-tool.exe", dir.join("sys-tool.exe")),
            dir.join("sys-tool.exe")
        );
        // Unresolvable names pass through for the warning.
        assert_eq!(
            with_path_fallback("nope-missing.exe", PathBuf::from("nope-missing.exe")),
            PathBuf::from("nope-missing.exe")
        );

        std::env::set_var("PATH", &original);
        std::fs::remove_dir_all(&dir).ok();
    }
}
