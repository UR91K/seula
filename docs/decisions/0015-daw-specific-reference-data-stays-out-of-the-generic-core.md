# 0015. Keep DAW-specific data out of the generic core, in its own side table

- **Status:** Accepted
- **Decided:** 2026-09-15, mechanism resolved 2026-09-16
- **Recorded:** 2026-09-15, updated 2026-09-16
- **Confidence:** decided now — both the principle and the storage mechanism are settled;
  what a *second* DAW's own tables contain is not, see Notes
- **Evidence:** `crates/vst-meta/src/meta.rs` (`PluginMeta`/`FormatExtra`, the precedent
  this follows); `src/database/core.rs` `plugins` table (VST2/VST3 extras as nullable
  columns on one table — precedent for a *closed, two-member* set, distinguished below
  from the open-ended DAW case); `src/database/core.rs:143-146` (`projects`' four
  `ableton_version_*` columns) and their use in `src/database/core.rs:404-406,436-438,558-560`
  (FTS triggers and search, string-concatenated inline), `src/database/stats.rs:454-457,
  576-587,826-832` (numeric filtering, sorting and grouping by version), and
  `src/grpc/handlers/projects.rs:56-78` (public gRPC filter params named after Ableton's
  version shape); `src/models.rs:70-91` (`AbletonVersion`); ADR-0005 (`instr`/`audiofx` is
  Ableton's opinion, never identity); ADR-0009 (`plugin_refs` gets its own table);
  ADR-0014 (DAW generalisation is one phase in, not started)

## Context

Two tables hit the same problem from different sides.

`plugin_refs` (ADR-0009) is keyed on `dev_identifier`, a string Ableton itself invents
(`device:vst3:audiofx:...`), and carries `ableton_name` and `ableton_format` — Ableton's
own instr/audiofx call on that reference, already documented (ADR-0005) as disagreeing
with the plugin's own self-reported classification in practice.

`projects` bakes Ableton's version shape in even harder: `ableton_version_major/minor/
patch/beta` are `NOT NULL` columns, and they are not just display data. They are
numerically filtered and sorted directly in SQL — `database/stats.rs:832` does
`ORDER BY ableton_version_major DESC, ableton_version_minor DESC, ableton_version_patch
DESC`, and `grpc/handlers/projects.rs` exposes `ableton_version_major/minor/patch` as
public gRPC filter parameters. A string column alone (e.g. collapsing `AbletonVersion` to
one `daw_version_display` field via `FromStr`/`Display`) would lose exact-match filtering
and correct numeric sort in SQL, and push both into Rust — fetch-and-parse-everything,
a real performance regression for what is currently plain indexed SQL, not just a
"convert a string" cost. That was considered and rejected on those grounds.

ADR-0014 records that a generic `Project`/`DawParser` model is planned but not started.
Both tables are exactly the kind that generalization forces a decision on: no other DAW
has a `dev_identifier`-shaped string or an `ableton_version_major`-shaped version. Better
to decide now, while the reasoning is fresh, than re-derive it under time pressure later.

The project is pre-release alpha with no users depending on API stability (only the
maintainer's own alpha testers, who are not relying on it), so breaking the current
gRPC filter shape is an acceptable cost here, not a blocker.

## Decision

DAW-specific structured data — for any table that generalizes across DAWs — gets **its
own side table, one per DAW, related 1:1 (or 1:many, e.g. `plugin_classes`) to the generic
row it extends.** Not nullable columns shared on the generic table, and not JSON.

Applied to the two known cases:

**`plugin_refs`:** the generic core keeps the resolved `plugin_id`, `resolved_via`,
`first_seen_at`, and a DAW-agnostic raw-reference key in place of `dev_identifier` as
primary key. `dev_identifier`, `ableton_name`, and `ableton_format` move to an
`ableton_plugin_refs` side table keyed on that same reference id.

**`projects`:** the generic core keeps a `daw_type` column and a `daw_version_display
TEXT NOT NULL` column — a plain string, produced by each DAW's own `Display` impl (for
Ableton, `AbletonVersion` gains `FromStr`/`Display`), used only for FTS/search/generic
display. The structured, queryable fields move to a side table:

```sql
CREATE TABLE IF NOT EXISTS project_ableton_metadata (
    project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    version_major INTEGER NOT NULL,
    version_minor INTEGER NOT NULL,
    version_patch INTEGER NOT NULL,
    version_beta BOOLEAN NOT NULL
);
```

Filtering and sorting move from a bare `WHERE`/`ORDER BY` on `projects` to a join against
this table — an indexed join on a primary key, not a meaningful cost change from today:

```sql
SELECT p.* FROM projects p
JOIN project_ableton_metadata a ON a.project_id = p.id
WHERE a.version_major = ?
ORDER BY a.version_major DESC, a.version_minor DESC, a.version_patch DESC
```

The gRPC filter fields can keep their current `ableton_version_*` names for now — alpha
status means there is no requirement to generalize that API surface in this pass; only
the storage needs to move.

This is the same shape `crates/vst-meta` already uses for `PluginMeta`/`FormatExtra`, but
deliberately *not* the same mechanism as the `plugins` table's own VST2/VST3 extras
(nullable columns on one table) — see Rejected alternatives for why that precedent does
not transfer here.

## Rejected alternatives

- **A `daw_specific_data JSON` column**, as sketched in the archived
  `docs/archive/generalisation_plan.md`. Rejected: it contradicts the precise,
  typed-per-format modeling this codebase already committed to (`PluginKey`,
  `FormatExtra`, the `plugins` table's own nullable extras), trading an indexed column for
  `json_extract()` in `WHERE`/`ORDER BY` — worse for the exact performance concern that
  ruled out the string-only column below, not better.
- **Collapse `AbletonVersion` to a single string column** (`FromStr`/`Display`,
  no structured columns at all), reusing the same nullable string field every DAW's
  version would populate. Rejected: real, currently-working functionality depends on the
  structured integers — gRPC exact-match filtering and SQL numeric sort/group (see
  Context) — and a string-only column would force those into in-memory parsing, a
  performance regression for what is currently indexed SQL. The `FromStr`/`Display` idea
  survives in narrower form: it is what produces `daw_version_display`, the generic string
  used for FTS and display, alongside — not instead of — the structured side table.
- **Nullable columns on the shared generic table** (`projects` or `plugin_refs` growing a
  column per DAW), mirroring the `plugins` table's VST2/VST3 extras. Rejected specifically
  *for these two tables*, unlike for `plugins`: VST2/VST3 is a closed, permanent
  two-member set, so a fixed handful of extra nullable columns never grows. DAWs are not a
  closed set — the archived plan alone names six — so the same pattern here means
  `projects` permanently accumulating every DAW's fields as nullable baggage. A side table
  per DAW keeps the generic table's shape fixed regardless of how many DAWs are added
  later.
- **Leave `ableton_name`/`ableton_format`/`ableton_version_*` in the generalized core
  row.** Rejected: leaks an Ableton-only concept into fields every other DAW's row would
  carry meaninglessly, repeating the mistake already made once with `PluginFormat` baking
  Ableton's instr/audiofx call into its variants.

## Consequences

No code changes now. Both tables stay exactly as they are (ADR-0009 for `plugin_refs`,
the current `projects` schema) until Phase 1 work on ADR-0014 actually begins. This ADR
pre-commits the shape that split takes — including the concrete `project_ableton_metadata`
schema — so it is not redesigned from scratch, or redesigned differently for the two
tables, whenever that happens.

Once implemented, this is a breaking schema change (new required join for version
filtering/sorting; gRPC filter behavior unchanged in name but backed by a join instead of
a bare column). Acceptable per Context: pre-release alpha, no compatibility guarantee.

## Notes

**Resolved, as of 2026-09-16:** the storage *mechanism* — side table per DAW, not shared
nullable columns, not JSON — is now decided for both tables, not deferred. It does not
require knowing a second DAW's actual fields, only that DAWs are an open-ended set; that
was the missing piece that let this resolve without waiting.

**Still not decided, and correctly so:** what a second DAW's own side table
(`project_reaper_metadata`, `reaper_plugin_refs`, etc.) actually contains. That is
unknowable until that DAW's parser is being built, the same reasoning ADR-0014's Notes
gave for not designing `DawVersion::Reaper{...}` in the abstract. Design each DAW's side
table against its real project-file shape when that DAW is actually added.

This pairs with the still-open `PluginFormat` instr/audiofx coupling noted alongside
ADR-0014: if either is touched first, do both at once rather than separately, since
they're the same underlying problem (Ableton's classification bleeding into a place that
needs to stay DAW- or format-agnostic) surfacing in multiple tables.
