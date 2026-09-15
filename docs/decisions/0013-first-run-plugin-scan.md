# 0013. Scan plugins on first use, triggered by whether we have looked

- **Status:** Accepted
- **Decided:** 2026-09-15
- **Recorded:** 2026-09-15
- **Confidence:** decided now
- **Evidence:** `process_projects_with_progress` in `src/lib.rs`;
  `PLUGIN_SCAN_COMPLETED_KEY` and `has_scanned_plugins` in
  `src/database/plugin_scan.rs`

## Context

After ADR-0012, a database that has never been scanned reports every plugin's
`installed` as `NULL`. That is honest, but it is not useful: a new user would see
"Unknown" against everything until they discovered `seula plugin refresh` on their own,
and nothing in the interface tells them it exists.

A full scan takes about 2m19s for 276 candidates on the maintainer's machine, so it
cannot simply run before every project scan.

## Decision

Run a plugin scan from `process_projects_with_progress` when one has never completed,
before project discovery.

**Before**, not after, because the project parse records plugin *references* which the
database layer resolves against the plugins table. Either order produces the same rows —
the upsert matches on identity regardless — but scanning first means the first run's
output is correct immediately rather than correcting itself later.

**Triggered by a recorded fact, not by an empty table.** `app_state` holds
`plugins_last_scanned_at`, written only when a *full* scan completes. The obvious
alternative — "scan if the plugins table is empty" — is wrong for a machine whose
plugins all fail to load: the table stays empty, so every project scan would trigger
another two-minute scan, forever. The question is whether we have looked, not what we
found.

A narrowed or truncated scan does not count as having looked, so it does not suppress
the first-run scan.

**Failure is non-fatal.** A missing scanner sidecar or an unreadable plugin directory
logs a warning and the project scan continues. Indexing projects is what the user asked
for; plugin metadata is not worth failing it over. The flag stays unset, so the next run
tries again.

## Rejected alternatives

- **Scan on every project scan.** Two minutes added to every rescan, almost always to
  rediscover exactly what was already known.
- **Scan if the plugins table is empty.** Cheap to implement and self-limiting in the
  common case, since a machine with no plugins scans instantly. But a machine whose
  plugins all crash the scanner would rescan forever, and that is precisely the machine
  least able to afford it.
- **Prompt the user instead of scanning.** There is no interactive surface in tray mode,
  and the gRPC client would have to implement the prompt too.
- **Scan after projects.** Same end state, but the first run reports every plugin as
  unknown and then silently changes its mind.

## Consequences

A first `seula scan` takes roughly two minutes longer than it otherwise would, and says
so: a `scanning_plugins` phase reports each plugin as it is attempted. The callback fires
on the worker's `begin` line rather than on a result, so the plugin named is the one
currently being loaded — which matters most when it is the one that hangs.

`ScanStatus` gains `SCAN_SCANNING_PLUGINS = 7` (additive, wire-compatible).

The cost is that a user who does not want plugin scanning has no way to decline it short
of editing `vst_search_paths` to a directory containing nothing. If that turns out to
matter, a config flag is the obvious fix.

## Notes

`app_state` is a general key/value table, added with `CREATE TABLE IF NOT EXISTS` and
**without** bumping `SCHEMA_VERSION`. Additive changes do not need a bump, because
`initialize()` creates missing tables on every open; bumping would discard the user's
database (ADR-0011) as the price of one new table. Reserve the bump for changes to
existing tables.
