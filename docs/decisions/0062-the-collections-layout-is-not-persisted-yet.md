# 0062. The collections layout is kept in memory until the preferences endpoint exists

- **Status:** Accepted — a deliberate deferral
- **Recognized:** 2026-10-05, planning the collections view
- **Decided:** 2026-10-05, accepted by the maintainer
- **Recorded:** 2026-10-05
- **Confidence:** decided now
- **Evidence:** ADR-0031 (preferences live in the `app_state` table); the Preferences
  section of `docs/architecture/frontend.md`, which says the endpoint does not exist yet

## Context

The grid-or-table choice on the collections list should survive a restart, and ADR-0031
says where it belongs. The HTTP endpoint for reading and writing `app_state` does not
exist.

## Decision

Keep the layout in a module-level store (ADR-0055). It survives switching views and is
lost when the app closes. Do not add the endpoint as part of the collections work.

The endpoint is one backend addition that every view's preferences will want. It should be
designed once, for all of them, not as a side effect of one view.

## Consequences

The layout resets to the default (the grid) on every launch. When the endpoint is added,
this record is superseded and the store is read from and written to it.
