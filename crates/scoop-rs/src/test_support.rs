//! Test-only helpers.
//!
//! Several tests point `Session` at scratch roots via the `SCOOP` /
//! `SCOOP_CACHE` environment variables. Process environment is global
//! state, so those tests must hold [`env_guard`] while mutating it;
//! otherwise a concurrent test creating its own `Session` could observe
//! a foreign root.

use std::sync::{Mutex, MutexGuard, OnceLock};

static ENV_GUARD: OnceLock<Mutex<()>> = OnceLock::new();

/// Hold while mutating process environment in tests.
///
/// The underlying mutex is NOT reentrant: never acquire it twice in one
/// scope (e.g. via shadowing `let _guard`), or the test will deadlock.
/// Split such cases into separate tests instead.
pub fn env_guard() -> MutexGuard<'static, ()> {
    ENV_GUARD
        .get_or_init(|| Mutex::new(()))
        .lock()
        .expect("test env guard poisoned")
}
