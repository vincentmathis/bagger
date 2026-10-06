use crate::error::Fallible;
use crate::internal;
use crate::package::Package;
use crate::Session;
use std::path::Path;

/// Build the Scoop PowerShell execution context variables.
///
/// Returns a list of `key= value` pairs that are set as environment variables
/// when invoking PowerShell, so that scripts can reference them.
fn build_context_variables(
    session: &Session,
    package: &Package,
    cmd: &str,
    working_dir: &Path,
) -> Vec<(&'static str, String)> {
    let config = session.config();
    let root_path = config.root_path().to_string_lossy().to_string();
    let bucket = package.bucket();
    let bucket_path = config
        .root_path()
        .join("buckets")
        .join(bucket)
        .to_string_lossy()
        .to_string();

    let version = package.version().to_string();
    let arch = package
        .manifest()
        .architecture()
        .map(|a| {
            if cfg!(target_arch = "x86_64") && a.amd64.is_some() {
                "64bit"
            } else if cfg!(target_arch = "aarch64") && a.aarch64.is_some() {
                "arm64"
            } else if cfg!(target_arch = "x86") && a.ia32.is_some() {
                "32bit"
            } else {
                "64bit"
            }
        })
        .unwrap_or("64bit")
        .to_string();

    let dir = working_dir.to_string_lossy().to_string();

    let global = if config.use_isolated_path().is_some() {
        "1"
    } else {
        "0"
    }
    .to_string();

    vec![
        ("SCOOP", root_path),
        ("SCOOP_BUCKET", bucket.to_string()),
        ("SCOOP_BUCKET_DIR", bucket_path.clone()),
        ("SCOOP_PACKAGE", package.name().to_string()),
        ("SCOOP_VERSION", version.clone()),
        ("SCOOP_ARCH", arch.clone()),
        ("SCOOP_CMD", cmd.to_string()),
        ("dir", dir),
        ("bucketdir", bucket_path),
        ("bucket", bucket.to_string()),
        ("version", version),
        ("architecture", arch),
        ("fname", package.name().to_string()),
        ("global", global),
    ]
}

/// Execute a PowerShell script with the Scoop execution context.
///
/// This function builds the appropriate environment variables and invokes
/// `powershell.exe` to run the given script lines.
///
/// # Arguments
///
/// * `session` - The Scoop session
/// * `package` - The package being installed
/// * `cmd` - The command being run (e.g., "install", "uninstall")
/// * `script` - The script lines to execute
/// * `working_dir` - The working directory for the script (typically the app dir)
pub fn invoke_script(
    session: &Session,
    package: &Package,
    cmd: &str,
    script: &[&str],
    working_dir: &Path,
) -> Fallible<()> {
    // Build the PowerShell script from the lines
    let ps_script = script.join("\n");

    // Build environment variables for the PowerShell context
    let env_vars = build_context_variables(session, package, cmd, working_dir);

    let mut command = std::process::Command::new("powershell.exe");
    command
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(&ps_script)
        .current_dir(working_dir);

    // Set environment variables
    for (key, value) in &env_vars {
        command.env(key, value);
    }

    if let Some(tx) = session.emitter() {
        let _ = tx.send(crate::Event::PackageCommitStart(format!(
            "running powershell script for {}",
            package.name()
        )));
    }

    let output = command.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(crate::Error::Custom(format!(
            "PowerShell script execution failed for '{}':\nstdout: {}\nstderr: {}",
            package.name(),
            stdout,
            stderr
        )));
    }

    if let Some(tx) = session.emitter() {
        let _ = tx.send(crate::Event::PackageCommitDone(package.name().to_owned()));
    }

    Ok(())
}

/// Execute a PowerShell script and capture its standard output.
///
/// Same Scoop execution context as [`invoke_script`], but returns the
/// trimmed stdout instead of discarding it. Used by `checkver.script`
/// manifests, where the script is expected to print the latest version
/// (optionally post-processed with `checkver.regex`).
pub fn invoke_script_capture(
    session: &Session,
    package: &Package,
    cmd: &str,
    script: &[&str],
    working_dir: &Path,
) -> Fallible<String> {
    let ps_script = script.join("\n");
    let env_vars = build_context_variables(session, package, cmd, working_dir);

    let mut command = std::process::Command::new("powershell.exe");
    command
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(&ps_script)
        .current_dir(working_dir);

    for (key, value) in &env_vars {
        command.env(key, value);
    }

    let output = command.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(crate::Error::Custom(format!(
            "PowerShell script execution failed for '{}':\nstdout: {}\nstderr: {}",
            package.name(),
            stdout,
            stderr
        )));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::manifest::Manifest;

    /// Build a minimal package for script tests.
    fn test_package() -> Package {
        let manifest = Manifest::parse_bytes(
            br#"{
                "version": "1.0",
                "homepage": "https://example.com",
                "license": "MIT"
            }"#,
            Path::new("test-pkg.json"),
        )
        .expect("fixture manifest should parse");
        Package::from("test-pkg", "main", manifest)
    }

    #[test]
    #[cfg(windows)]
    fn capture_returns_trimmed_stdout() {
        let session = Session::new();
        let pkg = test_package();
        let out = invoke_script_capture(
            &session,
            &pkg,
            "checkver",
            &["\"7.8.9\""],
            &std::env::temp_dir(),
        )
        .expect("powershell should run");
        assert_eq!(out, "7.8.9");
    }

    #[test]
    #[cfg(windows)]
    fn capture_reports_failure() {
        let session = Session::new();
        let pkg = test_package();
        let err = invoke_script_capture(
            &session,
            &pkg,
            "checkver",
            &["exit 3"],
            &std::env::temp_dir(),
        )
        .expect_err("failing script should error");
        assert!(err.to_string().contains("failed"));
    }
}
