# 0018. Shared service layer for gRPC, CLI/TUI, and future axum

- **Status:** Accepted
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** `src/services/` (new); `src/grpc/handlers/tags.rs`, `collections.rs`,
  `projects.rs` (rewritten to call it); `src/cli/commands/mod.rs` (`CliContext` gains a
  `services: Services` field); ADR-0019 through ADR-0022 (the specific behavioral fixes
  this refactor surfaced and resolved)

## Context

`src/grpc/handlers/*.rs` and `src/cli/commands/*.rs` each held their own
`Arc<Mutex<ProjectDatabase>>` and called its methods directly — there was no layer
shared between the two surfaces. An audit comparing all 12 gRPC services against their
CLI counterparts found this had already caused real behavioral drift, not just
presentational differences: the same nominal operation (e.g. "list projects", "tag a
project") was implemented twice, independently, and had quietly diverged. See
ADR-0019 through ADR-0022 for the specific divergences found and how each was resolved.

A third caller — an axum HTTP router — is planned. Adding it as a third independent
caller of `ProjectDatabase` would mean three copies of the same business logic instead
of two, compounding the problem this was found while investigating.

## Decision

Introduce `src/services/`, one struct per entity domain (`TagsService`,
`ProjectsService`, `CollectionsService`, more to follow), each owning
`Arc<tokio::sync::Mutex<ProjectDatabase>>` plus whatever other shared state that domain
needs (e.g. `MediaStorageManager`, scan/watcher state for domains not yet migrated).
Methods are `async fn`, `.lock().await` internally, then call the existing synchronous
`ProjectDatabase` methods — matching the calling convention both surfaces already used,
so no change to the sync/blocking DB model.

A `Services` aggregator (`src/services/mod.rs`) bundles one instance of each domain
service, constructed once from `ProjectDatabase` + config + media state. gRPC handlers
and `CliContext` each hold a `Services` (or the specific service they need) instead of
a raw `Arc<Mutex<ProjectDatabase>>`; handler/command methods shrink to: validate/convert
the surface-specific request → call the service method → convert the result to that
surface's output format.

Validation and business logic (e.g. "does this tag exist before I mutate it") lives in
the service layer, not the database layer — the database layer stays thin CRUD,
consistent with how it was already written.

Added alongside this: `impl From<DatabaseError> for tonic::Status`
(`src/grpc/error.rs`), so gRPC handlers use `?` instead of the ad hoc
`Status::new(Code::Internal, format!("Database error: {}", e))` every call site used to
build by hand.

Migration is incremental, one domain at a time (tags → projects → collections →
remaining domains), each landing with its own build/test verification rather than one
large rewrite.

## Rejected alternatives

- **Trait-based dependency inversion** (services depend on a `Database` trait instead
  of the concrete `ProjectDatabase`, enabling test doubles). Rejected for now: the
  codebase has zero existing test-double seams anywhere, and the concrete-struct
  approach is simpler and consistent with this codebase's general bias against
  `dyn Trait` (see ADR-0016). Revisit if testing services directly against a real
  SQLite file becomes a real obstacle, not preemptively.
- **Leave the duplication and reconcile behavior differences independently on each
  surface.** Rejected: the divergences found (ADR-0019 through ADR-0022) are exactly
  the kind of thing that re-drifts the moment a third surface (axum) is added on top,
  and reconciling them without a shared layer just relocates the duplication instead of
  removing it.
- **Big-bang rewrite of all 12 domains at once.** Rejected in favor of incremental,
  per-domain migration: smaller diffs, each independently verifiable, and the pattern
  gets proven on a small domain (tags) before being applied everywhere.

## Consequences

Handlers and CLI commands get thinner over time as more domains migrate; the domains
not yet migrated (search, samples, tasks, plugins, media, config, system/watcher) still
hold `Arc<Mutex<ProjectDatabase>>` directly until their turn comes. This is an accepted
transitional state, not a design inconsistency to fix urgently.

The future axum router becomes a third thin adapter over the same `Services` struct
once this migration is far enough along, rather than a fourth place business logic
gets duplicated.

## Notes

Remaining domains to migrate, in no particular urgency: search, samples, tasks,
plugins, media, config, system (stats/watcher — the largest and riskiest single
handler, `src/grpc/handlers/system.rs`, deliberately not attempted in this pass beyond
the insertion-path fix in ADR-0022).
