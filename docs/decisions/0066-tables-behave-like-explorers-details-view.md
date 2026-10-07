# 0066. Tables behave like Explorer's details view

- **Status:** Accepted
- **Recognized:** 2026-10-07, from the bug `headers-rendering-over-sidebar` in
  `docs/bugs.md`
- **Decided:** 2026-10-07, by the maintainer
- **Recorded:** 2026-10-07
- **Confidence:** decided now; checked in a browser against the maintainer's library at
  pane widths of 532 and 1118 px, for the projects, plugins and samples tables. The
  collections table uses the same code but was not looked at: that library has no
  collections
- **Evidence:** `web/solid`'s `shell/columns.tsx`, used by the projects, plugins, samples
  and collections tables; the overrides at the end of `web/shared/app.css`

## Context

Below a window width of about 1144 px, the table headers painted over the inspector. The
table was not the cause. `.main` is a grid with no column template, so its one column
grew to fit its widest child, the view bar, and the whole pane spilled into the
inspector's column. The inspector, later in the page, covered the spilled rows, but the
headers are sticky with `z-index: 1` and painted above it.

Fixing that raised what the tables should do when the pane is narrower than their
columns. The mockup's tables used automatic layout with `min-width: 100%`, so column
widths followed the content and the pane, and the header grips drew a resize cursor but
did nothing. The maintainer asked for Explorer's details view: every column keeps its
width whatever the pane does and can be dragged wider or narrower, the last column
reaches the pane's right edge, and every column is left-aligned.

## Decision

- **The main pane is clamped and clips.** `.main` gets `grid-template-columns:
  minmax(0, 1fr)` and `overflow: hidden`. Menus and popovers are positioned against the
  window, not the pane, so the clip does not cut them. The view bar wraps onto a second
  row when it does not fit, instead of squashing its controls into each other.
- **Fixed column widths.** Tables use `table-layout: fixed` at `width: 100%`. Every
  column has a width except the last, and the table's `min-width` is the sum of every
  column's width, the last one's default included. When the pane is wider, the last
  column takes the rest. When it is narrower, the table keeps its minimum and the pane
  scrolls sideways. Header widths are `border-box`, so the sum is exact.
- **Dragging resizes.** A header's grip sets that column's width in px, at least 24 px,
  starting from its drawn width. The grip swallows its own click, so a drag never sorts.
  It sits inside its header: the mockup's straddled the edge, the next header clipped its
  outer half, and a press on the edge sorted the neighbouring column instead. Header text
  cannot be selected.
- **Widths are per table and kept in memory** until the preferences endpoint exists, as
  the collections layout is (ADR-0062). The projects view and each collection's
  tracklist keep their own (ADR-0058).
- **Everything is left-aligned.** Numbers keep tabular figures. The narrow icon columns
  (checkbox, audition, cover, a tracklist's place) keep their centring and do not
  resize.

The approved mockup stylesheets are imported unchanged, so all of this is overrides in
`web/shared/app.css`. The static mockups still show the old behaviour.

## Rejected alternatives

- **Automatic layout, as the mockup had.** Rejected: column widths depend on the content
  and the pane, which is what was asked not to happen.
- **Fixed layout with every column given a width.** Rejected: at `width: 100%` a browser
  spreads the spare width across all the columns, so they still grow with the window.
  Leaving the last one without a width is what makes it take all the spare width.
- **A filler column after the last one**, as Explorer itself leaves empty space.
  Rejected: the maintainer wanted the last column to reach the edge.
- **Persisting widths in `localStorage`.** Rejected for now, for ADR-0062's reason:
  preferences should live in one place, and that place does not exist yet.
- **CSS `resize` on header cells.** Not possible: `resize` does not apply to table cells.

## Consequences

Columns no longer move when the window does, and a table wider than its pane scrolls.
The bug cannot come back from a wide child alone, because the pane clips.

The costs:

- Dragged widths are lost on restart.
- Dragged widths are in px while defaults are in `--u`, so a later density change
  (ADR-0032) rescales only the columns nobody has touched.
- Numbers are no longer right-aligned, so digits in a column no longer line up by place.
- Dragging the last column only changes the table's minimum, so it shows only when the
  table overflows.
- Explorer's double-click on an edge, to fit a column to its content, is not built.

## Notes

The pane clamp and the column behaviour are independent: the clamp alone fixes the bug.
The column rules are a handful of CSS lines and one small module, so they are cheap to
change.
