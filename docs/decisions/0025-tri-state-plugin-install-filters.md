# 0025. Plugin install-status filters take a tri-state, not a bool

- **Status:** Accepted
- **Recognized:** 2026-09-16, while diagnosing three stale tests left by the ADR-0006
  migration; the underlying defect is older
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** `src/database/plugins.rs` (`get_plugins_by_installed_status`,
  `search_plugins`, `get_all_plugins`); `proto/services/plugins.proto`
  (`GetPluginByInstalledStatusRequest.installed`, `installed_only` on the list and search
  requests, and `GetPluginStatsResponse` which is already tri-state);
  `src/services/plugins.rs`; `src/grpc/handlers/plugins.rs`;
  `src/cli/commands/plugin.rs` (`installed_cell`, `installed_label` — already tri-state
  for display); ADR-0012 (which made `installed` tri-state in the first place)

## Context

ADR-0012 made `plugins.installed` tri-state on purpose. `NULL` means no scan has looked,
which is deliberately not the same as looked-and-absent, so a partial scan can leave
plugins it never reached alone instead of declaring them missing.

The storage layer honors that. The query layer does not. Three filter paths take a
boolean:

- `get_plugins_by_installed_status(installed: bool)` builds `WHERE installed = ?`
- `search_plugins(installed_only: Option<bool>)` appends `p.installed = ?`
- `get_all_plugins(installed_only: Option<bool>)` does the same

In SQL, `NULL = 0` is not true, so every one of these silently drops the never-scanned
rows. A caller asking for "not installed" gets only the plugins a scan confirmed absent,
and is given no indication that a third category exists and was excluded. The boolean
cannot express the question, so the query answers a narrower one without saying so.

The clearest evidence that this is a defect rather than a deliberate narrowing is that
the rest of the system already speaks in three states. `GetPluginStatsResponse` carries
`installed_plugins`, `missing_plugins` and `unknown_plugins` with the comment
`installed + missing + unknown == total`. The CLI's `installed_cell` and
`installed_label` both match on `Option<bool>` and render all three. So `seula plugin
stats` will report that N plugins have never been scanned, and there is then no way to
list them — the display layer and the statistics layer know about the third state and
the filter layer does not.

The naming records the confusion too: `installed_only` is a field where `Some(false)`
means "not installed". A flag named `_only` with a meaningful `false` was already a sign
the parameter was carrying more meaning than its type could hold.

This was found while diagnosing `test_psp_springbox_plugin_from_real_project`, which
queries `get_plugins_by_installed_status(false, ...)` and expects a freshly batch-inserted
plugin to come back. It does not, because `BatchInsertManager` deliberately omits
`installed` from its INSERT (ADR-0012), leaving it `NULL`. The test is stale, but it is
stale because it encoded the reasonable assumption that "not installed" includes "never
looked" — the same assumption any caller would make.

## Decision

Replace the boolean on all three filter paths with an explicit tri-state, so callers
state which of the three row categories they want and no category can be excluded
silently.

```rust
enum InstallState { Installed, Absent, Unscanned }
```

Filters take a set of these rather than a single value, because the useful questions are
unions: "everything I cannot load" is `Absent | Unscanned`, and "everything a scan has
ruled on" is `Installed | Absent`. A single-valued parameter would reintroduce the
original problem one level up, forcing callers back into picking the closest available
approximation. An empty or absent set means no filtering, which is what `None` means on
the current `Option<bool>` parameters.

The mapping to SQL is explicit per variant — `installed = 1`, `installed = 0`,
`installed IS NULL` — rather than an equality test that happens to be right for two of
the three cases.

This changes the proto. `GetPluginByInstalledStatusRequest.installed` and the
`installed_only` fields on the list and search requests are replaced by a repeated enum
field, and `installed_only` loses its misleading name in the process. That is acceptable
here for the same reason ADR-0024 accepts changing the HTTP contract freely: every
consumer of this API is first-party and local, so there is no external client to break
and no deprecation window to serve.

## Rejected alternatives

- **Change `WHERE installed = ?` to `WHERE installed IS NOT 1` and keep the boolean.**
  This is the minimal fix and it makes the specific failing case behave sensibly.
  Rejected because it picks one of the two possible readings of "not installed" and bakes
  it in just as silently as the current code bakes in the other. A caller who genuinely
  wants confirmed-absent-only — the one honest use of the current behavior — would then
  have no way to ask for it, and the next person to need that distinction would find a
  boolean that lies in the opposite direction.
- **Leave it, and document at the query site that `false` excludes `NULL`.** Rejected:
  the site is three functions deep from the CLI and gRPC callers that reach it, and a
  comment does not stop a caller asking the wrong question — it only explains the wrong
  answer afterwards, to whoever thinks to read the database layer. ADR-0012 established
  the third state as load-bearing; a query API that cannot name it is incomplete, not
  under-documented.
- **A single-valued tri-state enum rather than a set.** Simpler signature. Rejected
  because the two most useful queries are unions, so callers would immediately need
  either two round trips or a fourth "not installed or unknown" pseudo-variant — which is
  the boolean's problem again, wearing an enum's clothes.
- **Keep the proto boolean and translate at the service layer.** Preserves the wire
  format. Rejected: it would leave the gRPC surface unable to express the query the layer
  beneath it now supports, which is exactly the gap being closed, and there is no external
  consumer whose stability would justify it.

## Consequences

Three call sites in the database layer, their service-layer wrappers, the gRPC handlers,
the proto definitions and the CLI's `--installed` flag all change together. The CLI flag
is currently `Option<bool>`; it gains a way to say "unscanned", which is new user-facing
capability rather than a rename.

Callers must now be explicit, which is the point but is also a real cost: a caller that
previously passed `false` must decide what it actually meant. Existing call sites need
that decision made for them one at a time rather than mechanically.

`seula plugin stats` and the plugin list can now agree with each other. They could not
before, since the unknown count was reportable but not listable.

The three stale tests found alongside this are not caused by it and are fixed separately
— two assert a plugin-name backfill that ADR-0006 deleted with Ableton's database, and
have no relationship to install-status filtering.

## Notes

This does not change what `installed` means, how it is written, or who may write it.
ADR-0012 remains the authority on all three, and the rule that only a plugin scan writes
`plugins.installed` is untouched. This ADR is about the read path only.

If a variant is ever added — a fourth state, or a distinction within "absent" — the set
parameter absorbs it without changing any existing caller's meaning, which is the main
thing the set shape buys beyond expressing unions.
