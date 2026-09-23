# Status

The single source of truth for what is done, in progress, deliberately out of scope, or
known-broken. If status appears anywhere else in this repo, that copy is wrong.

Last reviewed: 2026-09-23.

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
| HTTP API | **Done** | `src/http/`, axum, all 9 `Services` domains plus `system`. ADR-0024 |
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
| Web frontend | **Not started, specified** | Shape in `docs/architecture/frontend.md`. Stack ADR-0029, OS bridge ADR-0030, preferences ADR-0031, shell and density ADR-0032. Static mockups come before any React. The first round was rejected for inconsistency and is kept on the `old-mockups` branch. Second round, as review boards (ADR-0036): shell and projects approved; plugins (`mockup/plugins.html`) and samples (`mockup/samples.html`) awaiting review; collections and stats to come |
| Version control for projects | **Not started** | Aspiration only |
| macOS / Linux support | **Out of scope for now** | Paths exist, untested. Windows-first |

## Next

ADR-0024's HTTP router landed in full (2026-09-16): skeleton, then every domain in the
`Services` aggregator plus `system` (scan status, project add, watcher control, the two
SSE streaming endpoints, statistics with CSV export). Each domain has its own
hand-written DTOs in `src/http/dto/`, independent of the generated proto types, per the
ADR.

ADR-0028 records a tentative maintainer direction to retire the CLI and gRPC server
entirely once this surface is proven out, leaving a pure Axum API for a web frontend
and/or Tauri. Not decided or started — see that ADR before removing anything.

The plugin migration is complete. Phase 1 (worker + supervisor) landed in `b261200`;
phase 2 followed in three steps — schema and persistence, the first-run scan, and
retiring the Ableton database.

Plugin metadata now comes entirely from Seula's own scanner. `seula plugin refresh`
scans the system and persists into `plugins` / `plugin_classes` / `plugin_buses`, sweeps
what it no longer finds, and merges phantom rows into the bundle that owns their class.
`seula scan` runs that scan automatically when one has never completed. Parsing a
project records references in `plugin_refs` and resolves them by `(plugin_kind, uid)`,
falling back to `plugin_classes.class_id`.

Nothing in the plugin path reads Ableton's database any more, and `live_database_dir` is
gone from the configuration.

Worth picking up next, in no particular order:

- **A config flag to decline the first-run scan.** Today the only way out is pointing
  `vst_search_paths` at an empty directory (ADR-0013).
- **`plugin_paths`**, if duplicate install locations ever need surfacing (ADR-0010).

A full plugin scan is **2m19s for 276 candidates** (debug build, 2026-09-15). The cost
is plugin load time, not ours, so a release build will not change it much — which is why
it runs once on first use and thereafter only on demand (ADR-0013).

**Schema versioning:** `SCHEMA_VERSION` (now 3) is bumped only for changes to *existing*
tables. Adding a table needs no bump, because `initialize()` creates missing ones on
every open — and a bump discards the user's database (ADR-0011).

## Known issues

Five tests are `#[ignore]`d as of 2026-09-16 — three scan the real configured project
folders, two are hardcoded to paths on the maintainer's machine.
`cargo test --workspace` no longer runs them; use
`cargo test --workspace --tests -- --ignored` for the heavy pass. See CLAUDE.md.

