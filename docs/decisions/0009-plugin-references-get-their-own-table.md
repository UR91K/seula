# 0009. Store Ableton's reference strings in a `plugin_refs` table

- **Status:** Accepted — not yet implemented
- **Recognized:** 2026-09-15, while re-examining whether installed and referenced
  plugins should be separate tables
- **Decided:** 2026-09-15
- **Recorded:** 2026-09-15
- **Confidence:** decided now
- **Evidence:** ADR-0007; live scan output for Dexed showing three factory classes with
  three distinct IDs

## Context

ADR-0007 settled on one `plugins` table meaning "present on this machine and/or in a
project", and said of the reference string:

> `dev_identifier` remains as a column, since it is what project files reference, but it
> stops being the identity.

It cannot be a column. A multi-class VST3 bundle is one plugin with one `(format, uid)`,
but Ableton references whichever processor class the user instantiated — so the same
installed plugin can legitimately be named by several different `dev_identifier`s across
different projects. Dexed exports three classes with three distinct IDs. One column holds
one of them.

Separately, the maintainer revisited ADR-0007's rejection of a separate registry table.
That rejection was recorded as:

> the junction table already carries the usage relationship, so a second table would
> encode in schema what is already encoded in a join

`project_plugins` carries *project ↔ plugin*. It says nothing about *reference ↔
binary*. Those are different relationships, so the stated rationale was aimed at the
wrong one. The conclusion still holds, but for the reasons below rather than that one —
and a decision defended by a rationale that does not survive inspection is one that gets
re-litigated, which is what happened.

## Decision

Keep ADR-0007's single `plugins` table. Add a child table:

| Table | Key | A row means |
|---|---|---|
| `plugins` | `(format, uid)` | a plugin — installed, referenced, or both |
| `plugin_refs` | `dev_identifier` | an Ableton reference string, and the plugin it resolves to |
| `project_plugins` | `(project_id, plugin_id)` | which projects use which plugin — unchanged |

`plugin_refs` also stores the name Ableton recorded and its `instr`/`audiofx` call. Both
are kept out of identity per ADR-0005, but are worth having: a disagreement between
Ableton's classification and the binary's is a useful diagnostic rather than noise.

What makes this coherent is that `(format, uid)` is derivable from a `dev_identifier`
alone — `PluginKey::from_dev_identifier`. A referenced-but-missing plugin is therefore
not a second-class entity needing its own home; it gets an ordinary `plugins` row with
`installed = 0` and NULL scanner columns.

## Rejected alternatives

- **Separate `installed_plugins` and `discovered_plugins` tables joined by a junction.**
  The cardinality does not fit: many references resolve to one binary, never the
  reverse, so the relationship is a nullable foreign key rather than a junction. And it
  makes "show me every plugin" — the query this application is substantially about —
  a `UNION` or `FULL OUTER JOIN` across two tables, because an installed-but-unreferenced
  plugin would have no row in one of them and a referenced-but-missing plugin no row in
  the other. The single table matches how a user thinks about a plugin: one thing,
  whether or not it happens to be installed.
- **`dev_identifier` as a scalar column on `plugins`** (what ADR-0007 assumed).
  Silently loses every reference to a multi-class bundle beyond the first.
- **No reference table; re-derive references by reparsing projects.** The
  `dev_identifier`s only exist inside `.als` files, so "install a missing plugin, then
  refresh" would require reparsing every project to rediscover them. Storing them makes
  that refresh a SQL re-resolve.

## Consequences

The bought thing: **"refresh plugins and fill in the gaps" becomes a pass over
`plugin_refs`.** Install a missing plugin, rescan plugins, and every project that
referenced it lights up without touching a single project file. That is the whole reason
this table earns its place.

Costs, honestly:

- Project → plugin *details* gains a table to keep in sync, and diagnostics that want to
  know which reference a project used need the extra join.
- Because `project_plugins` points at the plugin rather than the reference, we do not
  record *which* class a given project referenced when a bundle is reachable by several.
  Nothing needs that today; if something does, the junction moves to `plugin_refs`.
- Two write paths now touch plugin state — the plugin scan and the project scan — and
  they must agree on the identity rules in ADR-0005.

## Notes

Reversal is cheap in one direction: collapsing `plugin_refs` back into a column is a
migration plus accepting the multi-class loss. Splitting `plugins` in two later is not
cheap, because every query and the gRPC surface assume one table.

Rows for VST2 shell containers (`is_shell = true`) still must never be matched — Ableton
stores the contained plugin's id, never the container's. Carried forward from ADR-0007.
