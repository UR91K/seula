# 0022. Consolidate project insertion onto BatchInsertManager

- **Status:** Accepted
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** `src/database/batch.rs` (`BatchInsertManager`, unchanged — accepts any
  `Vec<Project>` length including 1); `src/cli/commands/scan.rs` (`store_results`);
  `src/grpc/handlers/system.rs` (`add_single_project`, `add_multiple_projects`);
  ADR-0005, ADR-0009 (why batch insert resolves plugin references the way it does);
  ADR-0018 (the service-layer audit this was found during)

## Context

Three different code paths inserted projects into the database:

1. The default scan flow (`process_projects_with_progress`, `src/lib.rs`), shared by
   CLI (no explicit paths) and gRPC (`ScanDirectories`), builds every parsed project
   into one `Vec<Project>` and inserts it in a single `BatchInsertManager` transaction
   — the mechanism ADR-0005/ADR-0009 designed plugin-reference resolution around.
2. The CLI's "legacy" explicit-path scan (`ScanCommand`, when paths are given directly)
   parsed with `ParallelParser` and then called
   `ProjectDatabase::insert_project` once per project, sequentially, holding one DB
   lock for the whole loop.
3. gRPC's `add_single_project`/`add_multiple_projects` parsed with plain `Project::new`
   and also called `insert_project` once per project, but `add_multiple_projects`
   re-acquired the DB lock inside the loop on every iteration — letting other database
   operations interleave mid-batch, unlike path 2.

Three insertion mechanisms for what should be one operation, found while auditing
gRPC-vs-CLI divergence (ADR-0018). Whether the sequential `insert_project` path
resolved plugin references identically to the batch path was not something the codebase
guaranteed — it only worked by both paths independently doing the right thing, not by
sharing the mechanism ADR-0005/0009 specified.

## Decision

Both non-default paths now build their parsed projects into a `Vec<Project>` and
insert them via a single `BatchInsertManager` transaction, same as the default flow.
`BatchInsertManager` needed no changes — it already accepts any length, including one
project (`src/database/batch.rs:480-483`), so no special-casing was needed for
`add_single_project`.

`ProjectDatabase::insert_project` itself is untouched — it's still a valid single-project
insert method, and remains in active use by test setup code across the test suite.

## Rejected alternatives

- **Keep the sequential per-project loops, just make them consistent with each other.**
  Rejected: would still leave two insertion mechanisms (batch vs. sequential) instead of
  one, and wouldn't give the non-default paths the plugin-reference-resolution
  guarantees ADR-0005/0009 specified for batch insert.

## Consequences

A database-level failure now fails the whole batch for these two paths, not just the
one project that triggered it — previously each project in the CLI's legacy scan and
gRPC's `add_multiple_projects` succeeded or failed independently. This is a real
behavior change, but it brings these paths in line with the default scan flow, which
already had this all-or-nothing property and was the majority code path. Both call
sites still report per-file parse failures independently (parsing happens before the
batch insert, one file at a time) — only the database-insert phase becomes atomic.

`add_multiple_projects` no longer re-acquires the DB lock per project; it holds one
lock for the batch insert plus the post-insert re-fetch/proto-conversion loop,
removing the interleaving window path 3 had.

## Notes

This did not verify whether `insert_project`'s plugin-reference resolution actually
differed from `BatchInsertManager`'s in any observable way before this change — only
that nothing guaranteed they matched. If a discrepancy is later found in `insert_project`
itself, ADR-0005/0009 are the reference for what it should do.
