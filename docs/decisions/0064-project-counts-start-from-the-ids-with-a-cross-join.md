# 0064. Project counts start from the page's ids, forced with a CROSS JOIN

- **Status:** Accepted
- **Recognized:** 2026-10-07, running the frontend against the maintainer's real library
  for the first time
- **Decided:** 2026-10-07, asked for by the maintainer
- **Recorded:** 2026-10-07
- **Confidence:** decided now; the plans and timings below were measured on a copy of the
  maintainer's database (3,642 projects, 18,243 samples, 32,797 project–sample rows) and
  through the HTTP API against debug and release builds
- **Evidence:** `counts_sql` and its plan test in `src/database/project_counts.rs`;
  ADR-0034 (a page's project counts are one query, not a join in every list query);
  ADR-0040 (counts leave archived projects out by default)

## Context

The samples view was slow on a real library: `GET /api/v1/samples?limit=10000` took 14.0 s
in a debug build and 8.1 s in release. Release roughly halves every response (rusqlite
compiles its bundled SQLite with the profile's optimisation level, so debug runs an
unoptimised SQLite too), but that was not the main cost.

The list query itself took 47 ms for 5,000 rows. The time went into attaching each row's
project count. Those counts are queried per chunk of 500 ids:

```sql
SELECT j.sample_id, COUNT(*) FROM project_samples j
JOIN projects scope_p ON scope_p.id = j.project_id AND scope_p.is_active = 1
WHERE j.sample_id IN (?, ?, ... 500 ids) GROUP BY j.sample_id
```

SQLite planned that from the wrong end. It started from every active project, through
`idx_projects_is_active`, and probed each one against all 500 ids on the junction's
primary key: about 1.8 million lookups and 344 ms a chunk, so about 3.5 s of a 4.1 s
page. The database has no `sqlite_stat1`, so the planner works from fixed estimates, and
those favour the projects side when the `IN` list is long. With three ids it chooses the
other order, which is why small test data never showed it.

## Decision

**The count query is written so that SQLite must start from the ids.** In
`counts_sql`, the scope's join becomes a `CROSS JOIN`, which SQLite never reorders. The
plan is then: the junction through its index on the item column
(`idx_project_samples_sample`, `idx_project_plugins_plugin`), then each row's project by
its primary key.

The same function counts plugins, so plugin counts get the same plan. The other users of
`ProjectScope::join` are unchanged. They aggregate over every row rather than filter by a
page of ids, and none was measured to be slow.

A unit test plans the real query with a full chunk of ids and asserts that it starts
from the junction's item index, for both tables and both scopes. Without `CROSS` it fails
with the slow plan above.

Measured after the change, 5 ms a chunk instead of 344 ms with identical counts, and:

| `GET /api/v1/samples?limit=10000` | Before | After |
|---|---|---|
| debug | 14.0 s | 0.40 s |
| release | 8.1 s | 0.19 s |

## Rejected alternatives

- **Run `ANALYZE` and let the planner decide.** Rejected: it might choose well, but the
  plan would then depend on statistics that must be gathered and kept current, and on the
  shape of each user's library. Starting from the ids is right for any library: it reads
  only the junction rows of at most 500 ids, while starting from the projects probes every
  project against every id.
- **`INDEXED BY idx_project_samples_sample`.** Rejected: it names an index, so it needs
  a different name per table and turns an index rename into a query error. `CROSS JOIN`
  states the order, which is the actual intent, and leaves the index choice to SQLite.
- **A unary `+` on `scope_p.is_active` to hide its index.** Rejected: it removes one bad
  option without stating the intended order, and it is obscure to read.
- **Count in the list query itself.** The samples list already joins a usage count for
  sorting, so this is possible. Rejected for now: ADR-0034 kept counts out of the list
  functions so their signatures, and the gRPC and CLI callers, stay unchanged. The
  one-word fix leaves that untouched.
- **Accept it and run release builds.** Rejected: release removes about half the time;
  the plan removes about 97% of it.

## Consequences

The samples view on a large library loads in well under a second instead of 8 to 14
seconds, and the paging the frontend now does costs little.

The order is fixed. If a future scope narrows the projects sharply (one collection, a
handful of projects), starting from the projects could become the better plan, and this
query would not switch to it by itself.

## Notes

Reversal is one word. If the plan test fails after a SQLite upgrade, time the query on a
real library before deciding which side is wrong. The test checks the plan, not the
speed.

`[profile.dev.package.libsqlite3-sys] opt-level = 3` in the root `Cargo.toml` would
build SQLite optimised in debug builds too. It was not done here; it is a separate choice.
