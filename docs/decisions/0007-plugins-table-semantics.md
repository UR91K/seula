# 0007. One `plugins` table meaning "present on this machine and/or in a project"

- **Status:** Accepted — not yet implemented
- **Decided:** 2026-09-15
- **Recorded:** 2026-09-15
- **Confidence:** decided now
- **Evidence:** maintainer's decision; current schema in `src/database/core.rs:54`

## Context

The scanner (ADR-0004) produces a set of plugins *installed on this system*. The parser
produces a set of plugins *referenced by projects*. These overlap but neither contains
the other: a plugin can be installed and unused, used and not installed, or both.

The current `plugins` table is keyed `UNIQUE(dev_identifier)` and only ever holds
plugins some project referenced, with `installed` as a flag resolved against Ableton's
database.

## Decision

Keep a single `plugins` table, restructured. **Presence in it means the plugin exists on
this machine and/or in a project.** Usage continues to be expressed by the
`project_plugins` junction table (`src/database/core.rs:107`), exactly as now — a row
with no junction entries is installed-but-unused, and a row with junction entries but
`installed = false` is used-but-missing.

The table gains `uid` plus the `PluginMeta` fields (path, category, channel counts, bus
counts, latency, GUI presence, shell flags, vendor contact), with `plugin_classes` and
`plugin_buses` child tables for the VST3 lists. `installed` is driven by scan results
rather than an Ableton lookup.

## Rejected alternatives

- **A separate `installed_plugins` registry joined to `plugins` by `(format, uid)`.**
  Cleanly models the two distinct sets and was the initial recommendation. Rejected by
  the maintainer as unnecessary indirection: the junction table already carries the
  usage relationship, so a second table would encode in schema what is already encoded
  in a join.
- **Keep the current table unchanged and store scan results elsewhere.** Leaves two
  sources of truth for "what is this plugin", which is the problem ADR-0006 exists to
  remove.

## Consequences

One place to look for any plugin, installed or merely referenced. Queries like "plugins
I own but never use" become a left join rather than a cross-table reconciliation.

`UNIQUE(dev_identifier)` gives way to a `(format, uid)` key per ADR-0005.
`dev_identifier` remains as a column, since it is what project files reference, but it
stops being the identity.

Breaking schema change. Backwards compatibility with existing Seula databases is
explicitly not required.

## Notes

Not yet started. Depends on ADR-0005 matching landing first.

Rows for VST2 shell *containers* (`is_shell = true`) will exist but must never be
matched against a project reference — Ableton stores the contained plugin's id, never
the container's.
