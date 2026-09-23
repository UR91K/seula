# 0044. A collection opens into a full-width tracklist; the inspector shows the selection

- **Status:** Accepted
- **Recognized:** 2026-09-23, when ADR-0032 made the inspector general and left the
  collections detail page "in tension with the inspector", to be resolved by the mockup
- **Decided:** 2026-09-24, planning the collections board
- **Recorded:** 2026-09-24
- **Confidence:** decided now
- **Evidence:** ADR-0032 (the inspector is one panel across all four content views);
  `docs/archive/FRONTEND_SPEC.md` (the original sketch's separate collection page);
  `docs/architecture/frontend.md` (the collections section, which put the tracklist and
  its reordering in the inspector); ADR-0036

## Context

The original sketch gave a collection its own page. ADR-0032 then made the inspector a
general panel that shows whatever is selected, in every view, and the collections section
of `frontend.md` moved the whole collection into it: summary, a drag-to-reorder tracklist
and the consolidated tasks. Whether opening a card should also go somewhere was left open.

The inspector is 150 units wide. That is room for a project's name and length, but not
for its tempo, key, plugins or samples. A tracklist is where those matter: a collection is
usually an EP, a set or a batch of related ideas, and sequencing one means comparing
tempos and keys across the tracks. A drag handle, a number and a name also take most of
that width.

Offered the choice, the maintainer chose a separate view for an opened collection and the
inspector for whatever is selected.

## Decision

**Selecting a card fills the inspector; opening it (double-click, or Enter) replaces the
grid with the collection's tracklist.** The shell does not change: the sidebar still
says Collections, and the view toolbar leads with a back button and the path,
`Collections › night drives`.

**The opened view** has a header band (cover, name, description, and a line of facts from
`/collections/:id/statistics`), then the collection's projects in a table drawn by the
same renderer as the projects view, with the same columns and column chooser. A leading
`#` column gives each project's place. The table starts in that order, and dragging a
row's handle reorders (`PUT /collections/:id/reorder`). Clicking another header sorts the
view without changing the stored order, and the drag handles go away until `#` is clicked
again, as a music library does. Rows are selected as in the projects view, and the
selection can be removed from the collection.

**The inspector shows the selection, as everywhere else.** In the opened view, one
selected project shows the project inspector, and several show the shared one. With no
project selected, it shows the collection. The collection inspector (in the grid, or in
the opened view with nothing selected) has the cover, name and description, the facts, a
short numbered tracklist with no drag handles, and the consolidated tasks, each naming its
project.

**Two layouts for the list of collections:** a grid of cover cards (the default) and a
details table, switched from the view toolbar and remembered with the other preferences
(ADR-0031). A card shows the cover, the name, and the project count and length. The
table adds the description and both dates, and sorts from its headers. The grid sorts
from a Sort dropdown. Both sort on the server (`sort_by`, including `project_count` and
`total_duration`, ADR-0043).

## Rejected alternatives

- **The inspector is the whole of it.** One less view, but the tracklist would show only
  name and length, and a collection could not be sequenced by tempo or key. The
  maintainer's call was that sequencing needs the full table.
- **Opening a card goes to the projects view, filtered to the collection.** The projects
  view has no header for the collection, sorts by date and not by position, and has no
  place for reordering. It is still offered, as "Show in Projects" in the card's menu,
  through the `collection:` search operator added for it (`collection:"night drives"`).
- **Explorer's three layouts (large icons, list, details).** A compact list adds little
  over the details table for a library of tens of collections. The maintainer chose
  grid and details.

## Consequences

`frontend.md` changes: the tracklist and its reordering move from the inspector to the
opened view, and the card drops the created date, which the table and the inspector
still show.

Opening a collection is a view state, not a new sidebar entry. Whether the browser
target gives it its own URL, so that it survives a reload and can be linked, is for the
React work to settle.
