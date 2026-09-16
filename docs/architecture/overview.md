# Architecture overview

Shape only. For *why* a shape was chosen, follow the ADR links — see `docs/README.md`.

## What runs

Seula is one binary with two modes, plus a sidecar.

| | |
|---|---|
| **Tray daemon** (default) | `src/tray.rs` + the gRPC server. Background indexing, file watching, API on `grpc_port`. |
| **CLI** | `src/cli/`. Any subcommand switches to CLI mode. `seula` with no args is interactive (rustyline). |
| **`vst-meta`** | `crates/vst-meta`. Sidecar binary, spawned per scan batch, never long-lived. ADR-0004 |

Both modes talk to the same SQLite database and the same `config.toml`.

## The project scan pipeline

```
config.paths
     │
     ▼
project_scanner.rs ──── walks directories, finds .als files
     │
     ▼
parallel.rs ─────────── worker pool, one file per worker      ADR-0002
     │
     ▼
live_set.rs ─────────── gunzip, then hand the XML to:
     │
     ▼
parser.rs ───────────── ONE pass, state machine               ADR-0001
     │                  tempo, version, time sig, key, length,
     │                  plugins, samples — all in the same traversal
     ▼
database/batch.rs ───── batched insert
```

The whole point of the shape is that `parser.rs` traverses each file exactly once.
`parallel.rs` streams results back over a channel as each file completes, which is what
lets the caller report progress per file rather than per batch.

## The plugin pipeline

Two independent sources that are (as of now) only partly joined. See
`architecture/plugins.md` — this is the most intricate area in the codebase.

```
.als parse                              system scan
     │                                       │
     ▼                                       ▼
PluginInfo                          scan/plugins/discovery.rs   ADR-0008
  name, dev_identifier, format               │
     │                                       ▼
     │                          scan/plugins/spawner.rs ──┐     ADR-0004
     │                                       │            │ NDJSON
     │                                       │            │ over a pipe
     │                                       │            ▼
     │                                       │     crates/vst-meta
     │                                       │     (subprocess — loads
     │                                       │      plugin binaries)
     │                                       ▼
     ▼                                  PluginMeta
ableton_db.rs                            uid, vendor, version, buses,
(being retired — ADR-0006)               latency, classes, …
     │                                       │
     └──────────► Plugin ◄───────────────────┘
                 (the join is ADR-0005; NOT YET IMPLEMENTED —
                  scan results are printed, never persisted)
```

## Storage

SQLite, one module per entity under `src/database/`. Schema lives in `core.rs`. FTS5
powers search with operators (`plugin:`, `bpm:`, `key:`, `missing:`).

`batch.rs` exists because per-project inserts dominated scan time at this corpus size
(thousands of projects).

## API

12 gRPC services in `proto/services/`, one handler each in `src/grpc/handlers/`. The
service list mirrors the CLI command groups — projects, samples, collections, tags,
tasks, plugins, search, media, config, system, scanning, watcher — so a feature
generally exists in both surfaces or neither.

## Configuration

`config.toml`, loaded once into a global `CONFIG` (`src/config/`). Supports
`{USER_HOME}` expansion. Several values have environment overrides (`SEULA_*`) —
`STUDIO_PROJECT_MANAGER_*` before ADR-0027 finished the product rename.

## Tests

| Location | Covers |
|---|---|
| `#[cfg(test)]` in-module | Unit logic — e.g. `scan/plugins/discovery.rs`, `scan/parallel.rs` |
| `tests/scan/` | Parser fixtures, real-corpus parsing |
| `tests/database/`, `tests/grpc/` | Storage and API |
| `tests/integration/` | End-to-end against the configured project folders |
| `tests/plugin_scanner_tests.rs` | Supervisor recovery, driven by `examples/stub_vst_worker.rs` |

Run `cargo test --workspace`, not `cargo test --test <name>` — the latter skips
examples, and the plugin scanner tests need one built.

Some tests read the maintainer's real project folders and skip or fail when those are
absent. `docs/status.md` lists the known environment-dependent failure.
