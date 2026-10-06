use clap::{ArgAction, Parser};
use scoop_rs::{operation, Session, SyncOption};

use crate::Result;

/// Download a package without installing it
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package(s) to download
    #[arg(required = true, action = ArgAction::Append)]
    package: Vec<String>,

    /// Download specific version(s)
    #[arg(short = 'v', long, action = ArgAction::Append)]
    version: Vec<String>,

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
    crate::util::apply_arch_flag(args.arch.as_deref())?;

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
                scoop_rs::Event::PackageResolveStart => println!("Resolving packages..."),
                scoop_rs::Event::PackageDownloadSizingStart => {
                    println!("Calculating download size...")
                }
                scoop_rs::Event::PackageDownloadStart => println!("Downloading packages..."),
                scoop_rs::Event::PackageDownloadDone => println!("Download complete."),
                scoop_rs::Event::PackageIntegrityCheckStart => println!("Verifying hashes..."),
                scoop_rs::Event::PackageIntegrityCheckDone => {
                    println!("Hash verification complete.")
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
