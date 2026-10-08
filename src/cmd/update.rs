use clap::{ArgAction, Parser};
use crossterm::{cursor, ExecutableCommand};
use scoop_rs::{operation, Event, Session};

use crate::{cui, flavor, Result};

/// Fetch and update subscribed buckets (or upgrade named apps)
#[derive(Debug, Parser)]
pub struct Args {
    /// The app(s) to upgrade (forwards to `upgrade`, like upstream
    /// `scoop update <app>`); empty updates buckets
    #[arg(action = ArgAction::Append)]
    package: Vec<String>,
    /// Assume yes to all prompts and run non-interactively
    #[arg(short = 'y', long, action = ArgAction::SetTrue)]
    assume_yes: bool,
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    if !args.package.is_empty() {
        println!(
            "Forwarding to 'bagger upgrade {}' (upstream 'scoop update <app>' upgrades apps).",
            args.package.join(" ")
        );
        let upgrade_args = super::upgrade::Args::from_packages(args.package, args.assume_yes);
        return super::upgrade::execute(upgrade_args, session);
    }

    let rx = session.event_bus().receiver();
    let show_log = session.config().show_update_log();

    let handle = std::thread::spawn(move || {
        let mut progress = cui::BucketUpdateUI::new();
        let mut logs: Vec<(String, Vec<String>)> = Vec::new();

        while let Ok(event) = rx.recv() {
            match event {
                Event::BucketUpdateProgress(ctx) => {
                    if ctx.state().started() {
                        progress.add(ctx.name());
                    } else if ctx.state().succeeded() {
                        progress.succeed(ctx.name());
                    } else {
                        let err_msg = ctx.state().failed().unwrap();
                        progress.fail(ctx.name(), err_msg);
                    }
                }
                Event::BucketUpdateLog(ctx) => {
                    logs.push((ctx.name().to_owned(), ctx.commits().to_vec()));
                }
                Event::BucketUpdateDone => break,
                _ => {}
            }
        }

        // move cursor to the end
        let mut stdout = std::io::stdout();
        let step = (progress.data.len() - progress.cursor) as u16;
        let _ = stdout.execute(cursor::MoveToNextLine(step)).unwrap();

        if show_log {
            for (bucket, commits) in &logs {
                println!("\n{bucket}:");
                for commit in commits {
                    println!("  {commit}");
                }
            }
        }
    });

    println!("{}", flavor::progress("buckets"));

    let mut stdout = std::io::stdout();
    let _ = stdout.execute(cursor::Hide);

    operation::bucket_update(session)?;

    handle.join().unwrap();

    let _ = stdout.execute(cursor::Show);

    Ok(())
}
