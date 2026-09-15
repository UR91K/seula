# 0008. Keep plugin discovery in the parent process

- **Status:** Accepted
- **Decided:** 2026-08-19 (`b261200`)
- **Recorded:** 2026-09-15
- **Confidence:** decided now
- **Evidence:** `src/scan/plugins/discovery.rs`; `crates/vst-meta/src/main.rs`

## Context

ADR-0004 moves plugin *loading* into a disposable subprocess. Walking the plugin
directories to find candidates could reasonably live on either side — the superseded
`.kiro` spec put it in the worker.

## Decision

Discovery runs in the main process. The worker only ever receives explicit paths, on
stdin.

Two reasons. Walking directories is ordinary, safe file IO — nothing about it risks the
process, so there is no isolation argument for exiling it. And the supervisor *needs*
the full candidate list up front: resuming past a plugin that killed the worker means
knowing which paths come after it. If the worker owned discovery, that list would die
with it.

Paths go over stdin rather than argv so a large library cannot exceed the command-line
length limit.

## Rejected alternatives

- **Discovery in the worker** (the `.kiro` spec's design). The supervisor would have to
  re-run discovery after every crash and diff the results to work out where to resume.
- **Discovery in the worker, streamed back to the parent first.** Achievable, but it
  makes a crash mid-enumeration a case to handle for no benefit over just doing it in
  the parent.

## Consequences

Crash recovery is a simple cursor into a `Vec<PathBuf>` the parent owns.

Discovery rules live in the main crate, so its behaviour — VST3 bundles treated as a
single candidate rather than descended into, a depth cap, deduplication of overlapping
roots — is unit-testable without spawning anything. `discovery.rs` has five such tests.

## Notes

A `.vst3` "bundle" is a directory that *is* the plugin. `WalkDir::filter_entry` cannot
express "yield this directory but do not descend into it" — a `false` predicate drops
the entry entirely — so discovery drives the iterator manually and calls
`skip_current_dir()` after recording the bundle. Getting this wrong yields either zero
candidates or two per bundle; `treats_a_vst3_bundle_as_one_candidate` pins it.
