#![allow(unused)]
//! This crate aims to provide a full-featured, practical, and efficient Rust
//! reimplementation of [Scoop], the Windows command-line installer. It is a
//! library crate providing the core functionality of interacting with Scoop,
//! and is not intended to be used directly by end users. Developers who wish
//! to implement a Scoop frontend or make use of Scoop's functionality in their
//! own applications may use this crate. For end users, they may take a glance
//! at [bagger], a reference implementation built on top of this crate, which
//! provides a command-line interface similar to Scoop.
//!
//! # Overview
//!
//! The primary type in this crate is a [`Session`], which is an entry point to
//! this crate. A session instance is basically a handle to the global state of
//! scoop_rs. Most of the functions exposed by this crate take a session as
//! their first argument.
//!
//! ## Examples
//!
//! Initialize a Scoop session, get the configuration associated with the
//! session, and print the root path of Scoop to stdout:
//!
//! ```rust
//! use scoop_rs::Session;
//! let session = Session::new();
//! let config = session.config();
//! println!("{}", config.root_path().display());
//! ```
//!
//! [Scoop]: https://scoop.sh/
//! [bagger]: https://github.com/vincentmathis/bagger
#[macro_use]
extern crate serde;

mod bucket;
mod cache;
mod config;
mod constant;
pub mod diagnostic;
mod env;
mod error;
mod event;
mod internal;
mod manifest_cache;
mod package;
mod persist;
mod psmodule;
mod session;
mod shim;
mod shortcut;
#[cfg(test)]
pub(crate) mod test_support;

pub mod arch;
pub mod operation;

pub use error::Error;
pub use event::Event;
pub use internal::fs::{remove_symlink, symlink_dir};
pub use internal::os::running_apps as running_apps_under;
pub use package::{has_install_metadata, install_info_path, installed_manifest_path};
pub use package::{Package, QueryOption, SyncOption, Transaction};
pub use persist::unlink_links as persist_unlink_links;
pub use session::Session;
pub use shim::refresh as shim_refresh;
pub use shim::target_of as shim_target_of;
