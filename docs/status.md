# Status

The single source of truth for what is done, in progress, deliberately out of scope, or
known-broken. If status appears anywhere else in this repo, that copy is wrong.

Last reviewed: 2026-09-15.

## Subsystems

| Subsystem | State | Notes |
|---|---|---|
| `.als` parsing | **Done** | Single-pass state machine. ~160–270 MB/s. ADR-0001 |
| Parallel parsing | **Done** | Hand-rolled pool. Was silently serial until 2026-09-15; ADR-0002 |
| Project scanning / discovery | **Done** | `src/scan/project_scanner.rs` |
| SQLite storage | **Done** | 5NF schema, `src/database/core.rs` |
| FTS5 search | **Done** | Operators: `plugin:`, `bpm:`, `key:`, `missing:` |
| CLI | **Done** | 33+ commands, table/JSON/CSV via `src/cli/output.rs` |
| gRPC API | **Done** | 12 services in `proto/services/` |
| Tray mode | **Done** | `src/tray.rs`; default when run with no subcommand |
| File watcher | **Done** | `src/watcher/`, streams over gRPC |
| Tags / collections / tasks / notes | **Done** | |
| Media (cover art, audio) | **Done** | `src/media/`, size limits configurable |
| Plugin metadata via Ableton DB | **Works, being retired** | `src/ableton_db.rs`. ADR-0006 |
| Plugin scanner worker | **Done** | `crates/vst-meta`. ADR-0004 |
| Plugin scanner supervisor | **Done** | `src/scan/plugins/`. ADR-0004 |
| Plugin scan → database | **Not started** | Results are printed, never persisted |
| uid-based plugin matching | **Not started** | Design settled: ADR-0005 |
| `plugins` table restructure | **Not started** | Design settled: ADR-0007 |
| Analytics dashboard frontend | **Not started** | `FRONTEND_SPEC.md`, status unverified |
| Version control for projects | **Not started** | Aspiration only |
| macOS / Linux support | **Out of scope for now** | Paths exist, untested. Windows-first |

## Next

The plugin work is mid-migration. Phase 1 (worker + supervisor) landed in `b261200`.
Phase 2, in order:

1. Persist scan results — restructure `plugins` per ADR-0007.
2. Match project plugins by `(format, uid)` per ADR-0005, including the
   `plugin_classes` fallback for multi-class VST3 bundles.
3. Retire `src/ableton_db.rs` and drop the Ableton-only columns per ADR-0006.

Until 2 lands, `installed` still comes from Ableton's database.

## Known issues

| Issue | Where | Severity |
|---|---|---|
| `test_process_projects_integration` fails on `._*.als` AppleDouble sidecars found in the configured project folders. They parse as `.als`, fail, and never reach the DB — while the test asserts every discovered `.als` is present. Environment-dependent. Fixing it means deciding whether the scanner should skip `._` files. | `tests/integration/scanning.rs` | Low, but it keeps the suite red |
| `refresh_plugin_installation_status` tests `get_plugin_by_dev_identifier(..).is_ok()`, which is `true` for `Ok(None)` — so every plugin is marked installed. Superseded by step 2 above; fix sooner if the flag is trusted anywhere. | `src/database/plugins.rs:255` | Medium |
| ~16 iZotope helper DLLs in the VST3 folder are scanned and correctly classified `invalid_format`. Noise, not a bug. Filtering by heuristic risks dropping real VST2 plugins. | `src/scan/plugins/discovery.rs` | Cosmetic |
| Two `vst` crate deprecation warnings | `crates/vst-meta/src/scan.rs` | Upstream |

## Documentation triage

Undecided, needs a call from the maintainer:

- **`.kiro/`** — steering files plus a spec for the plugin subprocess that
  `crates/vst-meta` superseded. `steering/structure.md` is already stale (no
  `src/scan/plugins/`). Overlaps `CLAUDE.md` and `architecture/`. Keep, or retire?
- **`FRONTEND_SPEC.md`, `REQUIRED_FEATURES.md`, `TUI_ARCHITECTURE_ANALYSIS.md`,
  `TUI_PROJECT_PLAN.md`** — root-level planning docs. No corresponding code found for
  the TUI ones; `src/cli/interactive.rs` is a rustyline prompt, not a TUI. Aspirational,
  abandoned, or active?
- **`docs/archive/`** — two AI chat transcripts and an empty file, archived rather than
  deleted. `plugin_scanner_plan.md` proposes a C++ binary and is actively misleading.
  Git history preserves them; deleting is safe.

## Decisions not yet recorded

Candidates for ADR backfill, from the archaeology pass. These have no written rationale
anywhere — only the maintainer's memory, which is the one source that decays:

- SQLite + FTS5 over alternatives
- gRPC as the API surface (vs. REST/local IPC)
- Tray daemon + CLI dual mode
- 5NF schema normalisation
- `quick-xml` over `xml`/`elementtree` (all three are still in `Cargo.toml`)
- Gzip handling and the `flate2`/`zune-inflate` split
- Per-entity database module split
- Interactive mode via `rustyline`

Backfill opportunistically — when you next work in one of these areas, write the ADR
then. A dedicated archaeology sprint is what produced `docs/archive/`.
