# 0002. Keep the hand-rolled worker pool for parsing

- **Status:** Reaffirmed — considered and deliberately not changed
- **Decided:** ~2024. No commit exists; the deliberation produced no code change
- **Recorded:** 2026-09-15
- **Confidence:** remembered. No git evidence exists by nature — this is a decision *not*
  to act, and version control cannot see those

## Context

`src/scan/parallel.rs` is a pool built from `std::sync::mpsc` plus
`Arc<Mutex<Receiver<PathBuf>>>` as a shared work queue, with threads spawned by hand and
joined in `Drop`. Results stream back over a second channel so the caller can report
progress as they arrive.

It looks like something that predates the author knowing about `rayon`. The maintainer
spent several days revisiting it, and concluded it was already right — that the
available "improvements" were either scope creep, irrelevant to what it does, or fixes
for problems it does not have in practice.

That conclusion produced no diff, which means it was invisible to every form of
archaeology and would have been re-litigated by the next reader. Hence this record.

## Decision

Keep it. Do not replace the primitives.

## Rejected alternatives

- **`rayon`.** Built for data parallelism over collections. Here there are at most four
  coarse tasks, and results must *stream* back so progress can be reported per file.
  Adopting it would reshape the result flow to gain nothing measurable.
- **`crossbeam-channel` MPMC**, removing the `Arc<Mutex<Receiver>>`. Genuinely cleaner
  on paper. Irrelevant in practice: the lock is held for a channel `recv()` — microseconds
  — against hundreds of milliseconds of parsing per file. Contention is on the order of
  1:100,000. This is the clearest case of solving a problem the code does not have.
- **`tokio` / async.** Wrong tool. The workload is CPU-bound XML parsing, not IO
  concurrency.
- **Work-stealing or dynamic scheduling.** Thread count is `(total/2).max(1).min(4)` and
  tasks are uniformly expensive. Nothing to steal.

## Consequences

No dependency added, no restructuring, and the streaming-progress shape is preserved.

The cost is that the pool is bespoke and therefore not covered by anyone else's tests —
which is exactly how the defect below survived.

## Notes

The scope of this ADR is the *design*. It is not a claim that the implementation was
correct, and it was not.

On 2026-09-15 the worker loop was found to be:

```rust
while let Ok(path) = work_rx.lock().unwrap().recv() {
    worker.process_file(path);   // guard still held here
}
```

Temporaries in the scrutinee of a `while let` (or `match`) live until the end of the
block, so the `MutexGuard` survived the body. Every worker held the queue lock for its
entire parse: the pool spawned up to four threads and ran them strictly one at a time,
for the whole life of the code, on both scan paths. Rust 2024's `if let` rescoping does
not extend to `while let`.

Measured on the pattern in isolation — 8 items × 80 ms across 4 threads:
643 ms and max 1 concurrent worker, against 161 ms and max 4 once the guard is dropped
before the body.

The fix was to bind the item first, extracted into `drain()` so the invariant could be
tested. `worker_loop_runs_bodies_concurrently` asserts observed concurrency and was
verified to fail against the old loop with `max concurrent = 1`.

Two things are worth carrying forward. First, none of the days of architectural
deliberation could have surfaced this: the argument was about which concurrency
primitive to use, never about temporary scope. **An ADR records that a decision was
examined, not that an implementation is faithful to it** — those need different
mechanisms, and the mechanism here is the test. Second, the existing tests checked that
results came back correctly, never that they came back in parallel, so a total loss of
parallelism was invisible.
