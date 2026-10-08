#![allow(unused_assignments)]
use clap::{ArgAction, Parser};
use crossterm::{
    cursor,
    style::Stylize,
    terminal::{Clear, ClearType},
    ExecutableCommand,
};
use scoop_rs::{operation, Event, Session, SyncOption};
use std::io::Write;

use crate::{cui, flavor, util, Result};

/// Install package(s)
#[derive(Debug, Parser)]
#[clap(arg_required_else_help = true)]
pub struct Args {
    /// The package(s) to install (name, manifest URL, or local manifest file)
    #[arg(required = true, action = ArgAction::Append)]
    package: Vec<String>,
    /// Download package(s) without performing installation
    #[arg(short = 'd', long, action = ArgAction::SetTrue)]
    download_only: bool,
    /// Ignore failures to ensure a complete transaction
    #[arg(short = 'f', long, action = ArgAction::SetTrue)]
    ignore_failure: bool,
    /// Leverage cache and suppress network access
    #[arg(short = 'o', long, action = ArgAction::SetTrue)]
    offline: bool,
    /// Assume yes to all prompts and run non-interactively
    #[arg(short = 'y', long, action = ArgAction::SetTrue)]
    assume_yes: bool,
    /// Ignore cache and force download
    #[arg(short = 'D', long, action = ArgAction::SetTrue)]
    ignore_cache: bool,
    /// Do not install dependencies (may break packages)
    #[arg(short = 'I', long, action = ArgAction::SetTrue)]
    independent: bool,
    /// Do not replace package(s)
    #[arg(short = 'R', long, action = ArgAction::SetTrue)]
    no_replace: bool,
    /// Escape hold to allow changes on held package(s)
    #[arg(short = 'S', long, action = ArgAction::SetTrue)]
    escape_hold: bool,
    /// Do not upgrade package(s)
    #[arg(short = 'U', long, action = ArgAction::SetTrue)]
    no_upgrade: bool,
    /// Skip package integrity check
    #[arg(long, action = ArgAction::SetTrue)]
    no_hash_check: bool,
    /// Install globally for all users (requires admin rights)
    #[arg(short = 'g', long, action = ArgAction::SetTrue)]
    global: bool,
    /// Target architecture (32bit, 64bit or arm64; default: host arch)
    #[arg(long)]
    arch: Option<String>,
}

impl Args {
    /// Build args for programmatic installs (e.g. `import`).
    pub(crate) fn from_packages(
        package: Vec<String>,
        assume_yes: bool,
        arch: Option<String>,
    ) -> Self {
        Self {
            package,
            download_only: false,
            ignore_failure: false,
            offline: false,
            assume_yes,
            ignore_cache: false,
            independent: false,
            no_replace: false,
            escape_hold: false,
            no_upgrade: false,
            no_hash_check: false,
            global: false,
            arch,
        }
    }
}

