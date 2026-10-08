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
| `bagger install <app>` | [x] | Full install with deps, shims, shortcuts, persist, env vars (upgrades outdated apps unless `-U`; skips with a warning when already current); also accepts manifest URLs and local `.json` files (isolated); `app@version` / `bucket/app@version` / `url@version` / `path.json@version` pins (history → autoupdate generation); `-g/--global` installs for all users (admin); `--arch` overrides target arch |
| `bagger uninstall <app>` | [x] | Supports `-p` (purge), cascade removal, `-g/--global` |
| `bagger update` | [x] | Pull all subscribed buckets (no args); shows pulled commit logs unless `show_update_log` is false; stamps `last_update`; named apps forward to `upgrade` (like upstream) |

> **Note on `update *`:** In older Scoop, `scoop update *` meant "upgrade all apps". In modern Scoop (and in `bagger`), use `bagger upgrade` (with no arguments) to upgrade all installed apps. The `update` command only updates bucket manifests and does not accept app names or wildcards.
| `bagger upgrade` | [x] | Upgrade all installed apps (or named ones); `-g/--global` for global scope; `--arch` override; `--force` reinstalls current versions (previous dir rotated to `_<version>.old`), also via `force_update` config; reports when everything is already current |
| `bagger search <query>` | [x] | Search across all buckets |
| `bagger list` | [x] | List installed apps; supports `--upgradable` (incl. stale nightlies) |
| `bagger info <app>` | [x] | Show manifest info for any app |
| `bagger cat <app>` | [x] | Show manifest JSON |
| `bagger hold <app>` | [x] | Mark apps as held |
| `bagger unhold <app>` | [x] | Remove hold from apps |
| `bagger prefix <app>` | [x] | Show installation path for an app |
| `bagger which <command>` | [x] | Find which app owns an executable |
| `bagger status` | [x] | Show held, upgradable (incl. stale nightlies), and running apps |
| `bagger checkup` | [x] | Check for updates, report held/running/upgradable apps; system diagnostics (Defender exclusion, main bucket, long paths, developer mode) |
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
| `bagger alias` | [x] | Manage command aliases (list, add, rm; persisted to config) |
| `bagger import` | [x] | Import apps from export file/stdin and install them (restores holds, skips isolated) |
| `bagger export` | [x] | Export installed apps to JSON (incl. architecture + held) |
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
| `installer` | [x] | Custom installer: `file` and/or `args` (args alone runs the download, upstream `coalesce`); `is_in_dir` containment abort; `.ps1` retained, binaries removed unless `keep` |
| `uninstaller` | [x] | Custom uninstaller: same `file`/`args`/`keep`/`script` semantics as the installer side |

### Advanced Fields

| Field | Status | Notes |
| :--- | :---: | :--- |
| `checkver` | [x] | String form, `github` shorthand (`releases/latest` + tag regex), `regex` (+`reverse`, `replace` w/ captures)/`jsonpath`/`xpath`/`script` evaluated; `jsonpath`/`xpath` extract the string `regex` matches; `useragent` honored (else session UA) |
| `autoupdate` | [x] | URL templates expanded by autofetch; `--write` resolves `download`/`extract`/`json`/`xpath`/`rdf`/`fosshub`/`sourceforge`/`github` hashes (`metalink` + failures fall back to downloading the asset, like upstream) |
| `runtime` | [x] | Resolved like `depends`; shown in `info`/`depends` |

### Hash Extraction (`autoupdate.hash`)

