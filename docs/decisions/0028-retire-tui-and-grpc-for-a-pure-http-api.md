# 0028. Retire the CLI (interactive and non-interactive) and the gRPC server, leaving a pure HTTP API

- **Status:** Proposed — tentative, not yet decided, may be superseded
- **Recognized:** 2026-09-16, in conversation, immediately after ADR-0024's skeleton landed
- **Decided:** not yet
- **Recorded:** 2026-09-16
- **Confidence:** stated as a direction, not decided — this is a marker, not a record
- **Evidence:** ADR-0024 (the HTTP adapter this direction depends on existing first);
  ADR-0018 (the service layer that makes this mechanical rather than a logic rewrite);
  `src/grpc/` (12 handlers that would be removed); `src/cli/` (the whole module, clap
  commands and interactive mode alike, that would be removed)

## Context

None of the adapters this ADR proposes retiring were built because they were the right
architecture. They were built in sequence because a UI never landed:

1. The project started in Python; too slow.
2. Tried Tauri; at the time, too bloated and too slow to open.
3. Tried `iced`; frustrating to work with.
4. Built the gRPC server instead, reasoning it was the most efficient
   same-machine IPC mechanism, specifically to pair with a planned Avalonia front end;
   Avalonia turned out to be too much of a pain.
5. Built the CLI and interactive TUI mode just to have *something* to drive the program
   with while testing, since no GUI had landed.

ADR-0018 then built `src/services/` because the gRPC and CLI adapters, born from that
same desperation rather than a plan, had duplicated logic and drifted. ADR-0024 added
HTTP as a third adapter over that same layer, for two consumers — a Tauri app and a
browser — that actually exist now.

HTTP is not "one more adapter" in that lineage — it's the maintainer giving up on finding
a native Rust GUI story and switching to a React front end in a browser (or any other
HTTP-speaking client), which is universal in a way none of the earlier attempts were.
Performance is not a concern in making this switch: the program is I/O-bound, so an HTTP
round-trip costs nothing gRPC was actually buying.

The maintainer's stated long-term direction, as of this conversation, is to finish that
switch: once the HTTP surface is functionally complete, retire the gRPC server and the
CLI entirely — interactive mode and non-interactive `clap` commands alike — rather than
carry three adapters that only exist because earlier GUI attempts failed. The described
end state is a pure Axum API application, serving a web frontend and/or Tauri, with no
gRPC server and no CLI at all.

This is explicitly not committed. The maintainer's own framing: "that's the long-term plan
for now at least, might get superseded." It is recorded here, empty, so that HTTP
build-out work already in flight (ADR-0024's remaining domains) can be done in a way that
keeps this retirement mechanical if it happens — e.g. not growing gRPC-only or
CLI-only conveniences, not deepening the `Services`-aggregator gap ADR-0024 already
flagged (`SystemService` not being a member) in a way that only gRPC papers over.

## Decision

None yet. If and when this is decided, it needs its own pass through this document with:

- what specifically happens to gRPC (`src/grpc/`, `proto/services/`, the tonic/prost
  dependencies) — presumably straightforward removal, since HTTP would by then cover the
  same `Services` surface
- the CLI binary is removed in full, not just its interactive mode — confirmed by the
  maintainer. Nothing keeps a thin HTTP-client CLI around; the end state has no CLI at
  all, only the HTTP server and its consumers (web frontend, Tauri)
- what happens to `AppState`'s `Services` + `SystemService` split (ADR-0024's
  Consequences section already names this as incomplete) — with only one adapter left,
  there's no more reason not to fold `SystemService` into `Services` properly
- `grpc_port` config, tray integration, and the gRPC half of `build_shared_state()` in
  `main.rs`

## Rejected alternatives

None yet — no decision has been weighed, so nothing has lost.

## Consequences

None yet — recorded as an open direction, not an action taken.

## Notes

This ADR deliberately uses a `Status` value ("Proposed") outside this project's normal
vocabulary (`Accepted | Accepted (retrospective) | Reaffirmed | Incidental | Superseded by
NNNN`), because none of those fit a stated-but-unresolved future direction, and inventing
a false "Accepted" here would misrepresent something the maintainer explicitly called
tentative.

When this firms up or is abandoned, replace this file's content rather than editing around
it: either a real `Accepted` ADR with genuine Decision / Rejected alternatives /
Consequences sections, or a one-line `Superseded` if the plan changes before anything is
built against it. Until one of those happens, nothing should treat this file as
authorization to start removing gRPC or CLI code.

Scope is now settled on one axis: the CLI in full, not just its interactive mode. What
remains open is *when* — the direction still waits on the HTTP surface reaching parity
first.