| Issue | Where | Severity |
|---|---|---|
| `test_process_projects_integration` fails on `._*.als` AppleDouble sidecars found in the configured project folders. They parse as `.als`, fail, and never reach the DB — while the test asserts every discovered `.als` is present. Environment-dependent. Fixing it means deciding whether the scanner should skip `._` files. | `tests/integration/scanning.rs` | Low, but it keeps the suite red |
| `test_process_projects_with_progress` fails only when run alongside `test_process_projects_integration`. Both scan the real configured folders and write the same real database concurrently. Passes serially (`--test-threads=1`) and in isolation. Pre-existing test-isolation issue, not a product defect. Both are now `#[ignore]`d (2026-09-16, CLAUDE.md) so this only surfaces under `cargo test --tests -- --ignored`. | `tests/integration/scanning.rs` | Low |
| ~16 iZotope helper DLLs in the VST3 folder are scanned and correctly classified `invalid_format`. Noise, not a bug. Filtering by heuristic risks dropping real VST2 plugins. | `src/scan/plugins/discovery.rs` | Cosmetic |
| Two `vst` crate deprecation warnings | `crates/vst-meta/src/scan.rs` | Upstream |
| The project scan runs its blocking work (parsing, the first-run plugin scan) inside an async task, not on a blocking thread, so it occupies a runtime worker for the whole scan. The plugin scan uses `spawn_blocking` (ADR-0038). | `src/services/system.rs` (`start_scan`) | Low |

Resolved:

- **Sample sizes were invented, and the presence refresh froze the app** (found
  2026-09-23, fixed 2026-09-23). The "estimated" sample size was a guessed size per
  extension, summed; and `refresh_sample_presence_status` checked every file while holding
  the shared database mutex. Sizes are now measured by a background check that lists each
  folder once, on several threads, and locks the database only to read the paths and write
  the result (ADR-0041).
- **The sample list failed on a project-count filter combined with any other filter**
  (found 2026-09-23, fixed 2026-09-23). The count query referred to the table alias `s`
  from outside the subquery that defined it, so SQLite answered "no such column:
  s.is_present". Rewritten along with ADR-0040. Test:
  `archived_projects_count_only_in_the_all_scope` (`tests/database/project_counts.rs`).
- **A plugin's or sample's project count disagreed with its used-in list** (found
  2026-09-23, fixed 2026-09-23). The count included archived projects; the list did not.
  Both now take a `scope`, active by default (ADR-0040).
- **A rescan could never mark a sample missing** (found 2026-09-23, fixed 2026-09-23). The
  batch insert preloaded every stored sample and merged a rescan's presence into it with
  OR, then wrote it back with `is_present = EXCLUDED.is_present OR samples.is_present`.
  Once a sample had been seen, only the manual presence refresh could mark it missing.
  Every batch also rewrote every stored sample, including ones it never saw. Dated to
  `6e4f8c9` (2025-01-22) with no recorded reason; the maintainer considers it a bug. Now a
  batch writes only the samples its projects reference, and its answer replaces the stored
  one (within a batch, a sample any project finds is present). Regression test:
  `a_rescan_that_finds_a_sample_gone_marks_it_missing` (`tests/database/batch.rs`).
- **A project's samples got a new random id on every read** (found 2026-09-23, fixed
  2026-09-23). The eight queries that load a project's samples (in
  `src/database/projects.rs` and `src/database/collections.rs`) built each `Sample` with
  `Uuid::new_v4()` instead of reading the stored id, so the same sample had a different id
  in every response and could not be matched to `/api/v1/samples` or to itself in another
  project. Found when the mockup generator compared two responses that embed the same
  projects. Regression test: `a_projects_samples_keep_their_stored_ids_across_reads`
  (`tests/database/sample_ids.rs`).
