# 0032. The shell is two full-width bars around a VS Code column layout, at Windows 7 density

- **Status:** Accepted
- **Recognized:** 2026-09-17, when a mockup built from `docs/architecture/frontend.md`
  came back wrong-shaped
- **Decided:** 2026-09-17
- **Recorded:** 2026-09-17
- **Confidence:** decided now
- **Evidence:** the rejected mockup round, kept on the `old-mockups` branch (commit
  `cc6c3fd`); `docs/architecture/frontend.md` before this commit (the shell table that
  placed the title bar "top of content"); ADR-0029 (mockups precede React precisely so
  layout questions get answered cheaply); ADR-0031 (where the collapse state persists)

## Context

`docs/architecture/frontend.md` described the shell in three table rows: a left sidebar,
a bottom status bar, and a title bar at "the top of content". A mockup was generated
from that description and the maintainer's reaction was that it was not quite right. It
is kept on the `old-mockups` branch rather than on main.

Its defining fault was inconsistency: views that should have shared a frame did not,
because the spec never pinned the layout and structure down, so the generating agent
guessed per page and built whatever seemed right each time. Two things were wrong, and
only one of them was visible in the prose.

The visible one is nesting. "Top of content" puts the title bar inside the content
column, to the right of the sidebar, so the sidebar runs from the top of the window down
to the status bar and the two horizontal bars do not line up. The maintainer wants the
opposite: the top bar and the status bar both span the entire window, and the sidebar,
main area and right panel sit *between* them. That is the VS Code arrangement, and it is
also what every Explorer-lineage application does.

The invisible one is density, and it is the more damaging of the two. The spec said
nothing whatsoever about row height, type scale or padding. A generator handed a layout
brief with no density brief will produce today's defaults: 40px rows, 16px padding, four
or five font sizes doing semantic work. The result reads as a modern web dashboard,
which is the one thing this application is not. The sketch's whole design language is
Windows Explorer with a file list in it; the intended feel is Windows 7 era, where a row
is a line of text plus a couple of pixels and the entire window uses one font at one or
two sizes.

Nothing in the document stated that, so nothing in the document could be followed to it.
The mockup was a faithful rendering of an underspecified brief.

## Decision

**The frame.** Two bars span the full window width, edge to edge, and are present in
every view:

- A **top bar**: logo, menu bar, the global search box, a settings entry, and the window
  controls on the Tauri target. Its contents do not change between views.
- A **status bar**: view-specific counts, selection count when rows are selected, and scan
  progress fed by the SSE stream.

Between them sit three columns: the sidebar, the main area, and the inspector.

**The sidebar** is one panel with an icon and a label per view. Collapsed, it keeps the
icons and drops the labels; it never collapses to zero width, so navigation is always
one click away. The collapse state persists per ADR-0031.

**The inspector** is one component, on the right, in every view that has a selection. It
shows the selected project, collection, plugin or sample. Same width, same toggle, same
position throughout. Stats has no inspector.

**Per-view chrome** — view title, filters, the inspector toggle, the batch toolbar —
lives in a thin toolbar row at the top of the main area, not in the top bar. The top bar
is application-level and constant.

**Density.** The target is Windows 7 Explorer, stated in numbers so it is checkable. The
numbers are relative, anchored to the root font size, so that the whole UI can be scaled
by changing that one value:

| | |
|---|---|
| Root font | 9pt Segoe UI, one family everywhere. `1rem` is this size. |
| Type scale | Two sizes in the body of the app: `1rem`, and one step up for headings. Not five. |
| Table row | `1.5rem`: a `7/6rem` line box plus `1/6rem` padding top and bottom |
| Table header | `11/6rem` |
| Cell padding | `2/3rem` horizontal, `1/6rem` vertical |
| Bar heights | Top bar and status bar sized to their content, not to a round number |
| Whitespace | Structural only. Gaps separate regions; they do not decorate rows. |

