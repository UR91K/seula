# 0040. Plugin and sample usage is counted in a project scope, active by default

- **Status:** Accepted — implemented 2026-09-23
- **Recognized:** 2026-09-23, reviewing the plugins board
- **Decided:** 2026-09-23
- **Recorded:** 2026-09-23
- **Confidence:** decided now
- **Evidence:** the mock snapshot, where 41 of 50 plugins had a `project_count` larger
  than their used-in list (Pro-Q 3: 97 against 90, the 7 being archived projects);
  `src/database/project_counts.rs` (counted every row of the junction table);
  `src/database/projects.rs` (`get_projects_by_plugin_id` and `_by_sample_id`, active
  projects only); ADR-0019 (the projects list shows active projects by default); ADR-0034

## Context

A plugin's or sample's `project_count` counted archived projects, while its used-in list
(`/plugins/:id/projects`, `/samples/:id/projects`) showed only active ones. The two
disagreed for most rows, and neither said which projects it meant. Nobody had chosen
either behaviour; each query was written on its own.

The maintainer's view: whether archived projects count should be a deliberate setting,
not an accident of whichever query answered.

## Decision

**A `scope` query parameter, `active` or `all`, default `active`.** It applies to every
route that counts or lists the projects using a plugin or sample:

- the plugin list, search, by-status list and single plugin (`project_count`, and the sort
  and minimum-count filter that use it);
- the vendor and format rollups (`unique_projects_using`, `total_usage_count`);
- the sample list, search and single sample (`project_count`, its sort and range filters);
- the used-in lists, `/plugins/:id/projects` and `/samples/:id/projects`.

`active` leaves archived projects out, matching the projects view's default (ADR-0019).
`all` includes them. Any other value is a 400, not a silent default. Every project in an
API response now carries `is_active`, so a used-in list under `all` can mark the
archived ones.

The server stores no setting. The UI's choice is a preference, saved with the others
(ADR-0031), and sent with each request.

In the database layer this is `ProjectScope`, taken by every function above. The count
and the list for a row are computed with the same scope, so they agree.

## Rejected alternatives

- **A `config.toml` option.** One answer for every client and every view, changed by
  editing a file. A per-request parameter lets a client ask either question.
- **Always active, no parameter.** Would fix the disagreement, but make "how many projects
  ever used this plugin" unanswerable.

## Consequences

The gRPC and CLI callers pass the default, so their plugin and sample counts now leave
archived projects out, where they used to include them. ADR-0028 retires gRPC.

The sample list failed with an SQL error ("no such column: s.is_present") whenever a
project-count filter was combined with the present, missing or format filter: the count
query referred to the list's table alias from outside its subquery. That query was
rewritten here, and the combination works.

Collections, tags and statistics are not affected; they have their own rules for
archived projects.
