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
| `bagger install <app>` | [x] | Full install with deps, shims, shortcuts, persist, env vars |
| `bagger uninstall <app>` | [x] | Supports `-p` (purge), cascade removal |
| `bagger update <app>` | [x] | Single app update |
| `bagger upgrade` | [x] | Upgrade all installed apps |
| `bagger search <query>` | [x] | Search across all buckets |
| `bagger list` | [x] | List installed apps; supports `--upgradable` |
| `bagger info <app>` | [x] | Show manifest info for any app |
| `bagger cat <app>` | [x] | Show manifest JSON |
| `bagger hold <app>` | [x] | Mark apps as held |
| `bagger unhold <app>` | [x] | Remove hold from apps |
| `bagger prefix <app>` | [x] | Show installation path for an app |
| `bagger which <command>` | [x] | Find which app owns an executable |
| `bagger status` | [x] | Show held, upgradable, and running apps |
| `bagger which <command>` | [ ] | Find which app owns an executable |
| `bagger prefix <app>` | [ ] | Show installation path for an app |
| `bagger checkup` | [x] | Check for updates, report held/running/upgradable apps |
| `bagger reset <app>` | [x] | Reset an installed package to a specific version or re-extract |

### Bucket Commands

| Command | Status | Notes |
| :--- | :---: | :--- |
| `bagger bucket add <name> [url]` | [x] | Add bucket from built-in list or custom URL |
| `bagger bucket remove <name>` | [x] | Remove bucket |
| `bagger bucket update` | [x] | Update all buckets (parallel, git-based) |
| `bagger bucket list` | [x] | List added buckets |

### Cache Commands

| Command | Status | Notes |
| :--- | :---: | :--- |
| `bagger cache list [query]` | [x] | List cached downloads |
| `bagger cache remove <query>` | [x] | Remove cached files by name/version |
| `bagger cache clean` | [~] | Covered via `cache remove *` |

### Config & Utility Commands

| Command | Status | Notes |
| :--- | :---: | :--- |
| `bagger config [set] <key> [value]` | [x] | Get/set Scoop config.json |
| `bagger cleanup <app>` | [x] | Remove old versions, keep current |
| `bagger home <app>` | [x] | Open app homepage in browser |
| `bagger shim` | [x] | List, add, and remove shims |
| `bagger alias` | [x] | Manage command aliases (list, add, rm) |
| `bagger import` | [x] | Import apps from JSON (lists for install) |
| `bagger export` | [x] | Export installed apps to JSON |
| `bagger help` | [x] | Auto-generated via clap |

### Download & Verify Commands

| Command | Status | Notes |
| :--- | :---: | :--- |
| `bagger create` | [x] | Create a bucket or manifest template |
| `bagger depends <app>` | [x] | Show dependencies and reverse dependencies |
| `bagger download <app>` | [x] | Download package without installing |
| `bagger checkver <app>` | [ ] | Check for latest version from manifest URLs |
| `bagger autofetch` | [ ] | Auto-generate manifests from app URLs |

### Security Commands

| Command | Status | Notes |
| :--- | :---: | :--- |
| `bagger virustotal <app>` | [ ] | Scan downloads with VirusTotal API |

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
| `cookie` | [x] | Parsed (not actively used in download) |
| `notes` | [x] | Displayed after install |
| `suggest` | [x] | Parsed (suggested apps list) |

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
| `architecture.*.checkver` | [x] | Parsed but not actively used |

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
| `checkver` | [x] | Parsed: regex, url, jsonpath, xpath, script |
| `autoupdate` | [~] | Parsed but not actively used for auto-updates |
| `runtime` | [ ] | Runtime dependencies (e.g. VC++ redistributables) |

### Hash Extraction (`autoupdate.hash`)

| Field | Status | Notes |
| :--- | :---: | :--- |
| `regex` | [~] | Parsed but not used for hash extraction |
| `jsonpath` | [~] | Parsed but not used |
| `xpath` | [~] | Parsed but not used |
| `url` | [~] | Parsed but not used |
| `find` | [~] | Alias for `regex` |
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
| `aria2-*` settings | [x] | Parsed (aria2 not integrated) |
| `use_external_7zip` | [x] | Parsed (uses bundled 7z) |
| `scoop_branch` | [~] | Parsed (not actively used) |
| `scoop_repo` | [~] | Parsed (not actively used) |
| `gh_token` | [x] | Parsed for GitHub private repos |
| `private_hosts` | [x] | Parsed for auth headers |
| `alias` | [x] | Parsed (no CLI to manage) |
| `use_sqlite_cache` | [~] | Parsed (not implemented) |
| `show_manifest` | [~] | Parsed (display-only) |
| `ignore_running_processes` | [~] | Parsed (not actively used) |
| `shim` | [~] | Parsed (affects shim creation) |

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
| Multiple architecture support | [x] | 32bit, 64bit, arm64 |
| Cross-bucket package replacement | [x] | When package moves buckets |
| Candidate selection (multi-bucket) | [x] | Interactive or auto-select |
| Global app installation | [~] | Global path defined, but no `--global` flag on CLI |
| Isolated app installation | [~] | Manifest URL install planned but not fully wired |
| Aria2 integration | [ ] | Config exists but aria2 binary not used |
| SQLite manifest cache | [ ] | Config option exists, not implemented |
| Manifest auto-review prompt | [ ] | `show_manifest` config exists, not implemented |
| VirusTotal integration | [ ] | No implementation |
| App execution during install | [~] | Pre/post install/uninstall scripts run via PowerShell |
| Custom installer/uninstaller | [x] | Supports script or file-based installers |

---

## Missing Commands (Medium-High Priority)

1. **`scoop/virustotal`** - Scan downloads with VirusTotal API
2. **`scoop/checkver`** - Check latest version from app manifest URLs
3. **`scoop/autofetch`** - Auto-generate app manifests from app URLs

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

1. **Aria2 download manager integration** - Faster parallel downloads
2. **SQLite manifest caching** - Speed up operations
3. **Global app installs** (`--global` flag) - Install to `ProgramData`
4. **Isolated package installs** - Install from arbitrary manifest URL without adding to bucket
5. **Runtime dependencies** - Handle VC++ redistributables, .NET, etc.
6. **Manifest auto-review** - Show manifest before install prompt
7. **`aria2` warning suppression** - Config-driven warning control
8. **Ignore running processes** - Force install/upgrade even with running processes

## Notes

- The core install/uninstall/upgrade flow is fully implemented with commit logic supporting archives, shims, shortcuts, persist, psmodule, env, and scripts.
- PowerShell script invocation is functional but requires `powershell.exe` (Windows-only).
- Archive extraction depends on an external `7z` executable.
- The library (`scoop-rs`) is API-complete for manifests but some options are parsed-but-not-active.
- `bagger-hash` is a new crate providing MD5/SHA1/SHA256/SHA512 hashing.
