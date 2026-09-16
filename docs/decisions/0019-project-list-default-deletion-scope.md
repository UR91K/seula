# 0019. Project listing defaults to active-only across gRPC and CLI

- **Status:** Accepted
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** `proto/services/projects.proto` (`DeletionScope` enum,
  `GetProjectsRequest.deletion_scope`); `src/services/project.rs`
  (`DeletionScope`, `ProjectsService::list_projects`); `src/database/projects.rs`
  (`get_projects_with_filters` gained an `is_active` parameter); ADR-0018 (the
  service-layer audit this was found during)

## Context

gRPC's `GetProjects` with no filter fields set called
`get_all_projects_with_status(None)` — `None` meaning "all", so the unfiltered
default returned active **and** deleted projects. The CLI's default
(`project list` with no `--deleted` flag) called `get_all_projects_with_status(Some(true))`
— active only — and had no way to request "everything." The proto request had no
`is_active`/`deleted` field at all, so a gRPC client couldn't opt into active-only
either. Separately, `get_projects_with_filters` (used whenever any other filter field
was set) hardcoded `is_active = true` in its `WHERE` clause regardless of what the
caller wanted, so filtered and unfiltered requests didn't even agree with each other.

Three different behaviors for what was meant to be one operation, discovered while
auditing gRPC-vs-CLI divergence ahead of building a shared service layer (ADR-0018).

## Decision

Active-only default, with an explicit way to ask for deleted-only or everything: a
`DeletionScope` enum (`ActiveOnly | DeletedOnly | All`), defaulting to `ActiveOnly`
when unspecified. Both `ProjectsService::list_projects`'s two code paths (the
plain `get_all_projects_with_status` call and the filtered
`get_projects_with_filters` call) now take and honor the same scope, so filtered and
unfiltered requests agree.

`GetProjectsRequest` gained a `deletion_scope` field (proto3 enum, zero value =
`DELETION_SCOPE_ACTIVE_ONLY`, so an old client that never sets it keeps getting
active-only — the CLI's prior behavior, not gRPC's). `get_projects_with_filters`
gained an `is_active: Option<bool>` parameter, replacing its hardcoded
`is_active = true` condition.

## Rejected alternatives

- **Two independent boolean flags** (`include_active`, `include_deleted`), requiring at
  least one true. More explicit at the call site and closer to how a database query
  would naturally be shaped, but every caller (CLI flags, gRPC clients, the future axum
  handlers) would need to remember to set at least one or hit a runtime validation
  error for a case (`ActiveOnly`) that should just be the default. Rejected in favor of
  a tri-state enum with a sane default — the interface-ergonomics side of the trade-off
  won over the marginally more explicit code shape.
- **Default to `All`** (gRPC's old behavior). Rejected: a human listing projects almost
  always wants active ones; deleted projects showing up unasked-for in a plain "list
  projects" call is the more surprising default, and it's what the CLI already got
  right by accident.

## Consequences

This changes `GetProjects`' observable default behavior for existing gRPC clients: a
client that relied on the old "everything" default now gets active-only unless it sets
`deletion_scope = DELETION_SCOPE_ALL`. Acceptable per the project's pre-release alpha
status (no compatibility guarantee yet, same reasoning as ADR-0015).

`GetProjectsByDeletionStatus` (a separate, pre-existing RPC with its own `bool
is_deleted`) is unaffected — it already had explicit, unambiguous semantics and wasn't
part of the divergence.

## Notes

CLI's `project list --deleted` maps to `DeletedOnly`; there's currently no CLI flag
for `All`. Not added here — no CLI use case surfaced for it yet, and it's a small
addition whenever one does.
