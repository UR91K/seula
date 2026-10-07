# 0056. One shared dialog component

- **Status:** Accepted — implemented on branch `tauri-app`
- **Recognized:** 2026-10-05, planning the collections view
- **Decided:** 2026-10-05, agreed by the maintainer
- **Recorded:** 2026-10-05
- **Confidence:** decided now; small, and expected to be uncontroversial
- **Evidence:** the `.scrim` and `.dialog` styles in the collections mockup; ADR-0044
  (the new, edit, duplicate and delete dialogs); the Solid frontend, which has no dialog
  component yet

## Context

Collections need four dialogs: new, edit details, duplicate and delete. The mockup styles
them, and the Solid frontend has nothing to put in them.

## Decision

Build one `Dialog` component in the shell, which owns the scrim, the focus trap, closing
on Escape and on a click on the scrim, and the title and button row. Each dialog supplies
only its body and its actions. Later views (project delete, for one) use the same
component.

## Rejected alternatives

- **A dialog written inline per collection action.** Rejected: four copies of focus and
  Escape handling.
- **The native `<dialog>` element.** Not rejected on the merits; it may well be the
  implementation. This record fixes the component's contract, not its internals.

## Consequences

Toolbar controls that open a popover still need `data-keep`; a dialog is separate from the
popover click-away rule.
