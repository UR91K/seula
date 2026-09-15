# 0011. Recreate the whole database rather than migrate the plugin schema

- **Status:** Accepted — not yet implemented
- **Decided:** 2026-09-15
- **Recorded:** 2026-09-15
- **Confidence:** decided now
- **Evidence:** no migration mechanism exists — `src/database/core.rs` is
  `CREATE TABLE IF NOT EXISTS` throughout, and `PRAGMA user_version` is never set

## Context

ADR-0007, ADR-0009 and ADR-0010 together restructure `plugins`, add `plugin_refs`,
`plugin_classes` and `plugin_buses`, and change the key from `UNIQUE(dev_identifier)` to
`(format, uid)`. ADR-0006 drops five columns after that.

There is no migration machinery to hang this on. Every table is created with
`CREATE TABLE IF NOT EXISTS`, so an existing database simply keeps its old plugin schema
and no new column ever appears. Nothing stamps a schema version.

Three options were put to the maintainer: drop only the plugin tables and keep user
data, recreate the whole database, or stamp `user_version` and refuse to open a stale one.

## Decision

Recreate the whole database. Seula is pre-1.0 and the maintainer is currently its only
user; a rescan rebuilds everything derived from project files.

## Rejected alternatives

- **Drop only the plugin tables, keep everything else.** Was the recommendation, because
  tags, collections, tasks, notes and stored media are user-authored and unrecoverable
  by rescanning, whereas plugin rows are entirely derived. Rejected by the maintainer in
  favour of the simpler path. Note it also carries a trap: clearing plugin links is not
  enough on its own, because `filter_unchanged_projects` (`src/lib.rs:328`) skips
  projects whose hash is unchanged, so the links would never repopulate without also
  clearing stored hashes.
- **Stamp `user_version` and refuse to open a stale database.** Touches no data without
  consent, but makes every breaking change a manual step for the user, and there is no
  versioning infrastructure to build on yet.

## Consequences

Simple to implement and guarantees a clean schema with no half-migrated states.

The cost is real and worth stating plainly: **tags, collections, tasks, notes and stored
media are destroyed.** A rescan cannot rebuild them — they were never in the project
files. Anyone who has organised a library in Seula loses that organisation.

## Notes

This is the decision most likely to need revisiting, and the trigger is specific: **the
first time Seula has a user who is not the maintainer**, or the first time the
maintainer's own tags are worth more than the implementation time saved.

The eventual fix is the same either way — `PRAGMA user_version` plus ordered migration
steps. Adding it before this change lands would make this ADR unnecessary; adding it
after means one more destructive reset first.

An export of user-authored data (tags, collections, tasks, notes) before the reset would
blunt most of the cost for a fraction of the effort of real migrations, if the loss turns
out to sting.
