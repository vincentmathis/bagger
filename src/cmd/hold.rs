use clap::{ArgAction, Parser};
use crossterm::style::Stylize;
use scoop_rs::{operation, Session};

use crate::util;
use crate::Result;

/// Hold package(s) to disable changes
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package(s) to be held
    #[arg(required= true, action = ArgAction::Append, add = clap_complete::engine::ArgValueCompleter::new(crate::complete::installed_apps))]
    package: Vec<String>,
    /// Hold globally installed app(s) (requires admin rights)
    #[arg(short = 'g', long, action = ArgAction::SetTrue)]
    global: bool,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    util::apply_global_flag(args.global, session)?;
    for name in &args.package {
        print!("Holding {}...", name);
        match operation::package_hold(session, name, true) {
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
