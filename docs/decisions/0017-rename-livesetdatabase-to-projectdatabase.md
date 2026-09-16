# 0017. Rename `LiveSetDatabase` to `ProjectDatabase`

- **Status:** Accepted
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** `77bdb2c` (`feat(project)!: generalize the project model for future DAW
  support`, generalized the model but not this struct); ADR-0014 (DAW generalisation
  phasing); ADR-0015 (generic core vs. DAW-specific side tables); ADR-0016 (generic
  `Project`/`AnyDawParser` dispatch)

## Context

`LiveSetDatabase` is the struct owning the SQLite connection and every entity module
under `src/database/` (projects, tags, collections, tasks, plugins, media, and more) —
not just project storage. "Live Set" is Ableton's own name for a project file (`.als`),
so the name asserted an Ableton-only scope for what is, and has always been, the
whole persistence layer.

`77bdb2c` generalized the stored *model* to a DAW-agnostic `Project`, per the direction
already committed to in ADR-0014 (DAW generalisation is underway, `vst-meta` is Phase 0)
and ADR-0015 (generic core columns, DAW-specific data in side tables). That commit did
not rename the database struct alongside the model, leaving the code asserting two
different things at once: the schema and model say "generic, multi-DAW eventually," the
struct name says "this is Ableton's."

This surfaced while scoping a shared service-layer refactor (gRPC/CLI/future-axum
consolidation, unrelated to DAW generalisation itself) — a new layer of services
wrapping a struct named `LiveSetDatabase` would have baked the same stale assumption
into new code, one layer further from where it was introduced.

## Decision

Rename `LiveSetDatabase` to `ProjectDatabase` throughout the codebase. Purely
mechanical: no behavior, schema, or API change. Brings the struct's name in line with
the generic `Project` model it already stores, and with the direction ADR-0014/0015/0016
already committed to.

## Rejected alternatives

- **Leave it as `LiveSetDatabase`.** Rejected: the name would keep contradicting the
  schema and model it wraps, and the contradiction compounds every time new code (e.g.
  the planned service layer) is built on top of it under the stale name.
- **Defer the rename until Phase 1 (ADR-0014) DAW-generalisation work actually lands.**
  Rejected: the rename is free (no behavior change, low blast radius, mechanical) and
  waiting only means more call sites accrue under the wrong name in the meantime,
  including the service-layer work this was found while scoping.

## Consequences

No behavior change. Every reference to `LiveSetDatabase` in `src/`, tests, and docs
should be updated to `ProjectDatabase`; grepping for the old name should return nothing
once this lands. Future work (the service-layer extraction, DAW-generalisation Phase 1)
builds on the correctly-scoped name instead of perpetuating the old one.
