use std::fs;
use std::path::{Path, PathBuf};

use anyhow::anyhow;
use clap::{crate_name, CommandFactory, Parser};
use clap_complete::Shell;

use crate::{cmd::Cli, Result};

/// Marker lines framing the auto-installed block.
///
/// Shared with `scripts/install.ps1`, so a manual `--install` and a
/// scripted install never duplicate each other.
pub const MARKER_START: &str = "# >>> bagger shell completions >>>";
pub const MARKER_END: &str = "# <<< bagger shell completions <<<";

/// Generate shell completions
#[derive(Parser, Debug)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The shell type
    shell: Shell,
    /// Install the completions into the shell's startup files instead of printing them
    #[clap(long)]
    install: bool,
}

pub fn execute(args: Args) -> Result<()> {
    let mut buf = vec![];
    clap_complete::generate(args.shell, &mut Cli::command(), crate_name!(), &mut buf);
    let script = String::from_utf8(buf).expect("clap_complete did not generate valid shell script");
    if !args.install {
        println!("{script}");
        return Ok(());
    }
    match args.shell {
        Shell::PowerShell => install_powershell()?,
        Shell::Bash => append_rc_file("bash", home_file(".bashrc")?)?,
        Shell::Zsh => append_rc_file("zsh", home_file(".zshrc")?)?,
        Shell::Fish => write_generated(
            &home_file(".config")?.join("fish/completions/bagger.fish"),
            script.as_bytes(),
        )?,
        _ => {
            // Elvish has no startup-file convention we can wire automatically:
            // save the module and tell the user how to load it.
            let path = home_file(".elvish")?.join("lib/bagger.elv");
            write_generated(&path, script.as_bytes())?;
            println!("Add `use bagger` to ~/.elvish/rc.elv to enable completions.");
        }
    }
    Ok(())
}

fn home_file(name: &str) -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("could not resolve home directory"))?;
    Ok(home.join(name))
}

/// The live-evaluated block written into PowerShell profiles.
///
/// Evaluated at every shell startup so completions always match the
/// installed binary; the guards keep orphaned blocks silent after an
/// uninstall. Native argument completers only fire on PowerShell 7+ —
/// Windows PowerShell 5.1 registers them without error but never invokes
/// them — hence the version gate (avoids taxing 5.1 shell startup).
fn powershell_block() -> String {
    format!(
        "{MARKER_START}\nif (($PSVersionTable.PSVersion.Major -ge 7) -and (Get-Command bagger -ErrorAction SilentlyContinue)) {{\n    bagger completions powershell | Out-String | Invoke-Expression\n}}\n{MARKER_END}\n"
    )
}

/// The live-evaluated block appended to bash/zsh rc files.
fn sh_block(shell: &str) -> String {
    format!("{MARKER_START}\ncommand -v bagger >/dev/null && eval \"$(bagger completions {shell})\"\n{MARKER_END}\n")
}

fn install_powershell() -> Result<()> {
    let documents =
        dirs::document_dir().ok_or_else(|| anyhow!("could not resolve Documents folder"))?;
    // Cover both Windows PowerShell 5.1 and PowerShell 7+ profile locations.
    for dir in ["WindowsPowerShell", "PowerShell"] {
        let profile = documents.join(dir).join("Microsoft.PowerShell_profile.ps1");
        append_block(&profile, &powershell_block())?;
    }
    println!("Restart your shell to use completions.");
    Ok(())
}

fn append_rc_file(shell: &str, rc: PathBuf) -> Result<()> {
    append_block(&rc, &sh_block(shell))?;
    println!(
        "Restart your shell (or source {}) to use completions.",
        rc.display()
    );
    Ok(())
}

/// Write `block` to `path`, creating parents as needed. Idempotent:
/// files already containing the exact block are left untouched, while files
/// with a stale block under the same marker (e.g. from an older bagger) are
/// healed in place, so upgrades never stack duplicate blocks.
fn append_block(path: &Path, block: &str) -> Result<()> {
    if path.is_file() {
        let existing = fs::read_to_string(path)?;
        if existing.contains(block) {
            println!("Completions already installed in {}.", path.display());
            return Ok(());
        }
        if existing.contains(MARKER_START) {
            update_block(path, &existing, block)?;
            return Ok(());
        }
        let mut content = existing;
        if !content.ends_with('\n') {
            content.push('\n');
        }
        content.push_str(block);
        fs::write(path, content)?;
    } else {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, block)?;
    }
    println!("Installed completions in {}.", path.display());
    Ok(())
}

/// Replace the stale `MARKER_START..=MARKER_END` span with the new block.
fn update_block(path: &Path, existing: &str, block: &str) -> Result<()> {
    let start = existing.find(MARKER_START).unwrap_or(0);
    let end = existing[start..]
        .find(MARKER_END)
        .map(|i| start + i + MARKER_END.len())
        .unwrap_or(existing.len());
    let mut content = String::with_capacity(existing.len() + block.len());
    content.push_str(existing[..start].trim_end_matches(['\r', '\n']));
    content.push('\n');
    content.push_str(block);
    content.push_str(&existing[end..]);
    fs::write(path, content)?;
    println!("Updated completions in {}.", path.display());
    Ok(())
}

/// Write generated completion source to `path` (overwriting; the content is
/// derived, so re-running is naturally idempotent).
fn write_generated(path: &Path, script: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, script)?;
    println!("Installed completions in {}.", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn powershell_block_is_guarded_and_marked() {
        let block = powershell_block();
        assert!(block.contains(MARKER_START));
        assert!(block.contains(MARKER_END));
        assert!(block.contains("Get-Command bagger"));
        assert!(block.contains("bagger completions powershell"));
        // Native completers only fire on PS 7+; don't tax 5.1 startups.
        assert!(block.contains("PSVersionTable"));
    }

    #[test]
    fn sh_block_evals_matching_shell() {
        let block = sh_block("bash");
        assert!(block.contains(MARKER_START));
        assert!(block.contains("bagger completions bash"));
    }

    #[test]
    fn append_block_is_idempotent() {
        let dir = std::env::temp_dir().join(format!("bagger-compl-idem-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("profile.ps1");
        append_block(&path, &powershell_block()).unwrap();
        append_block(&path, &powershell_block()).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content.matches(MARKER_START).count(), 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn append_block_heals_stale_block() {
        let dir = std::env::temp_dir().join(format!("bagger-compl-heal-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("profile.ps1");
        let stale = format!("$profile = 1\n{MARKER_START}\nold block\n{MARKER_END}\n$other = 2\n");
        fs::write(&path, &stale).unwrap();
        append_block(&path, &powershell_block()).unwrap();
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content.matches(MARKER_START).count(), 1);
        assert!(content.contains("PSVersionTable"));
        assert!(content.contains("$profile = 1"));
        assert!(content.contains("$other = 2"));
        assert!(!content.contains("old block"));
        // Second run is a no-op.
        append_block(&path, &powershell_block()).unwrap();
        let again = fs::read_to_string(&path).unwrap();
        assert_eq!(again, content);
        let _ = fs::remove_dir_all(&dir);
    }
}
