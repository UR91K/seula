# 0049. The daemon's project scan writes through the shared database handle

- **Status:** Accepted — implemented 2026-09-24
- **Recognized:** 2026-09-24, a known issue in `docs/status.md`
- **Decided:** 2026-09-24
- **Recorded:** 2026-09-24
- **Confidence:** decided now
- **Evidence:** `src/lib.rs` (`process_projects_into`, `scan_projects`);
  `src/services/system.rs` (`start_scan`); `src/main.rs` (`SharedState`);
  `src/database/core.rs` (no journal mode or busy timeout is set); ADR-0024 (one shared
  connection for every adapter); ADR-0038 (the plugin scan already locks only to write)

## Context

`SharedState` in `src/main.rs` gives the HTTP and gRPC adapters one database connection
behind an `Arc<Mutex<ProjectDatabase>>`. The project scan did not use it. It opened a
second connection and wrote the whole batch in one transaction on it, along with the
first-run plugin scan's results.

The database uses rollback journalling and rusqlite's default five-second busy timeout.
While the batch insert held SQLite's write lock, a write from an adapter (a tag, a
collection edit) waited out that timeout and failed with "database is locked".

## Decision

**In the daemon, the scan reads and writes through the shared handle.**
`process_projects_into` takes the `Mutex` and locks it only for each database step: the
first-run plugin check, persisting that scan, filtering out unchanged projects, and the
batch insert. Discovery, preprocessing, parsing and the plugin subprocess run without
the lock. No lock is held across a progress callback, because the final update can wait
for a slow client (`send_scan_update`).

An adapter request that arrives during the insert now waits on the mutex until the
insert is done, and then succeeds.

`process_projects_with_progress`, which the CLI uses, keeps a connection of its own. It
runs in a separate process, where there is no shared handle to take.

## Rejected alternatives

- **WAL with a longer busy timeout.** WAL lets readers run beside the writer, but SQLite
  still allows one writer at a time. A tag edit during the insert would still wait on
  SQLite's lock and fail when the timeout ran out. A longer timeout only moves that
  limit. It would also keep two connections, which is what `SharedState` says there
  are not.
- **Both.** WAL only matters when there is a second connection. The daemon no longer
  has one.
- **Hold the lock for the whole scan.** Simpler, but it would block every request for
  as long as a scan runs, minutes on a first run with a plugin scan.

## Consequences

- With one connection, reads wait for the insert too. Under rollback journalling they
  were mostly unaffected before, except at commit. The insert is one transaction, so
  the wait lasts as long as the insert: seconds on a first run over thousands of
  projects, much less on a rescan. If that becomes noticeable, the options are
  splitting the insert into several transactions, or WAL with a separate read
  connection.
- A CLI scan run while the daemon is up still writes through a second connection and
  can still make the daemon's writes fail. ADR-0046 removes the CLI.
- `process_projects_into` must be called from a blocking thread: it takes the lock with
  `blocking_lock`. `start_scan` runs the scan on one.
- Regression test: `a_scan_writes_through_the_database_it_is_given` (`src/lib.rs`).
