# 0010. One plugin row per identity; duplicate install locations collapse

- **Status:** Accepted — not yet implemented
- **Decided:** 2026-09-15
- **Recorded:** 2026-09-15
- **Confidence:** decided now; the duplicate case was previously confirmed by the
  maintainer against a real install
- **Evidence:** `PluginMeta.path` in `crates/vst-meta/src/meta.rs`; `UNIQUE (format, uid)`
  in that crate's `SCHEMA_SQL`

## Context

The same plugin binary can sit in more than one place. Live supports a user-configured
VST2 folder alongside the standard ones, and vendors install into several. The scanner
walks every configured root, so one `(format, uid)` can come back from two or more paths.

`PluginMeta.path` is a single column, and the plugins table is keyed `(format, uid)`, so
the second occurrence has nowhere to go. The question is whether install locations
deserve a child table.

## Decision

No. One row per `(format, uid)`, with a single `path`. When a plugin is found at several
locations the extra paths are discarded.

This matches what Ableton itself presents: two copies of the same plugin are one plugin,
not two.

## Rejected alternatives

- **A `plugin_paths` child table**, one row per install location. Models the filesystem
  accurately and would let the UI say "you have three copies of this". Rejected as
  disproportionate: nothing in the application asks where a plugin lives beyond
  identifying it, and the table would exist to hold information no feature reads.

## Consequences

`path` means "a location where this plugin was found", not "the location". Anything that
treats it as authoritative — a reveal-in-explorer action, a staleness check against file
mtime — is reasoning about one arbitrary copy of possibly several. Which copy wins is
whichever the walk reached first, and discovery sorts its results, so it is stable
between runs but not meaningful.

Duplicate detection becomes impossible without rescanning, since the discarded paths are
never recorded.

## Notes

Cheap to reverse. Adding `plugin_paths` later is an additive migration plus a change to
how scan results are persisted; no existing query would need rewriting, because they all
read `path` from the plugin row and could continue to read a designated primary.

Worth revisiting if a "why is this plugin behaving oddly" diagnostic ever appears —
duplicate installs at different versions are a real cause of that, and this decision
makes it invisible.
