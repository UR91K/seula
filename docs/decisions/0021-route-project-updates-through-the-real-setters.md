# 0021. Route project name/notes updates through set_project_name/set_project_notes

- **Status:** Accepted
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** `src/database/notes.rs` (`set_project_notes`/`set_project_name`, now
  bump `modified_at`); `src/services/project.rs` (`ProjectsService::update_project`);
  `src/cli/commands/project.rs` (`update_project`, no longer touches `ctx.db.conn`
  directly); ADR-0018 (the service-layer audit this was found during)

## Context

The CLI's `project update` command ran raw SQL directly against `ctx.db.conn`
(`UPDATE projects SET name = ?, notes = ?, modified_at = ? WHERE id = ?`),
bypassing `ProjectDatabase::set_project_notes`/`set_project_name`
(`src/database/notes.rs`) entirely. gRPC's `UpdateProjectNotes`/`UpdateProjectName`
handlers called the real setters. The CLI's raw SQL also set `modified_at`, which the
real setters did not — so a project edited via gRPC didn't get its `modified_at`
bumped, but the same edit via CLI did.

This worked today only because search-index sync is driven by a SQL trigger on
`projects` (`projects_au`, `src/database/core.rs:385`), which fires on any `UPDATE
projects` regardless of which code path issued it. But any future validation or logic
added only to `set_project_notes`/`set_project_name` would silently not apply to the
CLI's path, and the `modified_at` inconsistency was already live.

## Decision

Route the CLI's `project update` through `ProjectsService::update_project`, which
calls `set_project_name`/`set_project_notes` — the same methods gRPC's handlers now
call via the same service — instead of raw SQL. Both setters gained `modified_at`
bumping (`src/database/notes.rs`), so both surfaces get it uniformly rather than only
the CLI's now-removed bypass.

## Rejected alternatives

- **Add `modified_at` bumping only to the CLI's raw SQL, leave the bypass in place.**
  Rejected: fixes the immediate inconsistency but leaves the actual problem (a second,
  divergent code path for the same mutation) in place for the next thing that needs to
  change in `set_project_notes`/`set_project_name`.

## Consequences

`project update --name X` and `project update --notes Y` from the CLI now run as two
separate transactions (one per setter) when both are provided, rather than the raw
SQL's single combined `UPDATE`. Not atomic across both fields as a result — acceptable
since gRPC's `UpdateProjectName`/`UpdateProjectNotes` are already two separate RPCs
with the same non-atomicity, so this doesn't introduce a new inconsistency, it matches
the existing one.

`modified_at` now updates on every name/notes edit regardless of surface, where before
only the CLI's bypassed path did it at all.
