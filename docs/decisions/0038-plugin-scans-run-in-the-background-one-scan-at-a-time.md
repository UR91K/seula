# 0038. Plugin scans run in the background, and one scan runs at a time

- **Status:** Accepted — implemented 2026-09-23
- **Recognized:** 2026-09-23, planning the plugins board
- **Decided:** 2026-09-23
- **Recorded:** 2026-09-23
- **Confidence:** decided now
- **Evidence:** `src/database/plugins.rs` (the former
  `refresh_plugin_installation_status`, which called `scan_system` from inside a
  `&mut self` database method); `src/services/plugins.rs` (which held the shared database
  mutex around that call); `docs/status.md` (a full scan is 2m19s for 276 candidates);
  `src/services/system.rs` (`start_scan`, the project scan's status and progress);
  ADR-0024 (streaming is SSE); ADR-0013 (the first-run scan)

## Context

The plugins view needs a "Scan plugins" action with progress. The only HTTP route that
scanned was `POST /api/v1/plugins/refresh-installation-status`. It ran the whole scan
inside the request and answered when it was done, which takes minutes on a real library.
Worse, it scanned while holding the database mutex that every service shares, so for
those minutes every other request waited, and the UI would have frozen.

The project scan already had the shape a UI needs. `POST /api/v1/system/scan` starts it
in the background and streams progress as Server-Sent Events, and
`GET /api/v1/system/scan-status` reports the current status and latest progress to any
client. Its status values already include `scanning_plugins`, used by the first-run
plugin scan (ADR-0013).

Nothing stopped two scans running at once. A second `POST /system/scan` reset the status
under the first and started another.

## Decision

**A background plugin scan, reported like a project scan.** `POST /api/v1/plugins/scan`
starts the scan and streams its progress as SSE:

- One `scanning_plugins` event while the plugins are found, then one per plugin binary,
  naming it ("Scanning Serum.dll") with `completed` of `total`.
- A final `completed` event, which also carries what the scan changed (`result`: the
  `PluginRefreshResult` the synchronous route returns), or an `error` event.

Each event has the project scan's fields (`completed`, `total`, `progress`, `message`,
`status`), plus `result`. The events also update the status and progress behind
`GET /api/v1/system/scan-status`, so a window that did not start the scan can follow it.

**The scan runs without the database.** It runs on a blocking thread, and the database
is locked only to write the report, in one transaction. The synchronous route and the
CLI's `seula plugin refresh` go through the same path, so they no longer block other
requests either.

**One scan at a time.** A project scan and a plugin scan share one status, and neither
starts while the status says a scan is running. Starting either while one runs answers
`409 Conflict` over HTTP, and `FAILED_PRECONDITION` over gRPC. Before, a second project
scan silently reset the first one's status.

## Rejected alternatives

- **A separate status for plugin scans.** Rejected because the status bar would need two
  scan indicators. It would also allow a plugin scan and a project scan at once, and the
  project scan's first-run step can itself scan plugins, so two scanner worker pools could
  load the same plugins at the same time.
- **A job resource (`POST` returns an id, `GET /jobs/:id` polls it).** Rejected because
  the project scan already set the pattern: an SSE stream for the client that started the
  scan, and the shared scan status for everyone else. A second pattern for the same kind
  of work gains nothing.
- **Keep the synchronous route and only release the lock.** It would stop the freeze, but
  a UI would still wait minutes on one request, with no progress to show.

## Consequences

`POST /api/v1/system/scan` can now answer 409, where it used to always start. The gRPC
`ScanDirectories` call can now fail with `FAILED_PRECONDITION`.

`POST /api/v1/plugins/refresh-installation-status` stays. It is synchronous and does not
take the shared scan status, so it can run beside a background scan. Nothing in the
frontend uses it; it is kept for scripts, and ADR-0028 retires the gRPC call that
mirrors it.

A client that connects to `scan-status` after a plugin scan finishes sees `completed`
and the final message, but not `result`: the stored progress uses the project scan's
type, which has no field for it. The plugins view refetches its list and stats when the
stream ends, so it does not need `result` from there.

## Notes

The project scan still runs its blocking work (parsing, and the first-run plugin scan)
inside an async task rather than on a blocking thread, which occupies a runtime worker
for the whole scan. That predates this decision and is noted in `docs/status.md`. Moving
it to `spawn_blocking`, as the plugin scan does, is cheap if it ever matters.
