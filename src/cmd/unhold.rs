use clap::{ArgAction, Parser};
use crossterm::style::Stylize;
use scoop_rs::{operation, Session};

use crate::util;
use crate::Result;

/// Unhold package(s) to enable changes
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package(s) to be unheld
    #[arg(required = true, action = ArgAction::Append, add = clap_complete::engine::ArgValueCompleter::new(crate::complete::installed_apps))]
    package: Vec<String>,
    /// Unhold globally installed app(s) (requires admin rights)
    #[arg(short = 'g', long, action = ArgAction::SetTrue)]
    global: bool,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    util::apply_global_flag(args.global, session)?;
    let packages = args.package.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    for name in packages {
        print!("Unholding {}...", name);
        match operation::package_hold(session, name, false) {
            Ok(..) => {
                println!("{}", "Ok".green());
            }
            Err(err) => {
                println!("{}", "Err".red());
                return Err(err.into());
            }
        }
    }
    Ok(())
}
