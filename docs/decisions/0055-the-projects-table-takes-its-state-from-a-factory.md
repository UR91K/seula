# 0055. The projects table takes its state from a factory, so the tracklist can reuse it

- **Status:** Accepted — implemented on branch `tauri-app`
- **Recognized:** 2026-10-05, planning the collections view
- **Decided:** 2026-10-05, agreed by the maintainer
- **Recorded:** 2026-10-05
- **Confidence:** decided now; the shape is the assistant's recommendation, the choice to
  reuse the table is the maintainer's
- **Evidence:** ADR-0044 (an opened collection shows its projects in a table); the
  `web/solid/src/projects/` directory, where the table, inspector, popovers and view bar
  all import one module of module-level state; ADR-0052 (module-level stores, because an
  unmounted view loses anything held inside it)

## Context

The opened collection is the projects table over a different list, with its own sort and
selection, plus a `#` column and drag handles. Today the table reads one set of
module-level stores: the list, the sort, the selection and the columns.

## Decision

Extract that state into a factory that returns a store of the same shape. The projects
view calls it once at module level and is the first consumer. The collections view calls
it again for the tracklist. The table component takes the store as a prop instead of
importing it.

The factory's stores stay at module level in each consumer (ADR-0052), so unmounting a
view still discards nothing.

## Rejected alternatives

- **A second table component for the tracklist.** Rejected: two tables drift, and column
  widths, keyboard handling and selection would have to be fixed twice.
- **One shared store, swapped by a mode flag.** Rejected: the tracklist's sort must not
  disturb the projects view's sort, and a flag makes that a bug waiting to happen.

## Consequences

The projects view changes without changing behaviour; it needs a re-check against the mock
daemon. The `#` column and the drag handles are optional parts of the table, shown only
when the consumer asks for them.
