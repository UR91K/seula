# 0006. Retire the Ableton plugin database dependency

- **Status:** Accepted — implemented 2026-09-15
- **Decided:** 2026-09-15
- **Recorded:** 2026-09-15
- **Confidence:** decided now
- **Evidence:** the former `ableton_db.rs` (deleted by this decision); `make_plugin` in
  `src/scan/parser.rs`

## Context

Everything Seula knows about a plugin beyond its name currently comes from Ableton's own
SQLite database. `make_plugin` looks up each `dev_identifier` and copies across vendor,
version, SDK version, Ableton's row ids, and its scan bookkeeping — with `installed`
meaning nothing more than "the lookup hit".

Three problems:

1. **It depends on Ableton having scanned recently.** A plugin Ableton has not indexed
   is indistinguishable from one that is not installed.
2. **It is limited to the fields Ableton happens to store.** No bus layouts, channel
   counts, latency, or GUI presence.
3. **The schema drifts between Live versions.** `build_plugin_query`
   (in the former `ableton_db.rs`) introspects the `plugins` table's columns at runtime and
   selects only the ones that exist. That workaround, and the `Live-plugins-*.db`
   filename filter added in `cb04a1b` / `802ec38` after "most recent `.db`" kept picking
   the *files* database, are both load-bearing and both fragile.

ADR-0004 and ADR-0005 together remove the need: the scanner reads richer data directly
from the binaries, and the join key is the plugin's own identifier, not Ableton's.

## Decision

Once uid matching lands, delete `ableton_db.rs` and drop the Ableton-only fields
from `Plugin`: `plugin_id`, `module_id`, `flags`, `scanstate`, `enabled`. `installed`
becomes a function of scan results.

## Rejected alternatives

- **Keep it permanently as a fallback** when a uid lookup misses. Preserves matching for
  plugins the scanner cannot load, but means maintaining both paths and the
  column-introspecting query indefinitely — and a plugin the scanner cannot load is
  usually one that is genuinely broken, which is worth surfacing rather than papering
  over.
- **Retire it immediately, in the same change as the scanner.** Rejected on sequencing:
  crash isolation needed validating against a real library before anything depended on
  it. The scanner landed alongside the old path instead.

## Consequences

Plugin metadata stops depending on another application's internal state, and the
fragile schema-compatibility code goes with it.

Five fields on `Plugin` lose their source. They are Ableton-internal bookkeeping that
means nothing outside Ableton's own scanner, so dropping them is the point rather than a
cost — but anything consuming them over gRPC will need updating.

This is a breaking schema change. Backwards compatibility with older Seula databases is
explicitly not required.

## Notes

Implemented 2026-09-15, after ADR-0009 and ADR-0013. What actually went:

- `ableton_db.rs` and its `DbPlugin` row type.
- `Plugin::reparse`, the `INSTALLED_PLUGINS` cache, and `LiveSet::reparse_plugins`.
- The transitional per-batch fallback added in phase 2 step 1.
- `plugin_id`, `module_id`, `sdk_version`, `flags`, `scanstate` and `enabled` from
  `Plugin`, the `plugins` table, and `Plugin` in `proto/common.proto` — field numbers
  reserved rather than recycled.
- `live_database_dir` from the configuration, which pointed at Ableton's database
  directory and had no other purpose.

`sdk_version` went with them, which this ADR did not originally list. It is a real VST3
concept the scanner could report, but nothing populated it except Ableton, so keeping a
permanently-NULL column would have been worse than removing it.

Re-adding it was raised and **declined** by the maintainer on 2026-09-15: no use case for
knowing a plugin's SDK version could be named. It is cheap to reverse — a `vst-meta`
change plus one column, no schema argument — but it should stay gone until something
actually wants it.

`refresh_plugin_installation_status` (`src/database/plugins.rs:255`) is replaced by this
work. It currently tests `get_plugin_by_dev_identifier(..).is_ok()`, which returns `true`
for `Ok(None)` — so every plugin is marked installed regardless. Worth fixing sooner if
that flag is trusted anywhere in the meantime.
