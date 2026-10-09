# Launch shorts — copy/paste per network

## X / Twitter (≤280)
bagger 1.0: Scoop-compatible Windows package manager in Rust.

993ms → 14ms startup. Same buckets, same manifests. Migrate in 2 commands.

https://github.com/vincentmathis/bagger

## Bluesky (≤300)
bagger 1.0 is out — I rewrote Scoop in Rust.

Same buckets and manifests, ~50x faster startup, verified behavior-by-behavior against upstream. `scoop export` → `bagger import` and you're moved.

https://github.com/vincentmathis/bagger

## Mastodon (≤500)
bagger 1.0: a Scoop-compatible package manager for Windows, written in Rust.

Why: every Scoop command pays ~1s PowerShell startup tax. Bagger starts in ~14ms with identical behavior — hooks, shims, persist, MSI/Inno extraction, nightly builds, the lot, each verified against upstream source.

Migration: `scoop export > scoopfile.json`, then `bagger import scoopfile.json`. Buckets auto-added, holds restored.

https://github.com/vincentmathis/bagger

## LinkedIn (professional, longer)
I just released bagger 1.0 — a Scoop-compatible package manager for Windows, rewritten in Rust.

The headline number: 993ms → 14ms startup (PowerShell's startup tax, gone). But the real work was compatibility: a public line-by-line parity ledger against upstream Scoop, with every command and manifest field verified install-by-install — plus a 2-command migration path (`scoop export` / `bagger import`) proven in both directions.

If you manage Windows fleets or dev environments with Scoop, it's a drop-in speed upgrade. Feedback and edge-case reports welcome:

https://github.com/vincentmathis/bagger
