# Launch shorts — copy/paste per network

## X / Twitter (≤280)
bagger 1.0: Scoop-compatible Windows package manager in Rust.

993ms → 14ms startup. Drop-in: same root, same apps, zero migration.

https://github.com/vincentmathis/bagger

## Bluesky (≤300)
bagger 1.0 is out — I rewrote Scoop in Rust.

Same buckets, same apps, same dirs, ~50x faster startup. No migration: point it at your Scoop setup and everything just works.

https://github.com/vincentmathis/bagger

## Mastodon (≤500)
bagger 1.0: a Scoop-compatible package manager for Windows, written in Rust.

Why: every Scoop command pays ~1s PowerShell startup tax. Bagger starts in ~14ms with identical behavior — hooks, shims, persist, MSI/Inno extraction, nightly builds, the lot, each verified against upstream source.

No migration needed: it uses the same root and manages your existing apps in place. (Fresh roots: `scoop export` → `bagger import`.)

https://github.com/vincentmathis/bagger

## LinkedIn (professional, longer)
I just released bagger 1.0 — a Scoop-compatible package manager for Windows, rewritten in Rust.

The headline number: 993ms → 14ms startup (PowerShell's startup tax, gone). But the real work was compatibility: a public line-by-line parity ledger against upstream Scoop, with every command and manifest field verified install-by-install — and no migration required, since it operates on the existing Scoop setup in place.

If you manage Windows fleets or dev environments with Scoop, it's a drop-in speed upgrade. Feedback and edge-case reports welcome:

https://github.com/vincentmathis/bagger
