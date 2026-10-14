use clap::{CommandFactory, Parser, Subcommand};
use crossterm::style::Stylize;
use scoop_rs::{operation, Session};

use crate::Result;

/// Manage command aliases
#[derive(Debug, Parser)]
pub struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// List all aliases
    #[clap(alias = "ls")]
    List,
    /// Add a new alias
    Add {
        /// The alias name
        name: String,
        /// The command to alias (with arguments)
        command: String,
    },
    /// Remove an alias
    #[clap(alias = "rm")]
    Remove {
        /// The alias name to remove
        name: String,
    },
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    // Aliases are stored in config under the "alias" key as a JSON object
    let aliases = get_aliases(session)?;

    match args.command {
        Command::List => {
            if aliases.is_empty() {
                println!("No aliases defined.");
            } else {
                println!("{}", "Aliases:".bold());
                for (name, cmd) in &aliases {
                    println!("  {} = {}", name.clone().green(), cmd.clone().yellow());
                }
            }
        }
        Command::Add { name, command } => {
            if aliases.contains_key(&name) {
                eprintln!("Alias '{}' already exists.", name);
                return Ok(());
            }

            let mut new_aliases = aliases.clone();
            new_aliases.insert(name.clone(), command.clone());
            set_aliases(session, &new_aliases)?;

            println!("Added alias '{}' = '{}'", name.green(), command);
        }
        Command::Remove { name } => {
            if !aliases.contains_key(&name) {
                eprintln!("Alias '{}' not found.", name);
                return Ok(());
            }

            let mut new_aliases = aliases;
            let removed = new_aliases.remove(&name);
            set_aliases(session, &new_aliases)?;

            if let Some(cmd) = removed {
                println!("Removed alias '{}' (was '{}')", name.green(), cmd);
            }
        }
    }

    Ok(())
}

fn get_aliases(session: &Session) -> Result<std::collections::HashMap<String, String>> {
    let config_str = operation::config_list(session)?;
    let config: serde_json::Value = serde_json::from_str(&config_str)?;
    let aliases = config["alias"]
        .as_object()
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default();
    Ok(aliases)
}

fn set_aliases(
    session: &Session,
    aliases: &std::collections::HashMap<String, String>,
) -> Result<()> {
    let alias_json = serde_json::to_string(aliases)?;
    operation::config_set(session, "alias", &alias_json)?;
    Ok(())
}

/// Expand a leading user alias into argv form: `[bin, head, tail…]` becomes
/// `[bin, expansion…, tail…]`. Builtins (and their clap aliases) always win
/// over user aliases; chains resolve up to a cap; self-loops are left for
/// clap to reject. Values split on whitespace.
pub(crate) fn expand_argv(argv: Vec<std::ffi::OsString>) -> Vec<std::ffi::OsString> {
    let Some(head) = argv.get(1).and_then(|s| s.to_str()).map(str::to_owned) else {
        return argv;
    };
    let builtin: std::collections::HashSet<String> = crate::cmd::Cli::command()
        .get_subcommands()
        .flat_map(|sc| {
            let mut names = vec![sc.get_name().to_owned()];
            names.extend(sc.get_aliases().map(str::to_owned));
            names
        })
        .collect();
    if builtin.contains(&head) {
        return argv;
    }
    let session = Session::new();
    let Ok(aliases) = get_aliases(&session) else {
        return argv;
    };
    let mut out = vec![argv[0].clone()];
    out.extend(expand_words(&head, &builtin, &aliases));
    out.extend_from_slice(&argv[2.min(argv.len())..]);
    out
}

/// Pure word expansion for [`expand_argv`]: resolve the head through the
/// alias map, splicing each expansion in place of the previous head.
fn expand_words(
    head: &str,
    builtin: &std::collections::HashSet<String>,
    aliases: &std::collections::HashMap<String, String>,
) -> Vec<std::ffi::OsString> {
    let mut words = vec![std::ffi::OsString::from(head)];
    for _ in 0..10 {
        let first = words[0].to_str().unwrap_or_default().to_owned();
        if builtin.contains(&first) {
            break;
        }
        let Some(expansion) = aliases.get(&first) else {
            break;
        };
        let parts: Vec<std::ffi::OsString> = expansion
            .split_whitespace()
            .map(std::ffi::OsString::from)
            .collect();
        if parts.is_empty() || parts[0].to_str() == Some(first.as_str()) {
            break;
        }
        words.splice(0..1, parts);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;

    fn builtin() -> std::collections::HashSet<String> {
        ["install", "i", "alias", "uninstall", "rm"]
            .into_iter()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn alias_expansion_rules() {
        let aliases: std::collections::HashMap<String, String> = [
            ("ll".to_string(), "list -u".to_string()),
            ("a".to_string(), "b".to_string()),
            ("b".to_string(), "install".to_string()),
            ("loop".to_string(), "loop".to_string()),
        ]
        .into_iter()
        .collect();

        // Plain substitution, original position kept.
        assert_eq!(
            expand_words("ll", &builtin(), &aliases),
            vec![
                std::ffi::OsString::from("list"),
                std::ffi::OsString::from("-u")
            ]
        );
        // Chains resolve to the builtin.
        assert_eq!(
            expand_words("a", &builtin(), &aliases),
            vec![std::ffi::OsString::from("install")]
        );
        // Self-loops are left for clap to reject.
        assert_eq!(
            expand_words("loop", &builtin(), &aliases),
            vec![std::ffi::OsString::from("loop")]
        );
        // Unknown words pass through untouched.
        assert_eq!(
            expand_words("nope", &builtin(), &aliases),
            vec![std::ffi::OsString::from("nope")]
        );
    }
}
