# Status

The single source of truth for what is done, in progress, deliberately out of scope, or
known-broken. If status appears anywhere else in this repo, that copy is wrong.

Last reviewed: 2026-09-16.

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
| Plugin metadata via Ableton DB | **Removed** | Retired 2026-09-15. ADR-0006 |
| Plugin scanner worker | **Done** | `crates/vst-meta`. ADR-0004 |
| Plugin scanner supervisor | **Done** | `src/scan/plugins/`. ADR-0004 |
| Plugin scan → database | **Done** | `src/database/plugin_scan.rs`. `seula plugin refresh` scans and persists |
| First-run plugin scan | **Done** | Runs before project discovery when one has never completed. ADR-0013 |
| Plugin identity (`PluginKey`) | **Done** | `src/models.rs`. Parses `dev_identifier`, derives from a scanned uid. ADR-0005 |
| uid-based plugin matching | **Done** | In the batch layer, with the `plugin_classes` fallback. ADR-0005, ADR-0009 |
| `plugins` table restructure | **Done** | ADR-0007, ADR-0009, ADR-0010, ADR-0011; tri-state `installed` is ADR-0012 |
| Analytics dashboard frontend | **Not started** | `FRONTEND_SPEC.md`, status unverified |
| Version control for projects | **Not started** | Aspiration only |
| macOS / Linux support | **Out of scope for now** | Paths exist, untested. Windows-first |

## Next

The plugin migration is complete. Phase 1 (worker + supervisor) landed in `b261200`;
phase 2 followed in three steps — schema and persistence, the first-run scan, and
retiring the Ableton database.

Plugin metadata now comes entirely from Seula's own scanner. `seula plugin refresh`
scans the system and persists into `plugins` / `plugin_classes` / `plugin_buses`, sweeps
what it no longer finds, and merges phantom rows into the bundle that owns their class.
`seula scan` runs that scan automatically when one has never completed. Parsing a project
records references in `plugin_refs` and resolves them by `(plugin_kind, uid)`, falling
back to `plugin_classes.class_id`.

Nothing in the plugin path reads Ableton's database any more, and `live_database_dir` is
gone from the configuration.

Worth picking up next, in no particular order:

- **A config flag to decline the first-run scan.** Today the only way out is pointing
  `vst_search_paths` at an empty directory (ADR-0013).
- **`plugin_paths`**, if duplicate install locations ever need surfacing (ADR-0010).

A full plugin scan is **2m19s for 276 candidates** (debug build, 2026-09-15). The cost is
plugin load time, not ours, so a release build will not change it much — which is why it
runs once on first use and thereafter only on demand (ADR-0013).

**Schema versioning:** `SCHEMA_VERSION` (now 3) is bumped only for changes to *existing*
tables. Adding a table needs no bump, because `initialize()` creates missing ones on
every open — and a bump discards the user's database (ADR-0011).

## Known issues

Five tests are `#[ignore]`d as of 2026-09-16 — three scan the real configured project
folders, two are hardcoded to paths on the maintainer's machine. `cargo test --workspace`
no longer runs them; use `cargo test --workspace --tests -- --ignored` for the heavy
pass. See CLAUDE.md.

| Issue | Where | Severity |
|---|---|---|
| `test_process_projects_integration` fails on `._*.als` AppleDouble sidecars found in the configured project folders. They parse as `.als`, fail, and never reach the DB — while the test asserts every discovered `.als` is present. Environment-dependent. Fixing it means deciding whether the scanner should skip `._` files. | `tests/integration/scanning.rs` | Low, but it keeps the suite red |
| `test_process_projects_with_progress` fails only when run alongside `test_process_projects_integration`. Both scan the real configured folders and write the same real database concurrently. Passes serially (`--test-threads=1`) and in isolation. Pre-existing test-isolation issue, not a product defect. Both are now `#[ignore]`d (2026-09-16, CLAUDE.md) so this only surfaces under `cargo test --tests -- --ignored`. | `tests/integration/scanning.rs` | Low |
| ~16 iZotope helper DLLs in the VST3 folder are scanned and correctly classified `invalid_format`. Noise, not a bug. Filtering by heuristic risks dropping real VST2 plugins. | `src/scan/plugins/discovery.rs` | Cosmetic |
| Two `vst` crate deprecation warnings | `crates/vst-meta/src/scan.rs` | Upstream |
| `plugin_count` in the `plugin vendors` / `plugin formats` aggregates counts plugin-project pairs, not plugins — the usage subquery groups by `(plugin_id, project_id)`, the `LEFT JOIN` multiplies each plugin row by its project count, and `COUNT(*)` counts the multiplied rows. A plugin used in 5 projects counts as 5. Found 2026-09-16 while adding the unscanned count to these two aggregates (`61cfc23`, ADR-0025's follow-up); the new reconciliation test (`vendor_and_format_aggregates_account_for_unscanned_plugins`) doesn't catch it because installed/missing/unknown all inflate together and still sum to the (wrong) `plugin_count`. `seula plugin stats` counts correctly and disagrees with both. Fix is grouping the usage subquery by `plugin_id` alone and computing `unique_projects_using` inside it. | `src/database/plugins.rs` (`vendor_stats`/`format_stats` CTEs) | Medium — user-visible wrong numbers, not just a red test |

## Documentation triage

Resolved:

- **`.kiro/`** — moved into `docs/archive/.kiro/` (2026-09-16). Superseded by
  `crates/vst-meta` and `CLAUDE.md`/`architecture/`; kept for history rather than
  deleted, same treatment as the rest of `docs/archive/`.

Undecided, needs a call from the maintainer:

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
