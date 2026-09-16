# Seula frontend mockups

Static HTML/CSS mockups of the Seula web frontend, made before any React exists. They
follow the shape in `docs/architecture/frontend.md` and the maintainer's original
sketch in `docs/archive/FRONTEND_SPEC.md`. See ADR-0029 (stack and placement), ADR-0030
(browser vs. native), and ADR-0031 (preferences storage) for the reasoning behind
choices reflected here.

## Running

No build step, no dependencies beyond the Python standard library and a network path
to the Lucide icon CDN (jsdelivr). From this directory:

```
python serve.py
```

This prints a `http://127.0.0.1:<port>/index.html` URL and serves the directory it
lives in, so it works regardless of your current working directory. Pass a port number
as the first argument to request a specific one; it falls back to a random free port if
that one is taken.

You can also open `index.html` directly from disk in a browser. Everything works
except the icons, which need the CDN.

## What each page shows

- **index.html** — a directory of every mockup, one line each.
- **projects.html** — the main view, shown in its richest state at once: three rows
  selected with the batch toolbar visible, the details panel open on the right with
  notes, collections and a task list, one row mid-rename with the greyed file name, a
  plugin hover list rendered open (not just hoverable) on one row, and a right-click
  context menu rendered open and statically positioned on another row. The title bar
  carries an archived-projects toggle; the status bar is shown mid-scan with a progress
  bar, status message, and the selection count.
- **projects-tag-manager.html** — the projects view with the tag manager modal open:
  create, inline rename (one row shown mid-edit), delete, and usage counts per tag.
- **collections.html** — the card grid with a grid/list layout switcher.
- **collection-detail.html** — one collection opened: cover art, description, a
  reorderable tracklist with two rows selected, and the consolidated task list across
  its projects.
- **plugins.html** — paginated list with search, format and vendor filters, a tri-state
  installed toggle, and a status bar with totals. One row shows an open tooltip with
  version and SDK details. The installed column renders three distinct states
  (installed, missing, unknown) as both a coloured dot and a pill.
- **samples.html** — the same shape over samples: present/missing/unknown indicator,
  a truncated path with an open tooltip on one row, an extension filter, a status bar
  with a size breakdown by file type, and an open context menu with a native-only
  show-in-Explorer entry.
- **stats.html** — the dashboard: overview cards, then musical analytics, plugin and
  sample analytics, activity, insights, collection and tag analytics, and task
  completion, in that order, with a date-range and collection/tag filter bar above them.
  Charts are hand-drawn CSS (bar rows, a histogram of divs, and conic-gradient donuts) —
  no charting library.

Every page carries the shell: a left sidebar with the five views and a settings entry
pinned at the bottom, a title bar, and an Explorer-style status bar at the very bottom
of the window.

The three native-only features from ADR-0030 (open in Ableton, show in Explorer,
import from an arbitrary path or by drag-and-drop) appear in the mockups but are always
marked with a small "native" badge, since the browser build hides them until the Tauri
trial resolves.

Minimal inline JavaScript handles only cosmetic toggling (there is one `createIcons()`
call per page for Lucide, plus a line on the projects page that shows the batch
toolbar). Nothing fetches data or persists anything; every value on every page is
hand-written fake data.

## Ambiguities resolved

The sketch and the architecture doc describe behaviour and content; a few layout
questions needed a concrete answer to draw anything. What follows is what was chosen
and why, so it is cheap to override once real component work starts.

- **Where the plugin/sample hover list statically "opens."** The spec says hover (or
  click) opens a scrollable list of plugins or samples with per-item status. Since a
  static mockup cannot hover, one row's popover is given a `static-open` class so it
  renders expanded permanently; every other row still has the CSS `:hover` behaviour
  wired up so it's clear the two are the same component in two states.
- **Context menu positioning.** "Statically positioned" was read literally: the menu is
  an absolutely-positioned element with fixed `top`/`left` values sitting over a
  specific row, rather than something JS opens on an actual right-click. A real
  right-click handler is a React concern, not a mockup one.
- **Details panel width and docking.** The sketch says "toggleable, docked right,
  similar to Windows Explorer" without a width. 340px was chosen: wide enough for the
  notes textarea and task checkboxes without crowding the table below a typical laptop
  width.
- **Batch toolbar placement.** Placed directly above the table, below the drop-target
  hint, matching "appears above the table" in the architecture doc. It pushes the table
  down rather than overlaying it, so row content is never hidden by the toolbar.
- **Archived-projects filter.** The sketch describes both "a toggle" and "a filter"
  interchangeably. A toggle switch in the title bar was chosen over a separate filter
  dropdown, and one archived row is shown dimmed and italicised inline in the table
  (rather than in a separate view) to keep the single-table mental model from the rest
  of the page.
- **Tri-state installed/present filters.** Rendered as a three-button segmented control
  (all / installed / missing, or all / present / missing) rather than a dropdown, since
  three mutually exclusive states read better as visible buttons than as a hidden list,
  and it matches the tri-state nature called out for the underlying data (ADR-0012).
  The "unknown" state is never a filter option, only a value a row can have — filtering
  only distinguishes what the user actually wants to slice on (installed vs. not),
  matching the API's three-value filter.
- **Inline rename affordance.** The sketch says a pencil appears "on hover." One row is
  shown already in an editing state (input box focused, original title struck through
  and greyed, pencil still visible) to demonstrate the mid-edit look, since a static
  page cannot show a hover transition.
- **Cover art and vendor logos.** Neither exists as real assets. Both are rendered as
  a grey placeholder box with a small icon (an image icon for cover art, a plug icon
  implied by context for plugins/vendors), per the brief.
- **Stats charts.** The spec calls for pie charts, histograms, bar charts, a heatmap and
  a word cloud. Pies became CSS `conic-gradient` donuts (no library, still readable at
  a glance); the word cloud became a horizontal bar list, since a real word cloud needs
  a layout algorithm that has nothing to do with the mockup's purpose and a bar chart
  communicates the same ranking more legibly.
- **Search advanced operators.** The sketch mentions "advanced search operators" without
  specifying syntax. The search box placeholder text sketches an example
  (`tag:`, `key:`) as a hint rather than committing to a real grammar, which is a
  backend and parsing question outside a mockup's scope.
