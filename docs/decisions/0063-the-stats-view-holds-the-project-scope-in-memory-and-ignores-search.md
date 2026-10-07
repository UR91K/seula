# 0063. The stats view holds the project scope in memory, ignores search, and downloads its CSV

- **Status:** Proposed
- **Recognized:** 2026-10-05, building the stats view
- **Decided:** 2026-10-05, by the implementer, awaiting the maintainer
- **Recorded:** 2026-10-05
- **Confidence:** decided now; the Tauri download is untested
- **Evidence:** ADR-0045 (the stats scope is "the same preference as the other views");
  ADR-0062 (an in-memory store until the preferences endpoint exists); ADR-0048 (an OS call
  is a Tauri command with its own ADR); the `projectScope` signal in the shell module and the
  stats view's `state.ts` under `web/solid/src/`

## Context

Three things the stats view needs had no answer in the earlier records.

1. **Where the scope lives.** ADR-0045 says the `active|all` scope is shared with the other
   views, but the web code had no such store. The projects view's `Scope` is `active` or
   `archived`, which is a different choice, and the samples and collections views do not
   expose a scope.
2. **The top bar's search box.** The shell's `View` requires `query()` and `setQuery()`;
   statistics have nothing to search.
3. **Export CSV.** `/api/v1/system/statistics/export` answers with a
   `content-disposition: attachment`, but what a Tauri webview does with that is untested.

## Decision

**The scope is one signal in the shell module**, `projectScope`, in memory only, like
ADR-0062's layout. The stats view is its only reader today; the other views take it from
there when they expose the scope. It resets to `active` on every launch.

**The search box does nothing on the stats view.** `query()` is empty and `setQuery()`
ignores its argument. The box stays visible, as the frame is the same for every view.

**Export CSV is a plain download:** an anchor click on the export URL for the scope on
screen. No Tauri command.

## Consequences

If the webview does not save the file, Export CSV needs a Tauri command with a save dialog
and its own ADR (ADR-0048); this record is then superseded. Hiding or disabling the search
box on a view without search would be a change to the frame, not to this view. The scope
store moves behind the preferences endpoint when it exists.
