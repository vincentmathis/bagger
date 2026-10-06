# bagger

> Bagger is a CLI implementation of [Scoop](https://scoop.sh/) in Rust

[![crate](https://img.shields.io/crates/v/bagger)](https://crates.io/crates/bagger)
[![license][license-badge]](LICENSE)

## Install

### From source

```sh
git clone https://github.com/vincentmathis/bagger
cd bagger
cargo build --release
# the binary lives at target/release/bagger.exe
```

### One-liner install script (Windows, requires admin for system-wide)

```ps1
# User scope (installs to $Env:LOCALAPPDATA\bagger):
iwr -useb https://raw.githubusercontent.com/vincentmathis/bagger/feat/install-commit/scripts/install.ps1 | iex

# System-wide (installs to $Env:ProgramFiles\bagger, requires admin):
iex (iwr -useb https://raw.githubusercontent.com/vincentmathis/bagger/feat/install-commit/scripts/install.ps1).Content -args -System
```

🚧 **Stability caveat**: `bagger` is on a pre-1.0 track (`0.1.0-beta.8`); while the core
Scoop command surface is fully implemented, individual flag behaviour may differ slightly from upstream
Scoop. Pinned releases and checksums are available on the [GitHub releases page](https://github.com/vincentmathis/bagger/releases).

**Prerequisites** (Windows-only): PowerShell 5+, an internet connection on first install.

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
# release build (8 MB, fat-LTO, panic=abort)
cargo build --release
```

## License

**bagger** © [Vincent Mathis](https://github.com/vincentmathis). Released under the [Apache-2.0](LICENSE) license.
For licenses of sub crates, see [COPYING](COPYING).

[license-badge]: https://img.shields.io/github/license/vincentmathis/bagger
