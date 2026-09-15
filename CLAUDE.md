# Seula

Ableton Live project manager. Scans `.als` files, extracts metadata into SQLite, and
exposes it over a CLI and a gRPC API. Runs either as a system tray daemon or a CLI.

Rust, Windows-focused (macOS/Linux paths exist but are untested).

## Layout

```
src/
  scan/          .als discovery and parsing
    parser.rs      single-pass state machine over the XML  (ADR-0001)
    parallel.rs    manual worker pool                      (ADR-0002)
    plugins/       system plugin scanning, supervisor      (ADR-0004)
  database/      SQLite, one module per entity, FTS5 search
    batch.rs       project batch insert; resolves plugin references (ADR-0005, 0009)
    plugin_scan.rs persists plugin scan results                     (ADR-0007, 0012)
  grpc/          12 services, one handler each
  cli/           clap commands + interactive mode; output.rs owns table/JSON/CSV
  config/        config.toml loading, validation, platform paths
  media/         cover art and audio file storage
  watcher/       filesystem change notifications
crates/
  vst-meta/      the plugin scanner worker — runs as a SUBPROCESS (ADR-0004)
                 the sole source of plugin metadata since ADR-0006
proto/services/  gRPC definitions
```

## Commands

```bash
cargo build --workspace
cargo test --workspace          # NOT `cargo test --test X` — that skips examples,
                                # and tests/plugin_scanner_tests.rs needs one
cargo run -- --help             # CLI
cargo run                       # tray mode (default)
```

## Rules that are easy to break

**Never load a plugin in the main process.** Plugin binaries are third-party native
code that segfaults, hangs, and cannot be reliably unloaded. `vst-meta` is depended on
with `default-features = false` precisely so the VST hosts are not linked here. If you
find yourself enabling its `host` feature in the root `Cargo.toml`, stop. See ADR-0004.

**`src/scan/parser.rs` is deliberately one pass.** It is long and stateful and looks
like it wants splitting into per-datum extractors. It used to be exactly that, and the
rewrite was the single biggest performance win in the project. See ADR-0001.

**`src/scan/parallel.rs` is deliberately a hand-rolled pool.** rayon, crossbeam MPMC
and async have all been considered and rejected on the merits. See ADR-0002 — which
also documents the guard-lifetime bug that made it silently serial for its whole life,
so do not reintroduce `while let Ok(x) = rx.lock().unwrap().recv()`.

**Plugin identity is `(format, uid)`, never the name or version.** Ableton's
`dev_identifier` is a wrapper around the plugin's own native ID. Parse it with
`PluginKey`, which has nowhere to put Ableton's instr/audiofx call — unlike
`PluginFormat`, whose four variants bake it in. See ADR-0005.

**Only a plugin scan writes `plugins.installed`.** Parsing a project records a
*reference*; a project file cannot know what is installed on this machine. The flag is
tri-state — `NULL` means no scan has looked, which is not the same as looked-and-absent
— so a partial scan can leave plugins it never reached alone instead of declaring them
missing. See ADR-0012.

**Bump `SCHEMA_VERSION` only for changes to existing tables.** Adding a table needs no
bump — `initialize()` creates missing ones on every open. A bump *discards the user's
database* (ADR-0011), which is an absurd price for one new table.

**Store uids and class IDs through `PluginKey::uid_hex()`.** Ableton writes them
lowercase and dashed, the scanner uppercase and undashed. Skip the normalisation and
every lookup silently misses.

## When something looks odd

Parts of this codebase are deliberately unusual, and the reasoning is frequently older
than the code. If something looks wrong-shaped, has no ADR, and the tests pass:

**Do not silently "fix" it, and do not silently leave it.** Both throw away information.

1. Check `docs/decisions/` and the module's own `//!` header. Several of these are
   already answered, at length.
2. If nothing covers it, ask — naming what looks odd *and* what you would expect
   instead. A concrete alternative gets a far better answer than "why is this like
   this?", because it can be accepted or rejected on the merits.
3. Write the answer down before moving on, in whichever home it belongs to:

| The answer you get | Where it goes |
|---|---|
| "Deliberate, because X" | new ADR, `Accepted (retrospective)` |
| "I looked at changing it and decided not to" | new ADR, `Reaffirmed` |
| "No idea, that's just what I wrote" | new ADR, `Incidental` — say what now depends on it |
| "That's a bug" | fix it, add a regression test, note it in `docs/status.md` |
| Too small to be a decision | a `//` note at the site |

Batch questions and ask at a natural pause, unless the answer changes what you do next.
One round of five beats five interruptions.

Two things worth knowing. **"I don't remember" is a real answer** — record it as
`Incidental` rather than inventing a plausible rationale; a fiction manufactures
confidence in a choice nobody made. And **odd-but-deliberate and actually-broken are not
exclusive**: `src/scan/parallel.rs` was both — the design was right and had been argued
out carefully, while a language-level detail one layer below the argument had silently
disabled it for the code's entire life. Verify, then ask.

`docs/status.md` lists decisions with no written rationale anywhere. If your question
lands on one of those, you have found the most valuable thing you can write today: the
maintainer's memory is the only source for them, and the only one that decays.

## Known, not broken

- `test_process_projects_integration` fails on `._*.als` — macOS AppleDouble sidecar
  files in the configured project folders. Environment-dependent, pre-existing.
  Tracked in `docs/status.md`.
- Two `vst` crate deprecation warnings in `crates/vst-meta` — upstream, unavoidable.

## Documentation

`docs/README.md` explains the layout and the one rule that keeps it honest. Briefly:

| Question | Where it is answered |
|---|---|
| What state is X in? | `docs/status.md` — the only place status lives |
| Why is it built this way? | `docs/decisions/` — ADRs, append-only, never edited |
| How do the pieces fit? | `docs/architecture/` |
| How does this module work? | the `//!` header in the module itself |
| How do I use it? | `README.md` |

Before changing something that looks over-engineered, check `docs/decisions/` — several
of these choices were argued out at length and the reasoning is not visible in the code.