- **A plugin refresh over the API froze every other request for minutes** (found
  2026-09-23, fixed 2026-09-23). `refresh_plugin_installation_status` ran the whole system
  scan (2m19s on the maintainer's library) inside a database method, while the service
  held the database mutex that every service shares. The scan now runs without the
  database, which is locked only to write the result, and the plugins view gets a
  background scan with progress, `POST /api/v1/plugins/scan` (ADR-0038). At the same time,
  a second scan no longer starts while one runs; before, a second project scan reset the
  first one's status. Regression test: `no_scan_starts_while_one_is_running`
  (`tests/grpc/server_setup.rs`).
- **Uploading a second audition audio lost the first** (found 2026-09-23, fixed
  2026-09-23). `store_audio_file` pointed `projects.audio_file_id` at the new upload,
  leaving the previous file referenced by nothing, so the next orphan cleanup deleted it.
  Uploads now join the project's audio list and become the primary only if there was none
  (ADR-0037). Tests: `tests/database/audio_files.rs`.
- **Every scale except Major and Minor was stored as `Empty`** (found 2026-09-23, fixed
  2026-09-23). `src/scan/parser.rs` mapped only those two of Ableton's scale names; the
  rest fell to `Scale::Empty` behind an "add other scale mappings as needed" comment. In
  the maintainer's files that was 1,048 key-detected clips, including 510 Mixolydian.
  Fixed by mapping through a full table of Ableton's names, with `Scale::Other` keeping
  any name the table lacks (ADR-0035). Regression tests: `models::key_name_tests`.
  **Projects parsed before the fix keep `Empty` until a forced rescan**, because the
  scanner re-parses only changed files.
- **`--config <path>` was accepted and ignored** (found 2026-09-23, fixed 2026-09-23).
  `Cli` declared the flag but nothing read it: `CONFIG` is a lazy static that loaded
  before `Cli::parse()` ran, from `SEULA_CONFIG` or the default locations. Fixed in
  `src/main.rs` by parsing first and exporting the flag as `SEULA_CONFIG` before `CONFIG`
  is touched. A missing path now exits with an error rather than falling through to
  another config, which the env var still does. Checked by running the mock data
  generator, which now passes `--config` and gets its own port.
- **`seula --server` was unreachable** (found 2026-09-23, fixed 2026-09-23). Server-only
  mode (2025-08-14) checked for `--server` by scanning raw argv, but a week later the clap
  CLI put `Cli::parse()` in front of that check, and clap rejects any flag it does not
  declare. So the mode could not be started for over a year. Found when the mockup data
  generator needed a headless server. Fixed by declaring `--server`/`-s` on `Cli` and
  branching on the parsed flag. Regression test: `server_flag_parses` (`src/cli/mod.rs`).

- **`plugin_count` inflation in the `plugin vendors` / `plugin formats` aggregates**
  (found 2026-09-16, fixed 2026-09-16). The `usage_stats` subquery has one row per
  `(plugin_id, project_id)` pair; the outer `LEFT JOIN` fanned each plugin row out once
  per project it's used in, so `COUNT(*)` and the installed/missing/unknown
  `SUM(CASE ...)`s counted that plugin once per project instead of once.
  `total_usage_count` and `unique_projects_using` were unaffected — they already
  summed/`COUNT(DISTINCT)`ed correctly over the fanned rows. Fixed by switching
  `plugin_count` and the three status counts to `COUNT(DISTINCT p.id)` /
  `COUNT(DISTINCT CASE WHEN ... THEN p.id END)`, which collapse back to one count per
  plugin regardless of the fan-out. Regression test:
  `vendor_and_format_plugin_count_does_not_inflate_with_project_usage`
  (`tests/database/plugin_scan.rs`) — a plugin used in three projects must still count
  once. `src/database/plugins.rs` (`vendor_stats`/`format_stats` CTEs). ADR-0026.

## Documentation triage

Resolved:

- **`.kiro/`** — moved into `docs/archive/.kiro/` (2026-09-16). Superseded by
  `crates/vst-meta` and `CLAUDE.md`/`architecture/`; kept for history rather than deleted,
  same treatment as the rest of `docs/archive/`.

Undecided, needs a call from the maintainer:

- **`FRONTEND_SPEC.md`, `REQUIRED_FEATURES.md`, `TUI_ARCHITECTURE_ANALYSIS.md`,
  `TUI_PROJECT_PLAN.md`** — root-level planning docs. No corresponding code found for the
  TUI ones; `src/cli/interactive.rs` is a rustyline prompt, not a TUI. Aspirational,
  abandoned, or active?
- **`docs/archive/`** — two AI chat transcripts and an empty file, archived rather than
  deleted. `plugin_scanner_plan.md` proposes a C++ binary and is actively misleading. Git
  history preserves them; deleting is safe.

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
