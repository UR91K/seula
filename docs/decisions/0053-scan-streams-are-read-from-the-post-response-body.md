# 0053. A scan's stream is read from the response body of the POST that starts it

- **Status:** Proposed — implemented on branch `tauri-app`, awaiting the maintainer's review
- **Recognized:** 2026-10-04, ADR-0047's "Not decided here" note on the client layer
- **Decided:** not yet; proposed 2026-10-04
- **Recorded:** 2026-10-04
- **Confidence:** proposed by the assistant while implementing, verified against a real
  plugin scan, not reviewed
- **Evidence:** ADR-0047 (names this exact choice); ADR-0038 (one scan at a time, in the
  background); ADR-0024 (the SSE endpoints); `web/shared/api.ts` (`sse`, `Api.scan`);
  `src/http/handlers/system.rs` (`scan_directories` answers 409 when a scan runs)

## Context

The project scan, the plugin scan and the sample check each start with a `POST` whose
response is the progress stream (ADR-0024). The browser's `EventSource` makes only `GET`
requests, so it cannot read them. ADR-0047 named the two ways out: read the stream from a
`fetch` response body, or add a `GET` route to the daemon that follows a running scan.

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

**No `GET` follow route is added.** The daemon does not grow an endpoint for this.

## Rejected alternatives

- **A `GET` route that follows the running scan.** The better design for a window that
  closes and reopens mid-scan: the window could attach to whatever is running. Rejected
  for now because it changes the daemon for a case no screen needs yet, and because it
  would let a stream be read by more than one client, which nothing has designed or
  tested. This is the first thing to reach for if re-attaching is wanted.
- **`EventSource` on a `GET` that starts the scan.** A `GET` that changes state, which
  caches and prefetchers are allowed to repeat. Rejected.
- **A library for reading SSE over `fetch`.** The parser is thirty lines. Rejected as a
  dependency to understand for thirty lines.

## Consequences

Verified against the real daemon: a plugin scan streamed its progress, the scan button
disabled while it ran, the final message reached the status bar, and the list and the open
inspector reloaded afterwards.

If the window is closed and reopened during a scan, the scan keeps running in the daemon
but the new window shows nothing of it and can start no second one (it gets a 409). This is
the cost of rejecting the follow route.

The parser handles `data:` lines only: no event names, ids or retry fields, because the
daemon sends none. A change to the daemon's stream format has to change `sse()` with it.

## Notes

The sample check (ADR-0041) uses the same shape; `Api.scan` takes the two kinds that
exist today and widens when that view is built.
