# bagger 1.0 launch checklist

Files in this folder:
- `bagger-demo.gif` (900x520, 367KB) — migration demo, embed in README/blog/posts
- `blog-post.md` — long-form post (dev.to / personal blog)
- `social-shorts.md` — X, Bluesky, Mastodon, LinkedIn copy
- `render_demo.py` — regenerates the GIF if the script changes

Texts already drafted in chat: This Week in Rust snippet, Show HN title + first comment,
r/scoop and r/rust angles.

## Order
- [ ] This Week in Rust PR (weekly deadline — first, it's calendar-bound)
- [ ] Blog post live (link target for everything else)
- [ ] Show HN, weekday morning US time, stay for comments (~3h)
- [ ] r/scoop, then r/rust a day apart
- [ ] Social shorts anytime
- [ ] Scoop Discord showcase (participate first, no drive-bys)
- [ ] AlternativeTo listing (15 min filler)

## Prerequisites (all done except winget)
- [x] GitHub release 1.0.2 with x64/i686/aarch64 + sha256
- [x] crates.io 1.0.2 (all three crates)
- [x] Scoop bucket current (1.0.2, verified install)
- [x] install.ps1 one-liner proven against latest
- [x] README: bucket method + migration section + fresh benchmarks
- [ ] Winget PR merged (manifests built, not submitted — winget.run listing follows)
- [ ] Consider: embed demo GIF in README hero

## Day-of checklist for Show HN
- [ ] Bucket + release + crates versions all current (re-verify morning-of)
- [ ] Reply to every technical question (compat ledger = cheat sheet)
- [ ] Watch: winget questions ("in review"), ARM ("CI-built aarch64 attached"),
  nightly/version-pin edge cases (all implemented — point at SCOOP_FEATURES.md)
