# 0056. A scan's stream is read from the response body of the POST that starts it

- **Status:** Accepted — implemented on branch `tauri-app`
- **Recognized:** 2026-10-04, ADR-0047's "Not decided here" note on the client layer
- **Decided:** 2026-10-04, accepted by the maintainer after review (proposed the same day)
- **Recorded:** 2026-10-04
- **Confidence:** decided now; drafted by the assistant while implementing, verified
  against a real plugin scan, reviewed against the code and accepted by the maintainer
- **Evidence:** ADR-0047 (names this exact choice); ADR-0038 (one scan at a time, in the
  background); ADR-0024 (the SSE endpoints); `web/shared/api.ts` (`sse`, `Api.scan`);
  `src/http/handlers/system.rs` (`scan_directories` answers 409 when a scan runs)

## Context

The project scan, the plugin scan and the sample check each start with a `POST` whose
response is the progress stream (ADR-0024). The browser's `EventSource` makes only `GET`
requests, so it cannot read them. ADR-0047 named the two ways out: read the stream from a
`fetch` response body, or add a `GET` route to the daemon that follows a running scan.
Of the three streams (project scan, plugin scan, sample check), `Api.scan` covers the
first two today; see Notes.

The window can be closed while a scan runs, because the daemon outlives it (ADR-0048). A
reopened window would have no stream to read.

## Decision

**The client reads the stream itself.** `sse()` in `shared/api.ts` does a `fetch`, pipes
the body through a `TextDecoderStream`, splits on blank lines and parses each `data:`
payload. `Api.scan(kind)` is that call against the project or the plugin route. The scan
runner in `shell/shell.ts` loops over it and writes each event to one signal that only
the status bar reads.

**A second scan is the daemon's to refuse.** The runner ignores a start while its own
stream is open, and the daemon answers `409` to a start from anywhere else. The client
shows that error in the status bar. It does not check ahead.

**No `GET` follow stream is added.** The daemon does not grow an endpoint for this. It
already has `GET /api/v1/system/scan-status`, a JSON snapshot of the running scan's
progress, which the plugin scan handler documents as the way for a client that did not
start the scan to follow it. The client does not read it today.

## Rejected alternatives

- **A `GET` route that follows the running scan.** The better design for a window that
  closes and reopens mid-scan: the window could attach to whatever is running. Rejected
  for now because it changes the daemon for a case no screen needs yet, and because it
  would let a stream be read by more than one client, which nothing has designed or
  tested. If re-attaching is wanted, the first thing to reach for is polling the existing
  `scan-status` snapshot, which needs no daemon change; a follow stream comes after that.
- **`EventSource` on a `GET` that starts the scan.** A `GET` that changes state, which
  caches and prefetchers are allowed to repeat. Rejected.
- **A library for reading SSE over `fetch`.** The parser is thirty lines. Rejected as a
  dependency to understand for thirty lines.

## Consequences

Verified against the real daemon: a plugin scan streamed its progress, the scan button
disabled while it ran, the final message reached the status bar, and the list and the open
inspector reloaded afterwards.

If the window is closed and reopened during a scan, the scan keeps running in the daemon
but the new window shows nothing of it and can start no second one (it gets a 409). The
daemon does not force this: the client simply does not read `scan-status` on startup.
Polling it would show the running scan's progress with no daemon change.

The parser handles `data:` lines only: no event names, ids or retry fields, because the
daemon sends none. A change to the daemon's stream format has to change `sse()` with it.

## Notes

The sample check (ADR-0041) uses the same shape; `Api.scan` takes the two kinds that
exist today and widens when that view is built.
