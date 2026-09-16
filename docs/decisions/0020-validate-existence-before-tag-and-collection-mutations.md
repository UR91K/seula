# 0020. Always validate existence before tag/collection membership mutations

- **Status:** Accepted
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** `src/services/tags.rs` (`TagsService::tag_project`/`untag_project`,
  `require_tag`/`require_project`); `src/services/collections.rs`
  (`CollectionsService::add_project_to_collection`/`remove_project_from_collection`,
  `require_collection`/`require_project`); `tests/grpc/collections.rs`
  (`test_add_project_to_nonexistent_collection`,
  `test_remove_project_from_nonexistent_collection`, updated to expect `NotFound`
  instead of `Internal`); ADR-0018 (the service-layer audit this was found during)

## Context

`ProjectDatabase::tag_project`/`untag_project` (`src/database/tags.rs`) and
`add_project_to_collection` (`src/database/collections.rs`) did no existence checks
before mutating: `tag_project` is a bare `INSERT OR IGNORE`, and
`add_project_to_collection` only *debug-logs* whether the project exists and never
checks the collection at all. gRPC's handlers called these directly. The CLI's
equivalents (`src/cli/commands/tag.rs`, `collection.rs`) happened to pre-check
existence and return a clean `NotFound` — not because anyone decided validation
belonged there, but because the CLI needed the tag/collection's name for display and
fetched it first as a side effect.

In practice, a gRPC call against a nonexistent tag or collection didn't silently
succeed as the initial read of the code suggested it might — SQLite's
`PRAGMA foreign_keys = ON` (`src/database/core.rs:118`) meant a truly nonexistent
foreign key raised a constraint error. But that error surfaced as an opaque
`Code::Internal` ("SQLite error: FOREIGN KEY constraint failed"), not a meaningful
`Code::NotFound` — a worse outcome than either "clean error" or "silent success",
and a moving target that only held because FK constraints happened to be on.

## Decision

Validate that both sides of a membership mutation exist before mutating, in the
service layer, returning `DatabaseError::NotFound` with a clear message otherwise.
Applied to `TagsService::tag_project`/`untag_project` (project and tag) and
`CollectionsService::add_project_to_collection` (collection and project) /
`remove_project_from_collection` (collection only — the underlying query already
naturally errors if the project isn't a member). The raw `ProjectDatabase` methods are
unchanged; validation is service-layer business logic, not database-layer CRUD.

## Rejected alternatives

- **Add the checks directly in the database methods instead.** Rejected: the database
  layer is deliberately thin CRUD throughout this codebase (see ADR-0018's framing);
  business-rule validation belongs in the layer being introduced for exactly that
  purpose.
- **Leave it relying on the FK constraint.** Rejected: it produced the wrong status
  code (`Internal` instead of `NotFound`) for gRPC clients, gave no equivalent
  protection at all for methods not backed by an FK-constrained column, and depended on
  a pragma setting elsewhere in the codebase rather than being a decided behavior.

## Consequences

Two existing gRPC tests
(`test_add_project_to_nonexistent_collection`,
`test_remove_project_from_nonexistent_collection`) asserted `Code::Internal` for this
case — that was pinning the FK-constraint side effect, not a decided contract. Updated
to assert `Code::NotFound`, which is what the fix is for.

Batch variants (`batch_tag_projects`, `batch_untag_projects`,
`batch_add_projects_to_collection`) are unchanged — they already return a per-item
`Result` and the database layer's existing per-item existence check
(`batch_add_projects_to_collection`) or FK behavior covers them without the same
"looks silent, isn't really" gap a single bare mutation had.

## Notes

`ProjectDatabase::remove_tag` and `delete_collection` still silently no-op against a
nonexistent id (bare `DELETE`, no rows-affected check). Not changed here — those are
resource deletes, not membership mutations, and idempotent-delete semantics (deleting
something already gone reports success) is a defensible default rather than the same
bug. Revisit only if a concrete case shows it isn't.
