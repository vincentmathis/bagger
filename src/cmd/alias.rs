use clap::{Parser, Subcommand};
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