| Field | Status | Notes |
| :--- | :---: | :--- |
| `regex` | [x] | **Implemented** - `regex` wins over the `find` alias (was inverted); `$base64` placeholder + base64→hex conversion; metalink `<hash>` fallback; verified live against an Electron-style `latest.yml` (`sha512: <base64>`) end-to-end |
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
| `aria2-*` settings | [x] | Parsed with full config getters; `aria2-enabled` defaults on like upstream (curl fallback on missing binary or failed download) |
| `use_external_7zip` | [x] | Parsed; extraction always shells out to 7z (PATH or Scoop 7zip app) |
| `scoop_branch` | [~] | Parsed (not actively used) |
| `scoop_repo` | [~] | Parsed (not actively used) |
| `gh_token` | [x] | GitHub API auth (`SCOOP_GH_TOKEN` env > config > `GH_TOKEN` > `GITHUB_TOKEN`), sent as `Bearer` with API-version pin; auth/rate-limit hints on failure |
| `private_hosts` | [x] | Per-host regex `match` + `Name=Value`/`Name: Value` headers sent on curl downloads |
| `force_update` | [x] | `upgrade` behaves as `--force` when true |
| `show_update_log` | [x] | Bucket commit logs on update (default shown, like upstream) |
| `last_update` | [x] | Stamped after every bucket update; readable via `config list` |
| `debug` | [x] | Raises the log floor to `DEBUG` (unless `-v` already set higher) |
| `default_architecture` | [x] | Persistent install-arch default (`--arch` > `SCOOP_ARCH` > config); previously parsed under a misspelled key and inert |
| `aria2-fallback-enabled` | [x] | Failed aria2c downloads retry via curl (default on, like upstream); exit-code meanings reported |
| `use_git_history` | [x] | Pinned `@version` manifests are searched in bucket git history first (default on, like upstream) |
| `update_nightly` | [x] | Nightly apps redated on `upgrade` when a new day dawned (otherwise only `--force`); nightly installs stamp `nightly-yyyyMMdd` |
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
| Candidate selection (multi-bucket) | [x] | Interactive prompt, or `BUCKET_PRIORITY` auto-select under `-y`/no-TTY |
| Global app installation | [x] | **Implemented** - `-g/--global` on install/uninstall/upgrade scopes the session root to `global_path` (admin-gated; gate verified live, routing unit-tested); shims/shortcuts/persist/env follow automatically; cache stays user-scoped |
| Isolated app installation | [x] | **Implemented** - `bagger install <manifest-url|path.json>` installs without a bucket (deps resolve from buckets, `__isolated__` marker in install.json) |
| Aria2 integration | [x] | **Implemented** - `aria2c` used when `aria2-enabled` + binary on PATH (split/connections/cookie/proxy/extra-opts honored), curl fallback + optional missing-binary warning |
| SQLite manifest cache | [x] | **Implemented** - opt-in via `use_sqlite_cache`; raw JSON keyed by (bucket, name) with mtime+size invalidation, shared across query threads |
| Manifest auto-review prompt | [x] | **Implemented** - `show_manifest` displays manifests before install/upgrade |
| VirusTotal integration | [x] | **Implemented** - real v3 file-report lookup by SHA256 |
| App execution during install | [x] | Pre/post install/uninstall + installer/uninstaller scripts run via PowerShell |
| Custom installer/uninstaller | [x] | Supports script or file-based installers |
| Aria2 warning suppression | [x] | **Implemented** - `aria2_warning_enabled` config getter and setter |
| Ignore running processes | [x] | **Implemented** - guard in sync install/remove, bypass via config |
| Heavy-machinery CLI flavor | [x] | **Implemented** - `BAGGER_FLAVOR=heavy` swaps progress lines (`Surveying the pit…`, `Hauling…`, `Assaying the ore…`); default output stays script-compatible |

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

