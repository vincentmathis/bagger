use clap::{ArgAction, Parser};
use scoop_rs::{operation, Session, SyncOption};

use crate::{flavor, Result};

/// Download a package without installing it
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package(s) to download
    #[arg(required = true, action = ArgAction::Append)]
    package: Vec<String>,

    /// Assume yes to all prompts
    #[arg(short = 'y', long, action = ArgAction::SetTrue)]
    assume_yes: bool,

    /// Ignore local cache and force download
    #[arg(short = 'f', long, action = ArgAction::SetTrue)]
    force: bool,

    /// Download for offline use (implies no hash check)
    #[arg(long, action = ArgAction::SetTrue)]
    offline: bool,

    /// Target architecture (32bit, 64bit or arm64; default: host arch)
    #[arg(long)]
    arch: Option<String>,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    crate::util::apply_arch_flag(args.arch.as_deref(), session)?;

    let queries = args.package.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    let mut options = vec![SyncOption::DownloadOnly];

    if args.assume_yes {
        options.push(SyncOption::AssumeYes);
    }

    if args.force {
        options.push(SyncOption::IgnoreCache);
    }

    if args.offline {
        options.push(SyncOption::Offline);
        options.push(SyncOption::NoHashCheck);
    }

    let rx = session.event_bus().receiver();
    let _tx = session.event_bus().sender();

    let handle = std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            match event {
                scoop_rs::Event::PackageResolveStart => println!("{}", flavor::progress("resolve")),
                scoop_rs::Event::PackageDownloadSizingStart => {
                    println!("{}", flavor::progress("sizing"))
                }
                scoop_rs::Event::PackageDownloadStart => {
                    println!("{}", flavor::progress("download"))
                }
                scoop_rs::Event::PackageDownloadDone => {
                    println!("{}", flavor::progress("download_complete"))
                }
                scoop_rs::Event::PackageIntegrityCheckStart => {
                    println!("{}", flavor::progress("verifying"))
                }
                scoop_rs::Event::PackageIntegrityCheckDone => {
                    println!("{}", flavor::progress("verified"))
                }
                scoop_rs::Event::PackageSyncDone => break,
                _ => {}
            }
        }
    });

    operation::package_sync(session, queries, options)?;
    handle.join().unwrap();

    Ok(())
}
