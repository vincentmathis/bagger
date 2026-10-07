# bagger

> Same buckets. Heavy machinery.

```raw
                          _________
                         |  CABIN  |
              ___________|_________|__________________
             /                                          \
     .---.  /                                            \     _______
    / o o \/                                              \___/       \
   | o   o |====================BOOM========================| COUNTER |
   | o + o |------------------------------------------------| WEIGHT  |
    \ o o /\                                                \_______/
     `---`  \____________________________________________/
             _/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_/_\_
            |_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|_|
```

*Bagger 288 (13,500 tonnes). Our binary is 8.5 MB. Same energy.*

Bagger is a Rust rewrite of Scoop for Windows, built like its namesake:
enormous capacity, relentless throughput, no wasted motion. One native
binary, no PowerShell startup, working with the Scoop buckets you already use.

Moves mountains. Installs apps.

[![crate](https://img.shields.io/crates/v/bagger)](https://crates.io/crates/bagger)
[![license][license-badge]](LICENSE)

## Why a 13,500-tonne machine to install `jq`

- **Speed.** Native binary, no shell startup cost, parallel manifest parsing.
  Numbers below, measured — not projected.
- **Zero setup.** One 8.5 MB `.exe`. No execution-policy fiddling: the
  installer runs in-memory (like Scoop's own), and bagger passes
  `-ExecutionPolicy Bypass` itself whenever a manifest needs PowerShell.
- **Compatible.** Reads your existing Scoop buckets, `config.json`, cache,
  and `apps/` layout in place. Set `SCOOP` to your current root and keep
  digging. See the [compatibility table](#compatibility) for exactly
  what that promise covers — and what it doesn't.

## Benchmarks

Median of 5 runs, fresh process each time, one warmup discarded.
Ryzen 7 3700X, Windows, 102 installed apps, 7 buckets.
`bagger` 0.1.0-beta.9 (release build) vs Scoop at current `master`
invoked through its `scoop.ps1` shim (which is the point: the ~1 s
PowerShell startup is included in Scoop's column).

| Operation            | Scoop (PS) | bagger | Speedup |
| :---                 | ---:       | ---:   | ---:    |
| `--version`          | 993 ms     | 17 ms  | ~58x    |
| `list`               | 1405 ms    | 26 ms  | ~54x    |
| `search python`      | 1515 ms    | 34 ms  | ~45x    |
| `info 7zip`          | 810 ms     | 30 ms  | ~27x    |

The manifest parser is parallel, so parsing 6,100 manifests takes ~30 ms
*without* any cache — enabling `use_sqlite_cache` measured 31 ms on the
same query. The cache exists for constrained disks, not for speed.

## Compatibility

Bagger shares Scoop's on-disk surfaces, so the two tools can work the same
root. Anything not listed here is not promised.

| Surface | Scoop | bagger | Notes |
| :--- | :---: | :---: | :--- |
| Bucket dirs + manifest JSON | ✓ | ✓ | reads your existing buckets in place; all 7 built-in buckets known |
| `config.json` keys | ✓ | ✓ | same file (`config list` prints its path); unknown keys ignored |
| `apps/<app>/<version>` + `current` junction | ✓ | ✓ | identical layout; upgrades keep version history |
| `install.json` (arch/bucket/url) | ✓ | ✓ | written on every install |
| Cache names `app#version#sha7.ext` | ✓ | ✓ | same naming scheme — caches are interchangeable |
| `shims/` (`.exe`/`.cmd`/`.ps1`/`.shim`) | ✓ | ✓ | same directory, symmetric add/remove |
| `persist/`, `modules/`, shortcuts | ✓ | ✓ | same locations |
| `-g/--global` under `%ProgramData%\scoop` | ✓ (admin) | ✓ (admin-gated) | gate verified; cache stays user-scoped |
| Hook scripts (`$dir`, `Expand-7zipArchive`, …) | ✓ | ✓ in source (unreleased) | full hook scope + archive helpers ship after `beta.9` |
| `BAGGER_FLAVOR=heavy` progress strings | — | ✓ | opt-in only; default output is script-compatible |
| Self-update (`update scoop`) | ✓ | — | reinstall via `install.ps1` or `cargo install -f bagger` |
| Non-Windows | — | — | Windows-only, like Scoop |

## Install

### One-liner (Windows, user scope)

```ps1
iwr -useb https://raw.githubusercontent.com/vincentmathis/bagger/main/scripts/install.ps1 | iex
```

System-wide instead (requires admin):

```ps1
iex (iwr -useb https://raw.githubusercontent.com/vincentmathis/bagger/main/scripts/install.ps1).Content -args -System
```

### From crates.io

```sh
cargo install bagger
```

### From source

```sh
git clone https://github.com/vincentmathis/bagger
cd bagger
cargo build --release
# the binary lives at target/release/bagger.exe
```

🚧 **Stability caveat**: `bagger` is on a pre-1.0 track; the core Scoop
command surface is fully implemented, but individual flag behaviour may
differ slightly from upstream Scoop. Pinned releases and checksums are on
the [GitHub releases page](https://github.com/vincentmathis/bagger/releases).

---

## Commands

The command line interface mirrors Scoop (run `bagger help <command>` for details).

```raw
$ bagger help
Bagger is a CLI implementation of Scoop in Rust

Usage: bagger.exe [OPTIONS] <COMMAND>

Commands:
  alias        Manage command aliases
  autofetch    Fetch latest versions and preview autoupdate URLs for apps
  bucket       Manage manifest buckets
  cache        Package cache management
  cat          Inspect the manifest of a package
  checkup      Check for updates and report app status
  checkver     Check for app updates without modifying anything
  cleanup      Cleanup apps by removing old versions
  completions  Generate shell completions
  config       Configuration management
  create       Create a new Scoop bucket or manifest template
  depends      Show dependencies for a package
  download     Download a package without installing it
  export       Export list of installed apps to stdout (JSON)
  hold         Hold package(s) to disable changes
  home         Browse the homepage of a package
  import       Import apps from an export file (or stdin) and install them
  info         Show package(s) basic information
  install      Install package(s)
  list         List installed package(s)
  prefix       Get the installation path of a package
  reset        Reset an installed package to a specific version or re-extract it
  search       Search available package(s)
  shim         Manage shims
  unhold       Unhold package(s) to enable changes
  uninstall    Uninstall package(s)
  update       Fetch and update subscribed buckets
  upgrade      Upgrade installed package(s)
  virustotal   Scan downloaded app archives with VirusTotal (requires API key)
  which        Find which application owns a given executable (shim)
  status       Show status of installed apps: held, upgradable, running processes
  help         Print this message or the help of a given subcommand(s)

Options:
  -v, --verbose...  Increase logging verbosity
  -q, --quiet...    Decrease logging verbosity
  -h, --help        Print help (see a summary with '-h')
```

Notable extras beyond stock Scoop parity:

- `bagger install <manifest-url|path.json>` installs isolated packages without a bucket
- `bagger install/upgrade/uninstall -g/--global` targets `%ProgramData%\scoop` (admin)
- `bagger install/upgrade/download --arch <32bit|64bit|arm64>` (or `SCOOP_ARCH`) overrides architecture resolution
- `bagger autofetch <app>|all [-w]` previews (and writes) autoupdate URL/hash refreshes
- `bagger checkver` supports `regex` (+`reverse`/`replace`), `jsonpath`, `xpath`, `script`, arch-specific specs and the `github` shorthand
- `bagger virustotal` looks up cached downloads against the VirusTotal v3 API
- Downloads use `aria2c` when `aria2-enabled` is set (curl fallback); manifests can be SQLite-cached via `use_sqlite_cache`
- `BAGGER_FLAVOR=heavy` swaps progress lines for industrial ones
  (`Surveying the pit...`, `Hauling...`, `Assaying the ore...`,
  `Nothing to haul — already stockpiled.`). Off by default so scripts
  keep parsing the plain output.

See [SCOOP_FEATURES.md](SCOOP_FEATURES.md) for the full Scoop-vs-bagger feature comparison.

## Configuration

Bagger reads the same `config.json` keys that Scoop uses, so a pre-existing Scoop
configuration is picked up automatically. Run `bagger config` to list current values or
`bagger config set <key> <value>` to change one.

```sh
# Example: enable aria2 for faster downloads
bagger config set aria2-enabled true
```

Notable keys (full list in [SCOOP_FEATURES.md](SCOOP_FEATURES.md)):

| Key | Effect |
| :--- | :--- |
| `use_sqlite_cache` | Cache parsed bucket manifests in SQLite (restart faster, less disk I/O) |
| `show_manifest` | Echo each manifest JSON to the terminal before an install/upgrade confirmation prompt |
| `ignore_running_processes` | Abort install/upgrade/uninstall when an app's process is still running (otherwise prompt) |
| `aria2-*` | Enable & tune Aria2 (falls back to libcurl when missing) |
| `private_hosts` / `gh_token` | Authenticate against private GitHub repos and enterprise hosts |
| `alias` | JSON map of `bagger <name>` → real command (managed via `bagger alias`) |

## Development

```sh
# run the CLI
cargo run -- help
# run the test suite
cargo test --workspace
# lint
cargo clippy --all-targets
# release build (8.5 MB, fat-LTO, panic=abort)
cargo build --release
```

## License

**bagger** © [Vincent Mathis](https://github.com/vincentmathis). Released under the [Apache-2.0](LICENSE) license.
For licenses of sub crates, see [COPYING](COPYING).

[license-badge]: https://img.shields.io/github/license/vincentmathis/bagger