- (none — all tracked features implemented)
- **Known divergences (deliberate)** - `update -f` spelling is `upgrade --force`; missing shim targets warn instead of aborting the install; `install` upgrades outdated apps unless `-U/--no-upgrade` (upstream warns and skips; bagger reports skips only when already current)

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
26. **autoupdate site hash modes** - `fosshub`/`sourceforge`/`github` auto-detected from asset URLs (page/API scraping as upstream) plus `rdf` digest docs; `metalink` documented as download-fallback; live-verified sourceforge against the real 7-Zip files page
27. **Schema-compat hardening** - verified string-form `checkver`/`sourceforge` and `github` shorthand parsing against the upstream schema; added missing `github` hash mode; live-verified `github:` checkver against api real releases page
28. **Real installing import** - `import [FILE] [-y]` installs exported apps through the install flow (was list-only) and restores holds; verified offline roundtrip incl. hold state
29. **Cold-path fixes from execution** - `download --help` no longer panics (`-v` collided with global verbose; now long-only `--version`); `reset` repoints `current` via junction-aware removal (was os error 183); verified by reset/cleanup e2e across two versions
30. **Archive/script/persist/broken-shim verification** - 7z extraction (incl. subdirs), `post_install` markers and persist symlinks all behave Scoop-true; `checkup` now detects broken content shims via `shim::target_of` (old check only saw dangling symlinks)
31. **Negative-path hardening** - exit codes verified (1 on error, 0 on success); offline cache-miss now reports the missing cache file via `InvalidCacheFile` instead of raw os error 2
32. **Installer-hook verification + README rebrand** - `pre_install`/`installer.script`/`pre_uninstall`/`uninstaller.script` all execute in order (verified offline); release profile builds (8.1MB, fat LTO); README rewritten for bagger with the real 31-command list
33. **Alias persistence + shim rm alias** - `config set alias` accepts the JSON alias map (alias add/rm were broken end-to-end); `shim rm` alias added; `create` builds the real `bucket/` layout; verified alias/config/shim/create/completions/virustotal-no-key behavior live
34. **Export/import architecture** - export records installed arch; import groups by arch with per-group resolution; caught + fixed a `query_installed` regression (a clippy refactor had moved `return Some` inside the upgradable-only branch, emptying all installed queries)
35. **Scoop hook-scope prelude** - every manifest script now runs with upstream's hook variables (`$dir`, `$version`, `$architecture`, `$app`, `$bucket`, `$bucketsdir`, `$fname`, `$global` as real bool, `$original_dir`, `$persist_dir`, `$cmd`) plus faithful ports of `Expand-7zipArchive`/`Expand-MsiArchive`/`Expand-InnoArchive`/`Expand-DarkArchive`/`Expand-ZipArchive`, `Get-HelperPath`, `Invoke-ExternalCommand`, `movedir`, `Add-Path`/`Remove-Path`, `Get/Set-EnvVar`, and msg helpers; fixed `$global` env (was wrongly derived from `use_isolated_path`) and added `SCOOP_GLOBAL` env
36. **Upstream install ordering** - `pre_install` now runs after extraction (was before), so `Expand-7zipArchive "$dir\inner.7z"` sees the extracted tree; uninstall hooks run in the real version dir instead of the `current` junction
37. **`installer.file`/`uninstaller.file` execution** - file-based installers run with `$dir`/`$global`/`$version` arg substitution (`.ps1` via hook scope, else direct execution), removed unless `keep`; 32 in-the-wild manifests (mostly nonportable) were silently skipped before
38. **Native MSI extraction + `extract_dir` promote** - `.msi` goes through lessmsi/msiexec (7z mangles MSI-internal names into `_`-prefixed garbage, and the old code never extracted MSIs at all — every MSI-backed install died with os error 5); `extract_dir` now promotes the named subdir's contents up (was inverted: moved the tree into its own descendant and crashed); proven by real `7zip` + `obsidian` installs
39. **Running-process skip (bulk) + persist upgrade parity** - bulk install/upgrade/uninstall now SKIPS apps with running processes (with a `Skipping 'x' (running: …)` message) instead of aborting the whole transaction; explicitly requested singles still fail loudly; re-checked at commit time to close the download window; `persist::link` mirrors upstream (`store wins, fresh content stashed as .original`) instead of renaming over existing data (os error 5 on every persist-app upgrade); proven live (`7zip`/`mediainfo-gui` upgrades that crashed before) plus real-spawn unit test
40. **Shim `.exe` poisoning fix + `shim refresh`** - bagger wrote batch content into `{name}.exe`, which cannot execute (os error 216) and shadows the working wrapper in `PATH` resolution (77 apps affected on one real root); `Exe` shims now write only `{name}.cmd`, `add()` removes poison (never native binaries), `remove()` cleans the new set, and `bagger shim refresh` repairs all installed apps
41. **Shim argument forwarding + 7z resolution hardening** - all shim flavors now forward caller args (`%*`/`"$@"`/`@args`; `7z l file` ran bare `7z` before), and 7z lookup probes `7z.cmd` plus the session root's `7zip` app (previously `PATH`-only, blind after the poisoning cleanup)
42. **Cleanup resilience + persist unlink hardening** - `cleanup` no longer aborts the whole run on one locked old version (reports `Couldn't remove x (in use by: …; quit them and retry)` and continues); persist-link removal goes through the hardened helper because raw `remove_dir` fails on junctions with os error 5; proven live on a running-from-old-version FanControl
43. **Real-name download staging** - downloads are staged into the app dir under upstream `url_filename` semantics (leaf after last slash, `#/...` coerces: `Obsidian-….exe#/dl.7z` stages as `dl.7z`) instead of cache hash names, so hook scripts, `installer.file` paths, `$fname`, and shim targets observe the names manifests were written against; `extract_to` no longer swallows unextractable app binaries (oh-my-posh's exe); proven by reinstalling hyperfine/delta/oh-my-posh/zoxide live
44. **Conservative cleanup + missing-target warnings** - `cleanup` never deletes a version dir lacking install metadata (a failed upgrade's manifest-less dir plus dangling `current` is a pending upgrade, not garbage); `shim add` warns loudly when a target was never materialized instead of succeeding silently
45. **`current`-first active version + shim target parsing** - `cleanup` resolves the `current` link target as sacred before consulting manifests (the manifest fallback could anoint the wrong version and delete the live one); `target_of` resolves relative alias targets against the shims dir and prefers sh-style `#`/`@rem` comment targets over `"$(wslpath…)"` wrappers (killed a dozen false broken-shim reports); `checkup` only assesses recognized shim files instead of flagging stray `.txt` junk
46. **Native Inno Setup extraction** - `.exe` files with `innosetup` go through innounp (`-x -d -c{app}`, mirroring `Expand-InnoArchive`) instead of being left raw with dangling shims; the dep was already auto-added but never executed; proven by reinstalling ollama-full live
47. **Installer/uninstaller execution parity** - `args` without `file` now runs the staged download (upstream `coalesce $installer.file $Name[0]`); installer paths resolving outside the app dir abort (`is_in_dir`); `.ps1` installers are retained (upstream only removes binaries) and `uninstaller.keep` is parsed; proven offline with an args-only batch installer/uninstaller plus traversal/missing negative cases
48. **Installer PATH scrub + scoped env backend** - `ensure_install_dir_not_in_path`: installer-added app dirs are removed from PATH after install (system PATH only earns a warning without admin, like upstream); env get/set honors global scope (HKLM) instead of always writing HKCU; every registry env change broadcasts `WM_SETTINGCHANGE` so new terminals see it; proven live (upstream's exact `Installer added '…' to path. Removing.` notice observed)
49. **Full-scope `env_set` expansion + env hardening** - values expand the whole hook scope (`$dir`/`$version`/`$app`/`$architecture`/`$global`/`$bucket`/`$bucketsdir`/`$fname`/`$original_dir`/`$persist_dir`, `$name`/`${name}` forms, `$env:*` with unknown→empty like `ExpandString`); current-process env is set/cleared too; `%`-values stored as `REG_EXPAND_SZ`; `env_add_path` entries escaping the app dir are dropped; removal scrubs both the default and isolated PATH targets and tolerates missing variables; proven live (`ARGAPP_HOME` resolved to the `current` link, `..` entry dropped, all traces removed on uninstall)
50. **Shim PATH fallback + global persist ACL** - bare `bin` targets missing from the app dir resolve through `PATH` (upstream `(Get-Command).Source`, incl. `PATHEXT` probing) instead of warning immediately; global installs with persist data grant `Users` write on the persist root (`persist_permission` via icacls, admin-gated); both unit-tested (SID-verified ACL, PATH-resolution cases)
51. **checkup system diagnostics** - Defender exclusion (service state + realtime flag + exclusion list, skipped when unreadable instead of false-alarming), `main` bucket presence, `LongPathsEnabled`, Developer Mode; each failure prints upstream's remediation; pure matchers unit-tested, live-verified (warned on missing exclusion, silent on healthy keys)
52. **Dead config keys wired** - `gh_token` (env-precedence resolution, `Bearer` auth on api.github.com with 401/rate-limit hints), `private_hosts` (regex match + `=`/`:` headers on curl downloads), `force_update` (implies `--force`), `show_update_log` (per-bucket commit logs via rev capture + `BucketUpdateLog` event), `last_update` (readable now), `debug` (log floor), `default_architecture` (lowest-precedence arch default; was parsed under a misspelled key), `aria2-fallback-enabled` (new, default on); `config set` accepts all of them with value validation
53. **Force reinstall** - `upgrade --force` (long-only; `-f` stays `--ignore-failure`) reinstalls current versions, rotating the old dir to `_<version>.old` (`(N)` sequence); confirm UI no longer panics on same-version entries; proven live (rotation chain `_.old`→`.old(1)`→…, installer rerun verified by marker resurrection, `force_update` config path too)
54. **aria2 failure fallback + misc upstream hints** - failed aria2c downloads warn with the exit-code meaning and retry via curl (structured `Error::Aria2`; stale `.aria2` control files cleaned); missing-binary warning is now visible instead of trace-only; SourceForge hash mismatches print upstream's retry hint; proven with a fake failing `aria2c.exe` (fallback success + disabled-abort unit tests)
55. **Failed-install purge + version validation + no-hash print** - version dirs without install metadata are purged before staging (upstream `ensure_none_failed`'s purge half), except the `current`-live directory (a pending upgrade, repaired in place); manifest versions outside `[\w.\-+_]` abort with the allowed set; hash-less manifests print the computed SHA256 in upstream's exact format; all proven live (stale-dir replacement, clean abort message, byte-identical warning)
56. **Download URL handling parity** - `Referer` set to the file's directory on downloads (never on size probes, never for sourceforge/portableapps — verified against a logging local server: probe had none, download had the directory); FossHub page URLs run the download-API handshake and `/download` SourceForge URLs reshape to the direct mirror form, with cache keys and staged names keeping the original URL; pure transforms unit-tested (live FossHub handshake unverifiable — the site is now JS-driven with no static `?dwl=` links left)
57. **`app@version` pins** - `split_version_query` only splits version-shaped suffixes (userinfo URLs safe); resolution order is current-manifest match → bucket git-history search (newest-first walk, capped, best-effort) → autoupdate expansion at the pinned version with hash resolution; generated manifests keep bucket attribution in `usermanifests/` so upgrades work; proven live (history 1.0 + generated 1.5 + HEAD 2.0 + qualified + ambiguous-bucket error + missing-version error, all with passing integrity)
58. **Nightly version stamping** - manifests with `version: nightly` install under `nightly-yyyyMMdd` (local date), saved self-describing; hash checks skipped for literal and dated forms; `upgrade` redates on a new day only with `update_nightly` (default off, like upstream), `--force` reinstalls any time with `_.old` rotation; same-day upgrade is a no-op; `cleanup` spares `_<v>.old*` backups (previously would have deleted them — found via upstream `Get-InstalledVersion`); proven live end to end
59. **GitHub private-release downloads** - `releases/download` URLs resolve to API asset URLs for private repos when a token is configured (repo check + tag asset pick, `token` scheme like upstream); `api.github.com` asset downloads carry `Accept: application/octet-stream` + `Bearer` (curl backend; aria2 failures fall back to curl); proven live against a public 93-byte asset (binary bytes, not JSON metadata); the private-repo branch is unit-tested (URL split, asset pick, header selection) but has no private repo to verify against
60. **Nightly status display** - `stale_nightlies()` (bucket manifest still `nightly`, installed stamp dated but not today, `update_nightly` set, holds excluded) feeds `status`, `list --upgradable`, and `checkup`, which show today's stamp as the target; proven live with a fabricated yesterday install (flagged in all three, silent by default, healed by `upgrade`); pins need no `update -f` equivalent — bucket attribution lets them upgrade normally (proven: `1.0` pin → plain `upgrade` → `2.0`)
61. **Upstream re-verification corrections** - reading `scoop-cleanup.ps1` showed `cleanup` deletes `_<version>.old*` backups (no exclusion like `Get-InstalledVersion`'s list filter), so the earlier exclusion was reverted — backups clean like any old version; `cleanup -k` now also drops `*.download` partials and stale `.aria2` control files (upstream drops the former); `aria2-enabled` flipped to default-on like upstream (safe under the curl fallback); `bagger update <app>` forwards to `upgrade` with a notice (upstream semantics) instead of erroring

## Build & Distribution

| Feature | Status | Notes |
| :--- | :---: | :--- |
| `cargo install bagger` | [x] | Install from crates.io (pre-releases are opt-in: `cargo install bagger --version 0.1.0-beta.10`) |
| `scripts/install.ps1` | [x] | One-liner PowerShell installer: `iwr -useb https://raw.githubusercontent.com/vincentmathis/bagger/main/scripts/install.ps1 \| iex` |
| GitHub Actions CI | [x] | Lint (fmt+clippy), test matrix (x64/i686/arm64), release build, auto-release via `release-please` |
| Release artifacts | [x] | `bagger-x86_64/i686/aarch64-pc-windows-msvc.zip` attached to GitHub releases |
| Scoop bucket | [~] | Published as `scripts/install.ps1` one-liner; dedicated `bagger` bucket (`scoop bucket add bagger …`) not yet created |
| winget package | [~] | Not yet published (would require community maintainer approval of a `vincentmathis.bagger` manifest) |

## Notes

- The core install/uninstall/upgrade flow is fully implemented with commit logic supporting archives, shims, shortcuts, persist, psmodule, env, and scripts.
- Verified by offline end-to-end runs against scratch roots: isolated `file://` install, bucket install (arch default), 1.0→2.0 upgrade, `--arch 32bit` + `SCOOP_ARCH` selection, invalid-arch rejection, `list`/`prefix`/`which`/`shim ls`/`status`/`checkup`, bucket add/list/update/remove + search/info/cat/depends, export/import roundtrips (incl. holds), sqlite cache creation + mtime invalidation, aria2-missing warning with curl fallback, download-only flow, running-process guard block + `ignore_running_processes` bypass, innosetup-implicit-`innounp` install, and clean uninstalls with no leftovers.
- 60 real Main-bucket manifests parse with zero failures; remaining schema shapes (`innosetup`, `cookie`) verified via synthetic manifests.
- PowerShell script invocation is functional but requires `powershell.exe` (Windows-only).
- Archive extraction depends on an external `7z` executable.
- The library (`scoop-rs`) is API-complete for manifests but some options are parsed-but-not-active.
- `bagger-hash` is a new crate providing MD5/SHA1/SHA256/SHA512 hashing.
- The original `scoop-hash` crate on crates.io was owned by a third party, so the crate was renamed to `bagger-hash`.
