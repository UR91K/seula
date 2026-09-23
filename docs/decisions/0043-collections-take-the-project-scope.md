# 0043. A collection's projects are counted and listed in the project scope, active by default

- **Status:** Accepted, implemented 2026-09-24
- **Recognized:** 2026-09-24, planning the collections board
- **Decided:** 2026-09-24
- **Recorded:** 2026-09-24
- **Confidence:** decided now
- **Evidence:** `src/database/collections.rs` (every count, length and list over
  `collection_projects` joined all projects, archived or not, and
  `get_collection_projects` marked every project `is_active: true`); ADR-0040, which
  said collections "have their own rules for archived projects", though none had been
  written down; ADR-0019

## Context

ADR-0040 gave the plugin and sample routes a `scope` so that a count and the list it
counts agree, and so that whether archived projects count is a choice rather than an
accident. It left collections out. Their counts, lengths, statistics, task lists and
project lists all included archived projects. The projects view hides those projects by
default (ADR-0019), so a collection's card would say 14 projects over a tracklist the
projects view shows as 12.

Asked while planning the collections board whether collections should follow ADR-0040
or count everything, the maintainer chose ADR-0040.

## Decision

**The same `scope` parameter, `active` or `all`, default `active`, on every collection
route that counts or lists its projects:**

- the list and search (`project_count`, `total_duration_seconds`, `project_ids`, and the
  `project_count` and `total_duration` sorts);
- the single collection, `GET /collections/:id`;
- the tracklist, `/collections/:id/projects`, where each project carries its real
  `is_active` so that `all` can mark the archived ones;
- the consolidated tasks, `/collections/:id/tasks`;
- the statistics, `/collections/:id/statistics` (every figure, from the count to the
  most common key).

It is the same preference as ADR-0040's, saved with the others (ADR-0031), not a
second one.

**Reordering takes the scope too.** `PUT /collections/:id/reorder?scope=active` takes
the order of the active projects, which is the list the caller was shown. The archived
members keep their slots, and the active ones fill the remaining slots in the new order,
so unarchiving a project puts it back where it was. Under `all` the order must name
every member, as before.

Membership itself is not scoped. Adding, removing and deleting work on the collection
as stored, and archiving a project does not take it out of any collection.

## Rejected alternatives

- **Count everything.** A collection is a list the user made, and an archived project
  is still on it. That is true, and it is why membership is not scoped and `all` exists.
  But the default view of a collection should agree with the default view of projects.
- **Leave archived projects at the end of a reorder.** Simpler, but a project that is
  archived and then unarchived would come back at the end of the tracklist instead of in
  its old place.

## Consequences

The CLI passes the default, so `seula collection` output now leaves archived projects
out. The gRPC handlers pass `all` and keep their old behaviour, because gRPC is being
retired (ADR-0028) and one of its tests relies on the old counts.

The system statistics' "largest collection" still counts everything. How statistics
treat archived projects is for the stats board to decide.

Two defects were fixed along the way and are in `docs/status.md`: sorting the list by
`project_count` failed with an SQL error, and every project in a collection came back as
active.
