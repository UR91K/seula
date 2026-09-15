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
  grpc/          12 services, one handler each
  cli/           clap commands + interactive mode; output.rs owns table/JSON/CSV
  config/        config.toml loading, validation, platform paths
  media/         cover art and audio file storage
  watcher/       filesystem change notifications
  ableton_db.rs  reads Ableton's own plugin DB            (being retired, ADR-0006)
crates/
  vst-meta/      the plugin scanner worker — runs as a SUBPROCESS (ADR-0004)
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
`dev_identifier` is a wrapper around the plugin's own native ID. See ADR-0005.

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
