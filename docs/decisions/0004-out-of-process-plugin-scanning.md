# 0004. Scan plugins in a supervised subprocess

- **Status:** Accepted
- **Decided:** 2026-08-19 (`b261200`)
- **Recorded:** 2026-09-15
- **Confidence:** decided now
- **Evidence:** `crates/vst-meta/`, `src/scan/plugins/`, `tests/plugin_scanner_tests.rs`

## Context

Reading plugin metadata requires loading the plugin's own binary — and for VST2,
actually *instantiating* it, because unlike VST3 there is no manifest to read. That
means executing arbitrary third-party native code inside whatever process does the scan.

That code, in practice:

- segfaults during init (an access violation is not a Rust panic; `catch_unwind` cannot
  see it)
- hangs on licence checks and splash screens (a blocked thread cannot be safely killed
  from inside — `TerminateThread` abandons locks and corrupts the heap)
- calls `exit()` or `abort()` on licence failure
- opens modal dialogs, invisibly, in a tray application
- **frequently cannot be unloaded cleanly** — crashes in `DllMain(DETACH)`, or leaves
  threads and COM apartments running after the handle is dropped

The last one decides it. Even a *fully successful* scan leaves the process dirtier than
it found it, and a library of 800 plugins accumulates 800 plugins' worth of loaded
libraries, threads, device handles and GDI objects. This is not a problem better error
handling can reach.

Every DAW that scans plugins does it out of process — Live, Bitwig, Cubase, Reaper's
optional mode.

## Decision

The scanner is a separate binary, `crates/vst-meta`, spawned and supervised by
`src/scan/plugins/spawner.rs`.

One worker per *batch*, not per plugin: process startup plus VST3 runtime init costs
20–40 ms on Windows, which is half a minute of pure overhead across a thousand-plugin
library. Crash isolation is recovered by protocol instead. The worker emits NDJSON and
announces each path **before** touching it:

```jsonc
{"event":"begin","path":"...\\Serum.vst3"}   // before the load is attempted
{"event":"result","path":"...","status":"success","plugins":[ ... ]}
{"event":"done","scanned":42}
```

A `begin` with no matching `result` names the plugin that killed the worker, so the
supervisor records the failure against that exact path and restarts from the next one.
Without that line a crash says only that the batch died.

The crate is split by a `host` feature so the parent can share the wire types with
`default-features = false` and never link a VST host at all.

## Rejected alternatives

- **In-process with thorough error handling.** Cannot catch access violations, cannot
  time out a blocked thread, cannot survive `exit()`, and does nothing about unload
  residue. See Context.
- **One process per plugin.** Maximum isolation, but 20–40 ms × N of pure startup, plus
  full runtime teardown each time.
- **Finish `crates/vst-parser`** (the earlier scaffold). Its loaders were never
  implemented, its dependency picks had already been rejected empirically in favour of
  `vst` 0.4 + `vst3-host` 0.9, and it pulled `tokio` with `features = ["full"]` into a
  process whose entire job is blocking `dlopen`. Deleted.
- **Keep depending on Ableton's plugin database.** See ADR-0006.

## Consequences

A plugin that crashes or hangs costs one restart and produces a precise `Crashed` or
`Timeout` record instead of taking the application down. Verified against a real
library: 274 candidates, 257 scanned, 17 classified failures.

Metadata is now far richer than Ableton's database ever held — bus layouts, channel
counts, latency, GUI presence, vendor contact, factory classes — and no longer depends
on Ableton having scanned recently.

The costs: a second binary to build and ship, which must be found at runtime (sidecar
next to the executable, `SEULA_VST_META_PATH` to override in development); and
supervision logic whose recovery paths no healthy machine exercises. That last one is
covered by `examples/stub_vst_worker.rs`, a worker that crashes, hangs or exits silently
on demand, driven by `tests/plugin_scanner_tests.rs`.

Writing those tests found a real hole: a worker dying *before* its first `begin` was
read as a clean finish, silently dropping every remaining candidate. A stall with
nothing in flight is now blamed on the next path so the scan always moves forward.

## Notes

**Never enable the `host` feature in the root `Cargo.toml`.** That is the line between
"this process can be killed by a plugin" and "it cannot".

Discovery deliberately stays in the parent — see ADR-0008.

Windows specifics that only make sense in a disposable process: the worker sets
`SEM_FAILCRITICALERRORS | SEM_NOGPFAULTERRORBOX` so a crash fails fast instead of
blocking on an error dialog, and the supervisor spawns with `CREATE_NO_WINDOW`.
