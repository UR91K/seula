# 0026. Count distinct plugin ids, not joined rows, in the vendor/format aggregates

- **Status:** Accepted
- **Recognized:** 2026-09-16, while adding the unscanned count to these two aggregates
  (`61cfc23`, ADR-0025's follow-up)
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** `src/database/plugins.rs` (`get_plugin_vendors`, `get_plugin_formats` —
  the `vendor_stats`/`format_stats` CTEs); `tests/database/plugin_scan.rs`
  (`vendor_and_format_plugin_count_does_not_inflate_with_project_usage`,
  `vendor_and_format_aggregates_account_for_unscanned_plugins`)

## Context

`get_plugin_vendors` and `get_plugin_formats` each build a CTE that joins `plugins` to a
per-usage subquery and aggregates by vendor or format:

```sql
FROM plugins p
LEFT JOIN (
    SELECT pp.plugin_id, COUNT(pp.project_id) as usage_count, pp.project_id
    FROM project_plugins pp
    GROUP BY pp.plugin_id, pp.project_id
) usage_stats ON usage_stats.plugin_id = p.id
GROUP BY p.vendor
```

`usage_stats` has one row per `(plugin_id, project_id)` pair, not one row per plugin. A
plugin used in five projects joins to five rows. `COUNT(*)` over that join does not count
plugins — it counts plugin-project pairs — so `plugin_count` reported five for a plugin
that exists once. The same fan-out hit `installed_plugins`, `missing_plugins` and
`unknown_plugins`, all of which summed `CASE WHEN p.installed = ...` over the same
duplicated rows.

`total_usage_count` (`SUM(usage_stats.usage_count)`) and `unique_projects_using`
(`COUNT(DISTINCT usage_stats.project_id)`) were unaffected by this fan-out — they already
aggregate correctly across the duplicated rows, since summing per-pair counts and taking
a distinct count of `project_id` both tolerate one row per pair by construction.

This was found the day after `61cfc23` added the unscanned count to these two aggregates,
while re-reading the CTEs for ADR-0025's follow-up. It predates that change; it was
invisible before because nothing checked `plugin_count` against a second source.
`seula plugin stats` (`get_plugin_stats`) counts plugins directly, without this join, and
was correct throughout — the two paths silently disagreed.

The existing reconciliation test,
`vendor_and_format_aggregates_account_for_unscanned_plugins`, did not catch this, and
could not have: every plugin in that test is used in at most one project, so the join
never fans out. It also asserts `installed + missing + unknown == plugin_count` per row,
which the bug does not break — all four columns inflate by the same per-plugin project
count and still sum correctly to each other, just to the wrong total.

## Decision

Keep the pair-level `usage_stats` subquery as is, since `total_usage_count` and
`unique_projects_using` already depend on seeing one row per pair. Fix the four
plugin-counting columns to count distinct plugin ids instead of joined rows:

```sql
COUNT(DISTINCT p.id) as plugin_count,
COUNT(DISTINCT CASE WHEN p.installed = 1 THEN p.id END) as installed_plugins,
COUNT(DISTINCT CASE WHEN p.installed = 0 THEN p.id END) as missing_plugins,
COUNT(DISTINCT CASE WHEN p.installed IS NULL THEN p.id END) as unknown_plugins,
```

`COUNT(DISTINCT ...)` collapses the fan-out regardless of how many usage rows a plugin
matched, including zero (the `LEFT JOIN` still produces exactly one row for a plugin with
no usage, so it is unaffected either way). The `CASE WHEN ... THEN p.id END` form counts
a plugin toward at most one of the three status buckets, the same partition the old
`SUM(CASE WHEN ... THEN 1 ELSE 0 END)` intended, just immune to duplication.

Applied identically to both `vendor_stats` and `format_stats`, which are the same query
shape grouped on a different column.

## Rejected alternatives

- **Group the usage subquery by `plugin_id` alone**, collapsing it to one row per plugin
  before the join, so no fan-out occurs at all. This was the first fix attempted. It
  breaks `unique_projects_using`: aggregated per-plugin project counts have to be summed
  back up across the vendor/format group, and that sum double-counts a project used by
  more than one plugin from the same vendor. The regression test that exists specifically
  to cover project overlap
  (`vendor_and_format_plugin_count_does_not_inflate_with_project_usage`, where one
  project uses two plugins from the same vendor) caught this immediately: `plugin_count`
  came out right but `unique_projects_using` over-counted. Rejected because it trades one
  inflation bug for another rather than removing the shared cause.
- **`COUNT(DISTINCT p.id)` for `plugin_count` only, leave the status sums as `SUM(CASE
  ...)`.** Fixes the headline symptom but leaves `installed_plugins` +
  `missing_plugins` + `unknown_plugins` inflated by the same fan-out that
  `vendor_and_format_aggregates_account_for_unscanned_plugins` reconciles against
  `plugin_count` — so the columns would still sum correctly to each other while all being
  wrong in the same direction, exactly the property that let the original bug hide from
  that test. Rejected for reintroducing the same class of silent-agreement failure this
  fix is meant to close.
- **A subquery per status count** (`(SELECT COUNT(*) FROM plugins WHERE vendor = p.vendor
  AND installed = 1)` etc.), run outside the join entirely. Would also be correct.
  Rejected as more verbose than `COUNT(DISTINCT CASE WHEN ...)` for the same result, and
  four correlated subqueries per row rather than one aggregate pass.

## Consequences

`plugin_count`, `installed_plugins`, `missing_plugins` and `unknown_plugins` in
`get_plugin_vendors` and `get_plugin_formats` now agree with `get_plugin_stats` and with
each other regardless of how many projects reference a plugin. `total_usage_count` and
`unique_projects_using` are unchanged in both value and implementation — they were never
wrong.

A plugin used in zero projects still counts once, via the `LEFT JOIN`'s single NULL-usage
row; `COUNT(DISTINCT p.id)` does not depend on `usage_stats` matching anything.

## Notes

The two failure modes this ADR distinguishes — a bug that stays hidden because every
affected column moves together, and a fix that trades one inflation source for a
different one — are both instances of the same risk: an aggregate query with a
one-to-many join anywhere upstream of `COUNT`/`SUM` will silently multiply whichever
columns are not shaped to tolerate the multiplicity. Any future column added to these two
CTEs should be checked against the same question: does it come from `p.*` (one value per
plugin, needs `DISTINCT`-safe aggregation) or from `usage_stats.*` (one value per pair,
already fan-out-tolerant by construction)? Mixing the two aggregation styles in one
`GROUP BY` is what produced this bug, not either style alone.
