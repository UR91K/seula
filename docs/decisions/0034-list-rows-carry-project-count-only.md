# 0034. Plugin and sample list rows carry `project_count`, and no separate usage count

- **Status:** Accepted — implemented 2026-09-23
- **Recognized:** 2026-09-23, when the samples view needed per-row counts the API did not
  send
- **Decided:** 2026-09-23
- **Recorded:** 2026-09-23
- **Confidence:** decided now
- **Evidence:** `src/database/samples.rs` (`get_all_samples` computes `usage_count` for
  filtering and sorting, then maps each row to `Sample`, which has no field for it);
  `src/http/dto/plugins.rs` (`PluginDto` carries `usage_count` and `project_count`, and
  sets both to `None` on every route except the full list); the
  `PRIMARY KEY (project_id, sample_id)` and `(project_id, plugin_id)` constraints in
  `src/database/schema.sql`

## Context

The frontend spec gives each plugin and sample row a usage count and a project count.
The samples list returned neither. The maintainer remembered building this, and part of
it does exist: the samples query computes a count for its `min_usage_count` filter and
`usage_count` sort, but maps each row to `Sample`, which has nowhere to put it. Plugins
got further, through the `GrpcPlugin` wrapper, but only on the full list: the search and
install-status routes return `None` for both counts.

Tracing this turned up the more important fact. `project_samples` and `project_plugins`
each have a primary key on the (project, item) pair, so an item appears at most once per
project. "Uses" and "projects using it" are therefore always the same number. Nothing
records how many times one project uses a sample, because the parser records only which
samples a project uses. The two columns were one number with two names.

## Decision

- Every plugin and sample row the HTTP API returns (list, search, filter-by-status and
  single-item routes) carries `project_count`, always populated.
- `usage_count` leaves the HTTP responses. The query parameters follow: the HTTP API takes
  `sort_by=project_count` and `min_project_count`/`max_project_count`, and maps them onto
  the database layer's existing names.
- Counts are attached at the HTTP layer by one batched count query per page, not by
  widening the database functions' return types. The gRPC handlers and the CLI call the
  same functions and are left as they are: gRPC is being retired (ADR-0028), and changing
  the return types would drag both along for no benefit.
- `project_samples(sample_id)` and `project_plugins(plugin_id)` get indexes. They are
  additive, so no `SCHEMA_VERSION` bump.

## Rejected alternatives

- **Keep both counts.** Rejected because two columns that always agree tell the user there
  is a difference to notice, and there is none.
- **Keep the name `usage_count`.** Rejected because "projects" is what it counts. A future
  true per-project usage count, if the parser ever records one, can then take the name
  without breaking anything.
- **Widen `get_all_samples` and friends to return counts.** Rejected for the gRPC and CLI
  churn described above. The cost of the chosen approach is one extra indexed query per
  page.

## Consequences

The vendor and format aggregates keep `total_usage_count`, which is a different number
(a sum of per-plugin project counts over the vendor's plugins), and it stays under that
name.

The database layer still says `usage_count` internally. That is only a naming leftover,
and the HTTP layer is the only place that translates it.
