# Reimplementing Scoop in Rust: races, junctions, and tab-completion

[bagger](https://github.com/vincentmathis/bagger) is a Scoop-compatible package manager in Rust — same buckets, same directories, ~14ms startup instead of ~1s. The launch post covers the *what*. This one covers three bugs from the last week of the parity drive, because each one taught something general about reimplementing a tool people already use.

## 1. The uninstall that ate its own shim, then died

A user reported this:

```
Uninstalling ttt...
Removing shim 'ttt.cmd'
ERROR The system cannot find the file specified. (os error 2)
```

State afterwards: shim gone, app directory intact. Re-running the same command succeeded. Classic exists-check-then-act race — the file was there at the check, gone at the delete (concurrent uninstall, AV quarantine, whatever) — and the code treated it as fatal:

```rust
if shim_path.exists() {
    // announce...
    std::fs::remove_file(&shim_path)?;  // TOCTOU: dies with os error 2
}
```

The fix wasn't just that one line. I walked the entire uninstall tail — shim deletion, alternate-shim restore rename, app-dir and persist-dir removal — and made every step treat "already absent" as success:

```rust
/// Fold `NotFound` into success: the goal state (absent) is already
/// achieved, so e.g. retrying an uninstall after a partial failure, or
/// losing an exists-check-then-act race, must not abort the transaction.
pub fn tolerate_not_found(result: io::Result<()>) -> io::Result<()> {
    match result {
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        other => other,
    }
}
```

The principle: **uninstall is a convergence operation, not a script.** A retried uninstall after any partial failure must converge, not abort. Upstream Scoop scripts tend to assume the happy path because a human re-runs them; a reimplementation that wants to be boring infrastructure has to be retry-safe by construction. Regression test: fabricate a half-uninstalled layout (shims pre-deleted, app dir present), assert full removal returns Ok.

Same week, same theme: `remove_dir` fails on junctions with os error 5, and `current` junctions can carry the readonly flag from upstream's `attrib +R` — so link removal has to clear readonly first, then pick file-vs-dir removal by the *target's* metadata. Junctions are the gift that keeps giving on Windows.

## 2. The green line that printed three times

Same user, next day: uninstall printed `'x' was uninstalled.` twice. (Sometimes three times.) The event flow had four `PackageCommitDone` sends and one renderer:

- `sync.rs` install path: real commit → send. Fine.
- `sync.rs` remove path: real commit → send. Fine.
- `run_installer_file`: sent one *after running the installer*. Not a commit — a mid-transaction step.
- `invoke_script` (hook scripts): sent one *after every script*. Also a step.

So any package with a `.ps1` uninstaller printed the success line three times: once per hook, once per file-run, once for real. The fix is a one-line semantic rule: **steps don't report completion; transactions do.** Both mid-step sends deleted, regression test drains the event bus over binary, ps1, and hook-script installer paths and asserts zero `CommitDone`. Exactly-once delivery isn't just for queues — human-readable output needs it too, because users (correctly) read repetition as malfunction.

## 3. Tab-completion, and what PowerShell 5.1 won't tell you

`bagger uninstall neo<TAB>` should offer `neovim`. `clap_complete` has two mechanisms: static scripts (`bagger completions powershell`, frozen at generation time) and `CompleteEnv` dynamic hooks (the shell calls back into the binary on every TAB, with per-argument custom completers). I wired the dynamic one: 22 commands annotated, name providers per universe — installed apps for `uninstall`/`upgrade`/`hold`, bucket manifests for `install`/`search`, shim names for `which`, config keys for `config set`. 58ms per TAB in a live shell.

Three things bit, all worth knowing if you use `clap_complete`:

**PowerShell 5.1 registers native completers without error and never invokes them.** `Register-ArgumentCompleter -Native` succeeds, `TabExpansion2` returns nothing, no error anywhere. I verified this empirically before believing it — it means the "works on my machine (PS7), silent on yours (5.1)" failure mode is completely noiseless. The install path now gates the hook to PS 7+ so 5.1 shells don't pay a process spawn per startup for nothing.

**The completion protocol includes the binary name in the words.** The shell invokes `COMPLETE=powershell bagger -- bagger uninstall neo`, and the engine skips `args[0]` as the bin name with the index counting from zero *including* it. My first engine test passed `[uninstall, neo]`/index 1 and got silence — the harness was wrong, not the annotations. (Also: `#[arg(add = ...)]` in derive maps to a builder `.add()` call via clap's `unstable-ext` — it compiles fine once you enable `unstable-dynamic`, which took a while to confirm since the failure mode is, again, silence.)

**Stdout discipline is load-bearing.** During a completion run, stdout *is* the protocol (`value\thelp` lines). Anything else printed — a tracing log line, a stray `println!` in a provider — corrupts candidates. So the completion entry point runs before the logger initializes, and every name provider is total and silent: errors fold to an empty list. This is the kind of invariant that deserves a comment at the call site, because the next person to add a `debug!` in the query path will break TAB completion without touching any completion code. (tracing macros are no-ops without a subscriber, which is exactly why hook-first ordering matters.)

## The method behind it

All of this came from one loop: read the upstream PowerShell source for a flow, diff it against the Rust, write the missing piece, prove it with a unit test *and* a live run. The repo keeps the scoreboard as `SCOOP_FEATURES.md` — every command and manifest field, implemented or missing, with evidence. And two honest edges I won't sand down: bagger writes `.cmd` shims where Scoop writes `.exe` (each tool only manages its own flavor on a shared root), and dynamic completion needs PS 7+.

If you run Scoop on Windows: `scoop bucket add bagger https://github.com/vincentmathis/bagger-bucket && scoop install bagger`, then try to break parity. The ledger tells you what's covered; anything you find becomes the next entry.
