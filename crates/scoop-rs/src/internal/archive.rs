#![allow(dead_code)]
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Format {
    /// .bz2
    Bz2,
    /// .gz
    Gzip,
    /// .rar
    Rar,
    /// .7z, .xz, .tar, .msi (all handled by 7z)
    XZip,
    /// .zip
    Zip,
    /// .zstd
    Zst,
}

impl Format {
    pub fn detect<P: AsRef<Path>>(path: P) -> Option<Format> {
        let ext = path
            .as_ref()
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase());
        match ext.as_deref() {
            Some("bz2") | Some("tbz2") | Some("tbz") => Some(Format::Bz2),
            Some("gz") | Some("tgz") => Some(Format::Gzip),
            Some("rar") => Some(Format::Rar),
            Some("7z") | Some("xz") | Some("lzma") | Some("iso") | Some("lzh") | Some("nupkg")
            | Some("tar") | Some("msi") => Some(Format::XZip),
            Some("zip") => Some(Format::Zip),
            Some("zst") => Some(Format::Zst),
            _ => None,
        }
    }
}

/// Find the path to the 7z executable.
///
/// Probes `PATH` first (`7z.exe`, then bare `7z`, then `7z.cmd` — batch
/// shims execute fine via process spawn), then the Scoop `7zip` app under
/// the `$SCOOP` environment root, an explicitly given root, and finally
/// next to the current executable.
fn find_7z_exe(scoop_root: Option<&Path>) -> Option<PathBuf> {
    // Try 7z on PATH (including .cmd shims, which spawn fine)
    for probe in ["7z.exe", "7z", "7z.cmd"] {
        if crate::internal::os::is_program_available(probe) {
            return Some(PathBuf::from(probe));
        }
    }

    // Try Scoop's own 7zip app at <root>\apps\7zip\current\7z.exe
    let mut roots: Vec<PathBuf> = vec![];
    if let Some(root) = std::env::var_os("SCOOP") {
        roots.push(PathBuf::from(&root));
    }
    if let Some(root) = scoop_root {
        roots.push(root.to_owned());
    }

    // Try relative to the current executable: <exe>\..\apps\7zip\current\7z.exe
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(parent) = exe_path.parent() {
            roots.push(parent.join(".."));
        }
    }

    for root in roots {
        let candidate = root
            .join("apps")
            .join("7zip")
            .join("current")
            .join("7z.exe");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    None
}

/// Determine if the file is a self-extracting 7z archive (PE header).
fn is_7z_sfx(path: &Path) -> bool {
    if let Ok(mut file) = std::fs::File::open(path) {
        let mut header = [0u8; 4];
        if file.read(&mut header).is_ok() {
            return header == *b"MZ\0\0";
        }
    }
    false
}

/// Extract an MSI package to the given destination directory.
///
/// Mirrors upstream `Expand-MsiArchive`: with `use_lessmsi` (and the Scoop
/// `lessmsi` app present) extraction goes through lessmsi, otherwise the
/// system `msiexec.exe` administrative install is used and its `SourceDir`
/// output is promoted to the destination.
pub fn extract_msi<P: AsRef<Path>, Q: AsRef<Path>>(
    src: P,
    dst: Q,
    use_lessmsi: bool,
    lessmsi_exe: Option<PathBuf>,
) -> crate::error::Fallible<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    crate::internal::fs::ensure_dir(dst)?;

    if use_lessmsi {
        let exe = lessmsi_exe.ok_or_else(|| {
            crate::Error::Custom(
                "cannot extract MSI: 'use_lessmsi' is set but the lessmsi app is not installed"
                    .to_string(),
            )
        })?;
        let output = std::process::Command::new(&exe)
            .arg("x")
            .arg(src)
            .arg(format!("{}\\", dst.to_string_lossy()))
            .output()?;
        if !output.status.success() {
            return Err(crate::Error::Custom(format!(
                "lessmsi failed to extract '{}': {}",
                src.display(),
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        return Ok(());
    }

    let source_dir = dst.join("SourceDir");
    let _ = std::fs::remove_dir_all(&source_dir);
    let status = std::process::Command::new("msiexec.exe")
        .arg("/a")
        .arg(src)
        .arg("/qn")
        .arg(format!(
            "TARGETDIR={}\\SourceDir",
            dst.to_string_lossy().trim_end_matches('\\')
        ))
        .status()?;
    if !status.success() {
        return Err(crate::Error::Custom(format!(
            "msiexec failed to extract '{}' ({})",
            src.display(),
            status
        )));
    }
    if source_dir.is_dir() {
        move_dir_contents(&source_dir, dst)?;
        let _ = std::fs::remove_dir_all(&source_dir);
    }
    Ok(())
}

/// Move the contents of `src` into `dst`, merging directories recursively.
fn move_dir_contents(src: &Path, dst: &Path) -> crate::error::Fallible<()> {
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            if to.is_dir() {
                move_dir_contents(&from, &to)?;
                let _ = std::fs::remove_dir(&from);
            } else {
                if to.exists() {
                    let _ = std::fs::remove_file(&to);
                }
                std::fs::rename(&from, &to)?;
            }
        } else {
            if to.is_dir() {
                continue;
            }
            if to.exists() {
                let _ = std::fs::remove_file(&to);
            }
            std::fs::rename(&from, &to)?;
        }
    }
    Ok(())
}

/// Extract an archive to the given destination directory using 7z.
///
/// This function supports the following archive formats: `.zip`, `.7z`, `.gz`,
/// `.bz2`, `.rar`, `.tar`, `.xz`, `.lzh`, `.iso`, `.zst`, `.nupkg`.
pub fn extract<P: AsRef<Path>, Q: AsRef<Path>>(src: P, dst: Q) -> crate::error::Fallible<()> {
    extract_with_root(src, dst, None)
}

/// Extract an archive, additionally probing the Scoop `7zip` app under the
/// given root when `7z` is not on `PATH`.
pub fn extract_with_root<P: AsRef<Path>, Q: AsRef<Path>>(
    src: P,
    dst: Q,
    scoop_root: Option<&Path>,
) -> crate::error::Fallible<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    crate::internal::fs::ensure_dir(dst)?;

    let exe = find_7z_exe(scoop_root).ok_or_else(|| {
        crate::Error::Custom(
            "7z executable not found. Please install the 7zip package via Scoop or add 7z to PATH."
                .to_string(),
        )
    })?;

    let mut cmd = std::process::Command::new(&exe);
    cmd.arg("x")
        .arg("-y")
        .arg(format!("-o{}", dst.to_string_lossy()));

    if is_7z_sfx(src) {
        cmd.arg("-sfx");
    }

    cmd.arg(src);

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(crate::Error::Custom(format!(
            "failed to extract archive '{}':\nstdout: {}\nstderr: {}",
            src.display(),
            stdout,
            stderr
        )));
    }

    Ok(())
}
