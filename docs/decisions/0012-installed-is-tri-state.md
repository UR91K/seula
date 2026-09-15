# 0012. `installed` is tri-state: yes, no, or not yet looked

- **Status:** Accepted
- **Decided:** 2026-09-15
- **Recorded:** 2026-09-15
- **Confidence:** decided now
- **Evidence:** `plugins.installed` in `src/database/core.rs`; `Plugin::installed` in
  `src/models.rs`; `optional bool installed = 7` in `proto/common.proto`

## Context

`installed` was a `NOT NULL BOOLEAN` meaning "this plugin is on the system". Two things
made that shape wrong once the scanner (ADR-0004) became its source.

**A database that has never been scanned has no honest value.** Every plugin would read
`false` — indistinguishable from a plugin that was looked for and genuinely not found.
The alternative considered was running a plugin scan automatically the first time
`process_projects` finds an empty plugins table, purely so the flag would not lie. That
is a two-minute scan triggered as a side effect of something else, to work around a
representation problem.

**A partial scan has no honest value either.** A scan narrowed with `--paths`, or one
whose restart budget ran out, has looked at some plugins and not others. With a boolean
it must either declare the ones it never reached missing, or skip the sweep and leave
stale `true`s behind. Neither is true.

The flag was also already lying: `refresh_plugin_installation_status` tested
`get_plugin_by_dev_identifier(..).is_ok()`, which is `true` for `Ok(None)`, so every
plugin was marked installed regardless of the answer (ADR-0006).

## Decision

`installed` is nullable. `NULL` means no scan has looked, `1` that the last scan found
it, `0` that the last scan looked and did not.

`Plugin::installed` is `Option<bool>`. The proto field becomes `optional bool` — the
same field number, so the change is wire-compatible. `PluginStats` gains
`unknown_plugins`, because `installed + missing` no longer sums to the total.

**Only a plugin scan writes it.** Parsing a project records a *reference*; a project file
cannot know what is installed on this machine, so the project insert path leaves the
column alone.

## Rejected alternatives

- **Keep the boolean, auto-scan on first run.** Hides the never-scanned case behind a
  side effect, and still cannot represent a partial scan. It also makes the first
  project scan silently two minutes slower.
- **Keep the boolean, no auto-scan.** Everything reads as not-installed until the user
  runs a scan they have not been told about.
- **A separate `last_scanned_at IS NULL` test for "unknown".** The column exists and is
  populated, but making the meaning of one column depend on another is exactly the
  implicitness the tri-state removes.

## Consequences

Roughly seventy call sites changed, concentrated in `src/database/plugins.rs` and
`src/cli/commands/plugin.rs`. The CLI shows three states; `"Unknown"` is a real answer
that tells the user to run `seula plugin refresh`, not a fallback.

SQL filters need no special handling: `installed = ?` matches neither `true` nor `false`
for a NULL row, which is the semantics we want. Aggregates do — `SUM(CASE WHEN installed
= 0 ...)` silently drops unknowns, so any query that partitions plugins needs a third
branch or its counts stop summing to the total.

## Notes

Reversing this means choosing which lie to tell for never-scanned plugins. If it is ever
reversed, the auto-scan-on-first-run rejected above is the only version that does not
mislead.
