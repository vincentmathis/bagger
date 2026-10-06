# Scoop Features Comparison: bagger vs Original Scoop

This document compares the features and commands of [Scoop](https://github.com/ScoopInstaller/Scoop) (the original PowerShell-based package manager) against the Rust reimplementation **`bagger`** and its library **`scoop-rs`**.

Legend:
- [x] Implemented
- [~] Partially/Experimental
- [ ] Not implemented

---

## Commands

### Core Commands

| Command | Status | Notes |
| :--- | :---: | :--- |
| `bagger install <app>` | [x] | Full install with deps, shims, shortcuts, persist, env vars; also accepts manifest URLs and local `.json` files (isolated); `-g/--global` installs for all users (admin); `--arch` overrides target arch |
| `bagger uninstall <app>` | [x] | Supports `-p` (purge), cascade removal, `-g/--global` |
| `bagger update` | [x] | Pull all subscribed buckets (no args; app upgrades live in `upgrade`; per-bucket update intentionally absent, matching upstream Scoop) |
| `bagger upgrade` | [x] | Upgrade all installed apps (or named ones); `-g/--global` for global scope; `--arch` override |
| `bagger search <query>` | [x] | Search across all buckets |
| `bagger list` | [x] | List installed apps; supports `--upgradable` |
| `bagger info <app>` | [x] | Show manifest info for any app |
| `bagger cat <app>` | [x] | Show manifest JSON |
| `bagger hold <app>` | [x] | Mark apps as held |
| `bagger unhold <app>` | [x] | Remove hold from apps |
| `bagger prefix <app>` | [x] | Show installation path for an app |
| `bagger which <command>` | [x] | Find which app owns an executable |
| `bagger status` | [x] | Show held, upgradable, and running apps |
| `bagger checkup` | [x] | Check for updates, report held/running/upgradable apps |
| `bagger reset <app>` | [x] | Reset an installed package to a specific version or re-extract |

### Bucket Commands

| Command | Status | Notes |
| :--- | :---: | :--- |
| `bagger bucket add <name> [url]` | [x] | Add bucket from built-in list or custom URL (git clone) |
| `bagger bucket remove <name>` | [x] | Remove bucket |
| `bagger bucket list` | [x] | List added buckets (manifest counts, sources) |

### Cache Commands

| Command | Status | Notes |
| :--- | :---: | :--- |
| `bagger cache list [query]` | [x] | List cached downloads |
| `bagger cache remove <query>` | [x] | Remove cached files by name/version; `--all` clears everything |

### Config & Utility Commands

| Command | Status | Notes |
| :--- | :---: | :--- |
| `bagger config [set] <key> [value]` | [x] | Get/set Scoop config.json |
| `bagger cleanup <app>` | [x] | Remove old versions, keep current |
| `bagger home <app>` | [x] | Open app homepage in browser |
| `bagger shim` | [x] | List, add, and remove shims |
| `bagger alias` | [x] | Manage command aliases (list, add, rm) |
| `bagger import` | [x] | Import apps from export file/stdin and install them (restores holds, skips isolated) |
| `bagger export` | [x] | Export installed apps to JSON |
| `bagger help` | [x] | Auto-generated via clap |

### Download & Verify Commands

| Command | Status | Notes |
| :--- | :---: | :--- |
| `bagger create` | [x] | Create a bucket or manifest template |
| `bagger depends <app>` | [x] | Show dependencies and reverse dependencies |
| `bagger download <app>` | [x] | Download package without installing; `--arch` override |
| `bagger checkver <app>` | [x] | Check latest version from app manifest URLs |
| `bagger checkver all` | [x] | Check all installed apps for updates |
| `bagger autofetch <app> [all] [-w]` | [x] | Checkver + autoupdate URL expansion preview; `-w` rewrites version, URLs and download-mode hashes |

### Security Commands

| Command | Status | Notes |
| :--- | :---: | :--- |
| `bagger virustotal <app>` | [x] | SHA256 + VirusTotal v3 file-report lookup (flags malicious/suspicious counts) |

---

## Manifest Field Support

Manifest fields from the [Scoop schema](https://github.com/ScoopInstaller/Scoop/blob/master/schema.json).

### Top-Level Fields

| Field | Status | Notes |
| :--- | :---: | :--- |
| `version` | [x] | |
| `description` | [x] | |
| `homepage` | [x] | |
| `license` | [x] | Supports SPDX identifier + URL |
| `depends` | [x] | Recursive dependency resolution implemented |
| `innosetup` | [x] | |
| `cookie` | [x] | Sent as `Cookie` header by both curl and aria2 download backends |
| `notes` | [x] | Shown after install via `PackageInstalledNotes` event |
| `suggest` | [x] | Shown after install alongside notes |

### Architecture Fields

| Field | Status | Notes |
| :--- | :---: | :--- |
| `architecture.32bit` | [x] | |
| `architecture.64bit` | [x] | Default selection |
| `architecture.arm64` | [x] | |
| `architecture.*.bin` | [x] | Per-arch shim definitions |
| `architecture.*.url` | [x] | |
| `architecture.*.hash` | [x] | |
| `architecture.*.env_add_path` | [x] | |
| `architecture.*.env_set` | [x] | |
| `architecture.*.shortcuts` | [x] | |
| `architecture.*.persist` | [x] | |
| `architecture.*.pre_install` | [x] | PowerShell script |
| `architecture.*.post_install` | [x] | PowerShell script |
| `architecture.*.pre_uninstall` | [x] | PowerShell script |
| `architecture.*.post_uninstall` | [x] | PowerShell script |
| `architecture.*.installer` | [x] | Custom installer with script/args |
| `architecture.*.uninstaller` | [x] | Custom uninstaller with script/args |
| `architecture.*.extract_dir` | [x] | |
| `architecture.*.checkver` | [x] | Used as fallback via `effective_checkver` |

### Top-Level (noarch) Fields

| Field | Status | Notes |
| :--- | :---: | :--- |
| `url` | [x] | Single URL or array |
| `hash` | [x] | md5, sha1, sha256, sha512 |
| `extract_dir` | [x] | |
| `extract_to` | [x] | Move extracted content to subdirectory |
| `bin` | [x] | Shim generation for binaries |
| `env_add_path` | [x] | PATH management |
| `env_set` | [x] | Environment variable management |
| `shortcuts` | [x] | Desktop/taskbar shortcuts |
| `persist` | [x] | Persist files across reinstalls |
| `psmodule` | [x] | PowerShell module import |
| `pre_install` | [x] | PowerShell script before extraction |
| `post_install` | [x] | PowerShell script after install |
| `pre_uninstall` | [x] | PowerShell script before uninstall |
| `post_uninstall` | [x] | PowerShell script after uninstall |
| `installer` | [x] | Custom installer (file-based or script) |
| `uninstaller` | [x] | Custom uninstaller (file-based or script) |

### Advanced Fields

| Field | Status | Notes |
| :--- | :---: | :--- |
| `checkver` | [x] | String form, `github` shorthand (`releases/latest` + tag regex), `regex` (+`reverse`, `replace` w/ captures)/`jsonpath`/`xpath`/`script` evaluated; `jsonpath`/`xpath` extract the string `regex` matches; `useragent` honored (else session UA) |
| `autoupdate` | [x] | URL templates expanded by autofetch; `--write` resolves `download`/`extract`/`json`/`xpath` hashes (others fall back to downloading the asset, like upstream) |
| `runtime` | [x] | Resolved like `depends`; shown in `info`/`depends` |

### Hash Extraction (`autoupdate.hash`)

| Field | Status | Notes |
| :--- | :---: | :--- |
| `regex` | [~] | Parsed; auto-applied in checkver + extract-mode (`$md5…`/`$checksum` placeholders), not for other hash modes |
| `jsonpath` | [x] | Evaluated by checkver (`$.a.b[0]` subset) and json-mode hashes |
| `xpath` | [x] | Evaluated by checkver (`tag`, `*`, `[n]`, `[@a='v']`, trailing `text()`/`@attr`) and xpath-mode hashes |
| `url` | [x] | Hash-document URL for extract/json/xpath modes |
| `find` | [x] | Alias for `regex`, honored in extract mode |
| `type` | [~] | Deprecated field, parsed |

---

## Sync Options (Install/Update/Uninstall)

| Option | Status | Notes |
| :--- | :---: | :--- |
| `AssumeYes` (`-y`) | [x] | Skip all prompts |
| `DownloadOnly` | [x] | Download but don't install |
| `EscapeHold` | [x] | Force operations on held packages |
| `IgnoreCache` | [x] | Ignore local cache, re-download |
| `IgnoreFailure` | [x] | Continue transaction on failure |
| `NoDependencies` | [x] | Skip dependency resolution |
| `NoHashCheck` | [x] | Skip integrity verification |
| `NoUpgrade` | [x] | Don't upgrade existing packages |
| `NoReplace` | [x] | Don't replace from different bucket |
| `Offline` | [x] | Use only local cache |
| `OnlyUpgrade` | [x] | Only upgrade, don't install new |
| `Remove` | [x] | Uninstall mode |
| `Purge` | [x] | Remove persist data on uninstall |
| `Cascade` | [x] | Remove dependencies recursively |
| `NoDependentCheck` | [x] | Skip reverse-dependency check |

---

## Config Options

| Key | Status | Notes |
| :--- | :---: | :--- |
| `root_path` | [x] | Scoop root directory |
| `cache_path` | [x] | Cache directory |
| `global_path` | [x] | Global apps directory |
| `proxy` | [x] | HTTP proxy support |
| `no_junction` | [x] | Disable symlink/junction for `current` |
| `use_isolated_path` | [x] | Isolated PATH management |
| `last_update` | [x] | Bucket update timestamp |
| `aria2-*` settings | [x] | Parsed with full config getters |
| `use_external_7zip` | [x] | Parsed; extraction always shells out to 7z (PATH or Scoop 7zip app) |
| `scoop_branch` | [~] | Parsed (not actively used) |
| `scoop_repo` | [~] | Parsed (not actively used) |
| `gh_token` | [x] | Parsed for GitHub private repos |
| `private_hosts` | [x] | Parsed for auth headers |
| `alias` | [x] | Managed via `bagger alias` (list/add/rm) + `config` |
| `use_sqlite_cache` | [x] | **Implemented** - bucket manifests cached in `<cache>/manifests.db`, invalidated by mtime+size |
| `show_manifest` | [x] | **Implemented** - Shows manifest JSON in install/upgrade confirmation |
| `ignore_running_processes` | [x] | **Implemented** - install/upgrade/uninstall abort on running processes unless enabled |
| `shim` | [~] | Parsed; no effect (script-based shims don't consume shim.exe variants) |
| `aria2_warning_enabled` | [x] | **Implemented** - Config getter and setter added |

---

## Other Features

| Feature | Status | Notes |
| :--- | :---: | :--- |
| Built-in buckets (main, extras, games, java, php, versions, nonportable) | [x] | All 7 built-in buckets defined |
| Bucket priority ordering | [x] | Defined in `BUCKET_PRIORITY` |
| Archive extraction (7z, zip, tar, gz, bz2, xz, zst, rar, etc.) | [x] | via external 7z executable |
| Archive auto-detection | [x] | Extension-based detection |
| Self-extracting 7z (SFX) detection | [x] | PE header detection |
| PowerShell script execution | [x] | `powershell.exe` with Scoop context vars |
| Shim creation | [x] | Batch, EXE, PowerShell, Python, Java, Bash |
| Desktop shortcuts | [x] | |
| Taskbar pinning | [x] | |
| Persist directories | [x] | Symlink-based persistence |
| Environment variables (PATH, custom) | [x] | Isolated path support |
| PowerShell module import | [x] | |
| Hash verification (MD5, SHA1, SHA256, SHA512) | [x] | via bagger-hash |
| Download progress tracking | [x] | |
| Parallel bucket updates | [x] | ThreadPool-based |
| Transactional install/uninstall | [x] | Emit events, rollback support |
| Held package protection | [x] | Packages are skipped unless `--force` |
| Multiple architecture support | [x] | 32bit, 64bit, arm64; `--arch` flag / `SCOOP_ARCH` env override resolution + install record |
| Cross-bucket package replacement | [x] | When package moves buckets |
| Candidate selection (multi-bucket) | [x] | Interactive or auto-select |
| Global app installation | [x] | **Implemented** - `-g/--global` on install/uninstall/upgrade scopes the session root to `global_path` (admin-gated); shims/shortcuts/persist/env follow automatically; cache stays user-scoped |
| Isolated app installation | [x] | **Implemented** - `bagger install <manifest-url|path.json>` installs without a bucket (deps resolve from buckets, `__isolated__` marker in install.json) |
| Aria2 integration | [x] | **Implemented** - `aria2c` used when `aria2-enabled` + binary on PATH (split/connections/cookie/proxy/extra-opts honored), curl fallback + optional missing-binary warning |
| SQLite manifest cache | [x] | **Implemented** - opt-in via `use_sqlite_cache`; raw JSON keyed by (bucket, name) with mtime+size invalidation, shared across query threads |
| Manifest auto-review prompt | [x] | **Implemented** - `show_manifest` displays manifests before install/upgrade |
| VirusTotal integration | [x] | **Implemented** - real v3 file-report lookup by SHA256 |
| App execution during install | [x] | Pre/post install/uninstall + installer/uninstaller scripts run via PowerShell |
| Custom installer/uninstaller | [x] | Supports script or file-based installers |
| Aria2 warning suppression | [x] | **Implemented** - `aria2_warning_enabled` config getter and setter |
| Ignore running processes | [x] | **Implemented** - guard in sync install/remove, bypass via config |

---

## Missing Commands (Medium-High Priority)

1. **`scoop/virustotal`** - ✅ Implemented (real v3 file-report lookup by SHA256)
2. **`scoop/checkver`** - ✅ Implemented (fetch URL, apply regex, compare versions)
3. **`scoop/autofetch`** - ✅ Implemented (checkver + autoupdate `$version` URL expansion, `--write` bumps manifests)

## Previously Missing - Now Implemented

1. **`scoop/which`** - Find which app provides a given command - ✅ Implemented
2. **`scoop/prefix`** - Print the install path of an app - ✅ Implemented
3. **`scoop/status`** - Show app status, running processes, held apps - ✅ Implemented
4. **`scoop/checkup`** - Check for updates and report issues - ✅ Implemented
5. **`scoop/reset`** - Reset app to a previous version - ✅ Implemented
6. **`scoop/import` / `scoop/export`** - Export/import installed app list - ✅ Implemented
7. **`scoop/alias`** - Manage command aliases - ✅ Implemented
8. **`scoop/shim`** - List, add, and remove shims - ✅ Implemented
9. **`scoop/create`** - Create a new bucket or manifest template - ✅ Implemented
10. **`scoop/depends`** - Show dependencies and reverse dependencies - ✅ Implemented
11. **`scoop/download`** - Download a package without installing - ✅ Implemented

## Missing Features (Medium Priority)

- (none — all tracked features implemented; `--global` covered install/uninstall/upgrade/cleanup)

## Previously Missing - Now Implemented (this iteration)

7. **Runtime dependencies** - `runtime` manifest field parsed, resolved like `depends`, shown in `info`/`depends`
8. **Ignore running processes** - sync install/remove abort with `PackageRunningProcesses` error unless `ignore_running_processes` is set
9. **VirusTotal binary scan** - real SHA256 + VT v3 `GET /files/{sha256}` verdicts (malicious/suspicious/harmless/undetected)
10. **Autofetch** - `bagger autofetch <app>|all [--write]` previews expanded autoupdate URLs + hash modes
11. **Aria2 download manager** - downloads delegated to `aria2c` when `aria2-enabled` is set and the binary is on PATH (honors split/max-connection/min-split/retry-wait/cookie/proxy/extra options); warns when enabled-but-missing if `aria2-warning-enabled`, otherwise curl backend is used
12. **Isolated package installs** - `bagger install <url|path.json>` fetches/parses the manifest, derives the app name from the basename, resolves deps from buckets, and records no bucket in install.json (`__isolated__` marker)
13. **SQLite manifest caching** - `use_sqlite_cache` caches bucket manifest JSON in `<cache>/manifests.db` keyed by (bucket, name) with mtime+size invalidation; failures fall back to disk parsing
14. **Global app installs** - `-g/--global` on install/uninstall/upgrade (plus existing cleanup support) scopes the session root to `global_path` via a runtime-only config override; admin rights required
15. **Install notes & suggestions** - `notes`/`suggest` shown after install via `PackageInstalledNotes`; added missing `notes()` getter
16. **checkver jsonpath + arch fallback** - `effective_checkver()` prefers top-level, falls back to `architecture.<arch>.checkver`; `jsonpath` (`$.a.b[0]` subset) evaluated when no `regex` matches
17. **checkver script support** - `checkver.script` runs via `invoke_script_capture` in the manifest dir context; stdout is the version (optionally filtered by `regex`)
18. **checkver xpath support** - `roxmltree`-backed subset evaluator (`/`, `//`, `[n]`, `[@attr='value']`, `text()`, `@attr`) with quote-aware step splitting
19. **Architecture override** - `scoop_rs::arch` module (`set_override`, process-wide); `arch_specific_field!` dispatches on it; `--arch` on install/upgrade/download plus `SCOOP_ARCH` env; `install.json` records the resolved arch
20. **checkver reverse/replace/captures** - `reverse` matches last occurrence; `replace` expands `$1`/`${name}` from captures (.NET-style, `$$` escape); `jsonpath`/`xpath` extract the string `regex` matches; captures feed autofetch `$match*` URL variables
21. **Download unknown-size fix** - unknown remote sizes no longer validate missing cache (`0==0` bug) nor skip downloads; found by first real end-to-end install
22. **Shim add/remove symmetry** - `add()` now creates exactly what `remove()` cleans (`{name}.exe`/`.cmd`/`.ps1`/bare); per-file content (ps1 vs cmd wrappers); `which` resolves through `current` junctions; stray `shims/exe` + unleaked `.bat` extras gone
23. **checkver useragent + quiet shim removal** - `checkver.useragent` sent (session UA fallback); removal progress only announced for files that exist
24. **autoupdate hash rewriting** - `autofetch -w` downloads expanded URLs, computes sha256 and rewrites `url`/`hash` (shape-preserving, non-download modes skipped); verified end-to-end offline plus unit test
25. **autoupdate extract/json/xpath hash modes** - upstream-faithful: textfile search with `$sha256…` placeholders, JSON/XPath hash documents, length-inferred `format_hash`, full `$version`/`$match<TitleCase>`/`$basename` substitutions, download fallback; verified offline e2e
26. **Schema-compat hardening** - verified string-form `checkver`/`sourceforge` and `github` shorthand parsing against the upstream schema; added missing `github` hash mode; live-verified `github:` checkver against api real releases page
27. **Real installing import** - `import [FILE] [-y]` installs exported apps through the install flow (was list-only) and restores holds; verified offline roundtrip incl. hold state
28. **Cold-path fixes from execution** - `download --help` no longer panics (`-v` collided with global verbose; now long-only `--version`); `reset` repoints `current` via junction-aware removal (was os error 183); verified by reset/cleanup e2e across two versions

## Notes

- The core install/uninstall/upgrade flow is fully implemented with commit logic supporting archives, shims, shortcuts, persist, psmodule, env, and scripts.
- Verified by offline end-to-end runs against scratch roots: isolated `file://` install, bucket install (arch default), 1.0→2.0 upgrade, `--arch 32bit` + `SCOOP_ARCH` selection, invalid-arch rejection, `list`/`prefix`/`which`/`shim ls`/`status`/`checkup`, and clean uninstalls with no leftovers.
- PowerShell script invocation is functional but requires `powershell.exe` (Windows-only).
- Archive extraction depends on an external `7z` executable.
- The library (`scoop-rs`) is API-complete for manifests but some options are parsed-but-not-active.
- `bagger-hash` is a new crate providing MD5/SHA1/SHA256/SHA512 hashing.
