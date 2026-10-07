# 0060. Tracklist reordering uses pointer events, not native drag and drop

- **Status:** Accepted — implemented on branch `tauri-app`
- **Recognized:** 2026-10-05, planning the collections view
- **Decided:** 2026-10-05, agreed by the maintainer
- **Recorded:** 2026-10-05
- **Confidence:** decided now; the reason is the assistant's reading of how HTML5 drag and
  drop behaves for row reordering, not something measured in this app
- **Evidence:** ADR-0044 (drag to reorder, handles hidden unless the sort is `#`);
  `PUT /api/v1/collections/:id/reorder`, which takes the full `project_ids` list

## Context

The tracklist needs row reordering with a handle and a drop indicator. HTML5 drag and drop
is the obvious tool, but it gives little control over the drag image and the drop
indicator, and it behaves differently across webviews.

## Decision

Use pointer events on the handle: capture the pointer, track which row boundary it is
nearest, draw the indicator there, and on release send the new order with one `PUT
.../reorder`. The handles exist only while the sort is `#`, because a reorder under any
other sort has no meaning.

## Rejected alternatives

- **Native HTML5 drag and drop.** Rejected for the control reasons above. Not tested in
  this app.
- **A drag-and-drop library.** Rejected as a dependency for one interaction.

## Consequences

Keyboard reordering is not provided by this decision. If wanted, it is a separate piece of
work, and the handle should be focusable so that it can be added.
