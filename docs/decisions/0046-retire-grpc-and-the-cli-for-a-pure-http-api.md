# 0046. Retire the gRPC server and the CLI, interactive mode included; HTTP is the only surface

- **Status:** Accepted, not yet implemented
- **Recognized:** 2026-09-16, recorded then as the tentative direction in ADR-0028
- **Decided:** 2026-09-24
- **Recorded:** 2026-09-24
- **Confidence:** decided now
- **Evidence:** ADR-0028 (the direction this settles, and the history of why gRPC and the
  CLI exist); ADR-0018 (the service layer that makes removal mechanical); ADR-0024 (the
  HTTP surface, complete for every domain per `docs/status.md`); `tests/grpc/` (behaviour
  tested only through gRPC); `mockup/data/generate.py` (starts `seula --config <path>
  --server`)

## Context

ADR-0028 recorded, as a direction and not a decision, that the gRPC server and the whole
CLI would go once the HTTP surface reached parity. Neither was built because it was the
right architecture. gRPC was built for an Avalonia front end that never landed, and the
CLI and its interactive mode were built to have something to drive the program with
while no GUI existed.

The HTTP surface now covers every `Services` domain plus `system`, including the SSE
streams. The frontend is committed to it (ADR-0048). Each service-layer change currently
ripples to three adapters, which ADR-0024 accepted as a cost while the other two still
had users. They no longer do.

## Decision

**The gRPC server, the CLI subcommands and interactive mode are all removed.** That is
`src/grpc/`, `proto/services/` and the tonic and prost dependencies, and every clap
subcommand in `src/cli/` along with the rustyline prompt. Nothing is kept as a thin
HTTP-client CLI, as ADR-0028 already settled.

**The binary keeps its process flags.** `--config` and `--server` are how the daemon is
started, not CLI commands, and the mock data generator depends on both. With no
subcommands left, `seula` runs the tray daemon and `seula --server` runs it without the
tray icon.

**One adapter, one aggregator.** `SystemService` folds into `Services`, closing the gap
ADR-0024 recorded. With gRPC gone there is nothing else that constructs it separately.

**Tests move before code goes.** Behaviour that is tested only through gRPC (`tests/grpc/`,
including the regression test `no_scan_starts_while_one_is_running`) is re-expressed
against the service layer or the HTTP router first. A gRPC test is deleted only when
what it checks is covered elsewhere.

## Rejected alternatives

- **Keep the CLI as a thin HTTP client.** Rejected in ADR-0028 and not reopened: it would
  be a second client to keep in step with the DTOs, for a single user who will have a GUI.
- **Keep gRPC dormant, unregistered but compiled.** Rejected because dead code still costs
  the proto, the DTO conversions and a third copy of every signature change, which is the
  exact cost this removes.
- **Wait longer for the frontend to prove the HTTP surface.** Rejected because ADR-0028's
  condition was parity, and parity is reached. Waiting would mean more HTTP work done
  under the three-adapter tax.

## Consequences

Every service-layer change touches one adapter. The wire format has two mirrors instead
of three: the HTTP DTO and its TypeScript type.

What is lost:

- **Scripting.** No `seula plugin refresh` in a terminal or a script. The equivalent is a
  `curl` against the HTTP route, which ADR-0038 and ADR-0041 already made the same path.
- **Debugging without a UI.** The CLI's table, JSON and CSV output (`src/cli/output.rs`,
  ADR-0003) goes with it. Until the frontend exists, `curl` and the browser's devtools
  are the way in.
- **A body of tests.** Rewriting `tests/grpc/` is real work, and it is the part most
  likely to be skipped. It must not be.

ADR-0003 (terminal-aware table formatting) describes code this removes.
`docs/architecture/overview.md` describes both surfaces and needs rewriting when they go.

## Notes

The order is: tests first, then gRPC, then the CLI subcommands, then the aggregator fold.
Each step leaves the build green. Removing gRPC before the CLI keeps the diff that
touches the service layer's signatures to one adapter.

Reversal is expensive once done, and the history is in git. Nothing here is a reason to
hesitate, since both surfaces exist only because earlier GUI attempts failed.