At the stated 9pt root, where `1rem` is 12px, those are 18px rows on a 14px line box, a
22px header, and 2px and 8px of padding. Each is a whole number of sixths of the root
font size, so the relative form is exact rather than rounded, and `calc(1rem / 6)` is
the unit the design is actually built on: rows are 9 of it, headers 11, the line box 7,
cell padding 4 across and 1 down.

**Nothing is in absolute units.** A px value anywhere in the stylesheet is a bug,
because it is a value that will not move when the root font size does. The one exception
is a `1px` hairline border: a border's weight is a rendering detail rather than a
measure of the design, and a hairline that scales with the root font just looks heavy.

**No reference stylesheet.** The first mockup round derived its CSS from two vendored
references, Pokémon Showdown's `panels.css` and an app-surface extension of it
(`example-styles/`, added in `5cc8b26`). Both are deleted, deliberately: the maintainer
believes they drove much of that round's inconsistency, and the target is a denser,
Windows-system-inspired UI that neither describes. The mockups' `shared.css`, which
ADR-0029 called the seed of the real stylesheet, went to the `old-mockups` branch with
them. The seed is now the numbers in this ADR, written fresh.

## Rejected alternatives

- **Keeping the title bar inside the content column.** Rejected because it is what
  produced the wrong mockup, and because a status bar that spans the window under a title
  bar that does not is visibly lopsided. The full-width frame also gives the Tauri target
  somewhere to put window controls without a second row of chrome.
- **A VS Code-style activity rail beside the sidebar.** Rejected as one element too many
  for five views. The rail earns its place in VS Code because the sidebar's contents
  change completely per activity; here the sidebar is a fixed list of five destinations,
  so collapsing that same list to icons does the same job with one element instead of two.
- **A sidebar that collapses to zero width.** Rejected because the main area gains perhaps
  150px and loses the ability to switch views without a detour.
- **Two stacked full-width bars, app chrome then view chrome.** Rejected because it spends
  a second full-width row of vertical space on something only the main area needs, in a
  design whose entire premise is that vertical space is for rows.
- **An inspector on the projects view only, as originally specced.** Rejected because
  plugins and samples both have per-row detail the spec was smuggling into hover tooltips,
  and a hover tooltip is a worse inspector that has to be reinvented per view. One
  component, four views.
- **Leaving density to the mockup and judging it by eye.** Rejected because that is what
  just happened. "Looks not quite right" is not reviewable and does not survive being
  handed to a fresh generator; a row height stated as a fraction of the base font is both,
  and unlike a pixel value it survives the whole UI being scaled up.

## Consequences

The spec now constrains appearance, not just structure. That is the point, and the cost
is that a density change is now a documented change rather than a stylesheet tweak.

Small rows raise real accessibility questions. A `1.5rem` row is 18px at the stated
root, which is below any touch target guidance and close to the floor for a mouse. This
is a single-user desktop application for its maintainer, which is the only reason that
is acceptable, and it should be reconsidered immediately if the audience ever widens.
Relative units are the mitigation that is actually available: the maintainer can raise
the root font size and every row, header and gap grows with it.

The inspector becoming general means four views must describe what they put in it. That
work lands in `docs/architecture/frontend.md`, not here.

The collections view's separate detail page is now in tension with the inspector.
Resolving that is left to the mockup.

## Notes

The numbers above are taken from 7.css, a CSS recreation of the Windows 7 UI. They were
not measured from Explorer itself. 7.css is the source of the numbers only; it is not a
dependency and is not to be pulled in. The aim is a dense UI inspired by Windows system
software, not a Windows 7 costume, so the look is the project's own and only the metrics
are borrowed. With no base stylesheet to inherit them from, these values have to be
written and held by hand, which is the reason for stating them here at all.

Cheap to reverse while the mockups are static HTML, which is why ADR-0029 put the
mockups first. After the React shell exists, the frame is a layout rewrite and the
density is a stylesheet rewrite. Do both before, not after.
