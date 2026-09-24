# 0045. The stats view takes the project scope, splits its counts, and is one page

- **Status:** Accepted
- **Recognized:** 2026-09-24, mapping the stats sketch onto `/api/v1/system/statistics`
  before its board
- **Decided:** 2026-09-24, planning the stats board
- **Recorded:** 2026-09-24
- **Confidence:** decided now
- **Evidence:** `docs/archive/FRONTEND_SPEC.md` and the stats section of
  `docs/architecture/frontend.md` (the sketch: a date range and collection or tag
  filters across every section); `src/database/stats.rs`, `src/services/system.rs`
  (the route took no parameters); ADR-0040 and ADR-0043 (the scope the other views
  take); ADR-0012 (the three plugin states); `docs/status.md` (the bugs found at the
  same time)

## Context

The sketch for the stats view had a date-range selector and filters by collection or
tag, applied to every section. The route behind it took no parameters at all, and the
gRPC request that declares those filters has never read them. Every other view counts in
a project scope, `active` or `all`, kept as one preference (ADR-0040, ADR-0043); the
statistics counted active projects in some figures and every row in others.

Checking the route against the sketch also found that several figures were wrong in ways
a chart would hide: lumped tempo bins, months left out, averages over a subset, and a
completion rate in two units (all in `docs/status.md`). The maintainer confirmed each as
a bug.

The overview row was a set of bare totals. The maintainer asked for each to show what it
is made of, as the plugins view's status bar already does for installed, missing and not
scanned: "plugins: 50, 30 installed, 12 missing, 8 not scanned; projects: 220, 200
active, 20 archived".

## Decision

**The stats view filters by the project scope only.** `/system/statistics` and its CSV
export take `scope=active|all`, the same preference as the other views. There is no date
range and no collection or tag filter. A collection's own statistics are in its inspector
(ADR-0044).

**Every figure counts the projects in scope and what they use.** Plugins, samples and
tags are the distinct ones used by those projects; tasks are theirs; a collection
counts its projects in scope. Averages divide by every project in scope, including
projects with none of the thing averaged, and by every collection, including empty
ones. The one exception is the project split: active and archived are both shown
whichever the scope is, since they are what the scope chooses between.

**Each overview count is split into its states, and the total is the sum of the
parts:**

| Count | Split into |
|---|---|
| Projects | active, archived |
| Plugins | installed, missing, not scanned (ADR-0012) |
| Samples | present, missing |
| Collections | with projects, empty |
| Tags | in use, unused |
| Tasks | completed, pending, and the completed share |

The HTTP response carries these as objects (`projects: {total, active, archived}`,
and so on) in place of the flat totals. The proto keeps its flat totals and adds a
`LibraryCounts` message beside them.

**One scrolling page.** A row of the split counts, then bands for music, plugins and
samples, activity, and the library, each a grid of small panels. Not tabs: the view is
for looking over the whole library at once.

## Rejected alternatives

- **The sketch's filters.** A date range means a range picker and a second axis
  through every query. A collection or tag filter overlaps the collection inspector's
  statistics and the projects view's search. None was asked for since the sketch.
- **Bare totals in the overview.** A plugin total alone says nothing about whether
  those plugins are installed, and the plugins view already shows the split.
- **Tabs per section.** Less scrolling, but no view of everything at once.

## Consequences

The gRPC handler passes `active` and still ignores its request's filters. The CLI's
system summary keeps its own unscoped `get_basic_counts`.

Time series come back oldest first, with empty periods as zero, and the tempo histogram
in equal 10 BPM bins from the lowest tempo used to the highest. A chart can draw them as
they come.

The proto `Project` gains `is_active`, so any statistic that embeds a project can say
it is archived.
