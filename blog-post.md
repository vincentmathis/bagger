# bagger 1.0: a Scoop-compatible package manager in Rust

Every Scoop command pays a ~1 second tax before doing anything. That's PowerShell starting up, and you feel it on every `scoop list`, every `scoop search`, every `--version`. [bagger](https://github.com/vincentmathis/bagger) removes that tax: same buckets, same apps, same directories — starting in ~14ms. It's at 1.0 today, and there's no migration: it runs your existing Scoop setup in place.

## The pitch in one table

Median of 5 runs, same machine, 114 installed apps:

| Operation | Scoop (PS) | bagger | Speedup |
| :--- | ---: | ---: | ---: |
| `--version` | 993 ms | 14 ms | ~71x |
| `list` | 1405 ms | 26 ms | ~54x |
| `search python` | 1515 ms | 30 ms | ~51x |
| `info 7zip` | 810 ms | 26 ms | ~31x |

## Where it came from

Full credit where it's due: bagger grew out of [hok](https://github.com/chawyehsu/hok) by Chawye Hsu, which laid the foundation — core library, early commands, architecture. The 1.0 push took that foundation to full Scoop parity, verified behavior-by-behavior against upstream source, with every divergence documented rather than silently differed.

## Compatibility as a method, not a claim

"Compatible" is easy to say. The repo makes it checkable instead with `SCOOP_FEATURES.md`: a line-by-line ledger of all 28 upstream commands plus every manifest field, each marked implemented or missing with the evidence. When something diverged, it got documented as deliberate. That file is the real product, in a sense — it's what lets a skeptic answer "but does it handle X?" in ten seconds.

The verification loop behind it: read the upstream PowerShell source for a flow, diff it against the Rust implementation, write the missing piece, prove it with a unit test *and* a live run on a scratch root, commit. Some highlights:

- **Installer execution** has more cases than you'd think: `args` without `file` runs the download, `.ps1` installers are kept while binaries are removed, and installer-added `PATH` entries get scrubbed so the manifest stays in control. Each of those was a real behavioral difference found by reading `Invoke-Installer`, not by guessing.
- **Nightly builds** stamp dated directories (`nightly-20261008`), gated by the same `update_nightly` config key upstream uses.
- **`app@version` pins** resolve through bucket git history first, then autoupdate generation — the same two-step upstream does.
- The loop also caught real interop bugs, including one where an `install.json` convention mismatch broke upstream `scoop update` on a shared root — fixed and proven live in both directions.

One honest edge: each tool only manages its own shim flavor (Scoop writes `.exe`, bagger writes `.cmd`), so a shared root can accumulate the other's droppings when you switch managers mid-stream. `checkup` flags those.

## Trying it changes nothing

```ps1
scoop bucket add bagger https://github.com/vincentmathis/bagger-bucket
scoop install bagger
```

Then just run `bagger` in the existing setup — `list`, `status`, `upgrade` all work on the current apps. (Prefer a sandbox? `bagger import` takes a `scoop export` file into a fresh root, buckets auto-added, holds restored.)

## What's deliberately different

Three things, all documented: `upgrade` (inherited from hok — upstream upgrades via `update`, which bagger also accepts by forwarding), `autofetch` (upstream's equivalent is the bucket-maintainer tool `checkver -Update`), and shell completions. Everything else aims at parity, including the boring parts like `Referer` headers and SourceForge hash-failure hints.

## Try to break it

That's the actual ask: run it against a Scoop setup and file issues where behavior differs. The compat ledger tells you exactly what's covered, and anything found becomes the next entry. Questions welcome here or on the repo.