pub fn execute(args: Args, session: &Session) -> Result<()> {
    crate::util::apply_global_flag(args.global, session)?;
    crate::util::apply_arch_flag(args.arch.as_deref(), session)?;

    let mut options = vec![];

    if args.assume_yes {
        options.push(SyncOption::AssumeYes);
    }

    if args.download_only {
        options.push(SyncOption::DownloadOnly);
    }

    if args.escape_hold {
        options.push(SyncOption::EscapeHold);
    }

    if args.ignore_failure {
        options.push(SyncOption::IgnoreFailure);
    }

    if args.ignore_cache {
        options.push(SyncOption::IgnoreCache);
    }

    if args.no_upgrade {
        options.push(SyncOption::NoUpgrade);
    }

    if args.no_replace {
        options.push(SyncOption::NoReplace);
    }

    if args.offline {
        options.push(SyncOption::Offline);
    }

    if args.independent {
        options.push(SyncOption::NoDependencies);
    }

    if args.no_hash_check {
        options.push(SyncOption::NoHashCheck);
    }

    let rx = session.event_bus().receiver();
    let tx = session.event_bus().sender();

    let show_manifest = session.config().show_manifest();

    let mut stdout = std::io::stdout();
    let _ = stdout.execute(cursor::Hide);

    let mut dlprogress = cui::MultiProgressUI::new();

    let handle = std::thread::spawn(move || {
        while let Ok(event) = rx.recv() {
            match event {
                Event::PackageResolveStart => println!("{}", flavor::progress("resolve")),
                Event::PackageRunningSkipped { name, processes } => {
                    println!("Skipping '{name}' (running: {processes}). Quit it and retry to include it.");
                }
                Event::PackageAlreadyInstalled { name, version } => {
                    println!(
                        "{}",
                        format!("'{name}' ({version}) is already installed. Skipping.").yellow()
                    );
                }
                Event::PackageDownloadSizingStart => println!("{}", flavor::progress("sizing")),
                Event::PackageDownloadStart => println!("{}", flavor::progress("download")),
                Event::PackageDownloadProgress(ctx) => {
                    let ident = ctx.ident.to_owned();
                    let url = ctx.url.to_owned();
                    let filename = ctx.filename.to_owned();
                    let dltotal = ctx.dltotal;
                    let dlnow = ctx.dlnow;

                    dlprogress.update(ident, url, filename, dltotal, dlnow);
                }
                Event::PackageDownloadDone => {}
                Event::PackageIntegrityCheckStart => {
                    println!("{}", flavor::progress("integrity"))
                }
                Event::PackageIntegrityCheckProgress(ctx) => {
                    let mut stdout = std::io::stdout();
                    stdout
                        .execute(cursor::MoveToPreviousLine(1))
                        .unwrap()
                        .execute(Clear(ClearType::CurrentLine))
                        .unwrap();
                    println!("{}{}", flavor::progress("integrity"), ctx.dark_grey());
                }
                Event::PackageIntegrityCheckDone => {
                    let mut stdout = std::io::stdout();
                    stdout
                        .execute(cursor::MoveToPreviousLine(1))
                        .unwrap()
                        .execute(Clear(ClearType::CurrentLine))
                        .unwrap();
                    println!("{}{}", flavor::progress("integrity"), "Ok".green());
                }
                Event::PromptPackageCandidate(pkgs) => {
                    let name = pkgs[0].split_once('/').unwrap().1;
                    println!("Found multiple candidates for package '{}':\n", name);
                    for (i, pkg) in pkgs.iter().enumerate() {
                        println!("  {}: {}", i, pkg);
                    }

                    let mut index = 0;
                    let mut stdout = std::io::stdout();
                    let _ = stdout.execute(cursor::Show);
                    loop {
                        print!("\nPlease select one, enter the number to continue: ");
                        std::io::stdout().flush().unwrap();
                        let mut input = String::new();
                        std::io::stdin().read_line(&mut input).unwrap();
                        let parsed = input.trim().parse::<usize>();
                        if let Ok(num) = parsed {
                            index = num;
                            // bounds check
                            if num < pkgs.len() {
                                break;
                            }
                        }
                    }

                    let _ = stdout.execute(cursor::Hide);
                    let _ = tx.send(Event::PromptPackageCandidateResult(index));
                }
                Event::PromptTransactionNeedConfirm(transaction) => {
                    if show_manifest {
                        cui::show_manifests(&transaction);
                    }
                    if let Some(install) = transaction.install_view() {
                        println!("The following packages will be INSTALLED:");
                        let output = install
                            .iter()
                            .map(|p| {
                                format!(
                                    "{}{}{}",
                                    p.ident(),
                                    "-".dark_grey(),
                                    p.version().dark_grey(),
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("  ");
                        println!("  {}", output);
                    }

                    if let Some(upgrade) = transaction.upgrade_view() {
                        if transaction.install_view().is_some() {
                            println!();
                        }
                        println!("The following packages will be UPGRADED:");
                        let output = upgrade
                            .iter()
                            .map(|p| {
                                format!(
                                    "{}{}{}",
                                    p.ident(),
                                    "-".dark_grey(),
                                    // Force reinstalls target the current
                                    // version, which has no "upgradable" ref.
                                    p.upgradable_version()
                                        .unwrap_or_else(|| p.version())
                                        .dark_grey(),
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("  ");
                        println!("  {}", output);
                    }

                    if let Some(replace) = transaction.replace_view() {
                        if transaction.install_view().is_some()
                            || transaction.upgrade_view().is_some()
                        {
                            println!();
                        }
                        println!("The following packages will be REPLACED:");
                        let output = replace
                            .iter()
                            .map(|p| {
                                format!(
                                    "{}{}/{}",
                                    p.installed_bucket().unwrap().dark_grey().crossed_out(),
                                    p.bucket(),
                                    p.name(),
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("  ");
                        println!("  {}", output);
                    }

                    if let Some(download_size) = transaction.download_size() {
                        let out = util::humansize(download_size.total, true);
                        if download_size.total > 0 {
                            if download_size.estimated {
                                println!(
                                    "\nTotal download size: {} {}",
                                    out,
                                    "(estimated)".dark_grey()
                                );
                            } else {
                                println!("\nTotal download size: {}", out);
                            }
                        } else {
                            println!("\n{}", flavor::progress("cached"));
                        }
                    }

                    let mut stdout = std::io::stdout();
                    let _ = stdout.execute(cursor::Show);
                    let answer = cui::prompt_yes_no();
                    let _ = tx.send(Event::PromptTransactionNeedConfirmResult(answer));
                    let _ = stdout.execute(cursor::Hide);
                }
                Event::PackageInstalledNotes {
                    ident,
                    notes,
                    suggest,
                } => {
                    if !notes.is_empty() {
                        println!("\nNotes for {}:", ident);
                        for note in &notes {
                            println!("  {}", note);
                        }
                    }
                    if !suggest.is_empty() {
                        println!("Suggestions for {}:", ident);
                        for entry in &suggest {
                            println!("  {}", entry);
                        }
                    }
                }
                Event::PackageSyncDone => break,
                _ => {}
            }
        }
    });

    let queries = args.package.iter().map(|s| s.as_str()).collect::<Vec<_>>();
    operation::package_sync(session, queries, options)?;

    handle.join().unwrap();

    let mut stdout = std::io::stdout();
    let _ = stdout.execute(cursor::Show);

    Ok(())
}
