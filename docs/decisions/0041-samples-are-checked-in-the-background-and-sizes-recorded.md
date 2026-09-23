# 0041. Sample files are checked in the background, by folder, and their sizes recorded

- **Status:** Accepted — implemented 2026-09-23
- **Recognized:** 2026-09-23, planning the samples board
- **Decided:** 2026-09-23
- **Recorded:** 2026-09-23
- **Confidence:** decided now
- **Evidence:** `src/database/samples.rs` (the former `refresh_sample_presence_status`,
  which called `path.exists()` for every sample while holding the database; and the size
  "estimates", a guessed 5 MB per WAV, 0.5 MB per MP3 and so on); ADR-0038 (the background
  plugin scan this follows); a run on the maintainer's library, below

## Context

Two things the samples view needs were missing or wrong.

**Size.** The status bar's "estimated total size" was invented: a fixed size per
extension, summed. It looked like a measurement and meant nothing. Nothing recorded a
sample's real size.

**Presence.** The only way to mark a sample missing after the scan that found it (see
the rescan bug in `docs/status.md`) was `POST /samples/refresh-presence-status`. It
checked every file one at a time, while holding the database mutex every service shares,
and answered only when done. On a large library on slow or network drives, that stalls
the whole app.

The maintainer asked for real sizes, and for the check to run in the background,
multithreaded, using the right I/O techniques.

## Decision

**A background check, reported like the other scans.** `POST /api/v1/samples/check`
streams progress as SSE: `checking_samples` (a new scan status, shared with project and
plugin scans, so one runs at a time, ADR-0038), then `completed` with what changed, or
`error`. The synchronous route and `seula sample check-presence` use the same check.

**The database is locked only at the ends.** The check reads the paths, releases the
database, looks at the files on a blocking thread, then locks once to write the result
in one transaction.

**By folder, not by file.** Samples cluster in folders. The check groups paths by folder
and lists each folder once. On Windows a directory listing returns every entry's size
and times without opening any file, where asking for one file's metadata opens it. A
folder holding fewer than three of the samples is checked file by file instead, because
it may hold thousands of other files. Names compare without case on Windows. A listing
that fails (a missing or unreadable folder) falls back to asking file by file.

**Several threads.** The time goes to waiting on the disk, so the check uses twice the
cores, at most 16. Each thread takes the next folder from an atomic counter. Nothing
else is shared, and progress goes back over a channel to the calling thread.

**Sizes in a table of their own.**
`sample_files (sample_id, size_bytes, modified_at, checked_at)`. A new table needs no
`SCHEMA_VERSION` bump (ADR-0011). A found file's size and time are written. A missing
sample keeps the size it last had, so it can still say how big it was. Totals count
present samples only, and the stats say how many present samples have a size
(`sized_samples`), so a library no check has measured reads as unmeasured, not as empty.

The guessed sizes are gone from every query. Sample rows carry `size_bytes`.

## Measured

On the maintainer's library, 18,243 samples in 3,510 folders: 0.47 s for the whole
check, 147 GB measured over 8,937 present samples (debug build, 2026-09-23). The file
cache was warm from an earlier run, so a first cold run will be slower. The presence it
found matched what the database already held.

## Rejected alternatives

- **Size the files during the project scan.** The parser already tests each sample's
  existence, one file at a time, once per project that uses it. Folding sizes in there
  would repeat that per project. A separate check that sees each file once is cheaper, and
  can run without a rescan.
- **A size column on `samples`.** Changing an existing table needs a schema bump, which
  discards the user's database (ADR-0011).
- **rayon for the thread pool.** An atomic counter over a vector of folders is all this
  needs, and ADR-0002 argues against adding a pool crate.
- **Keep the estimate, labelled as one.** A number that looks like a measurement but isn't
  one invites trust it does not deserve.

## Consequences

The gRPC `SampleStats.total_estimated_size_bytes` and the analytics' storage fields now
carry measured values (zero until a check has run), under their old names.

A check does not run by itself yet. The project scan still tests existence as it parses
(so a first scan finds missing samples), but sizes appear only after a check. Running
one automatically at the end of a project scan would be a small change.

## Notes

The first implementation held the database for the whole check and then deadlocked:
`db.blocking_lock().sample_paths().and_then(|p| …)` keeps the guard alive to the end of
the statement, closure included. The same trap as ADR-0002's. The check now takes the
paths in a statement of its own, and `the_background_sample_check_finishes` fails
cleanly if it ever hangs again.
