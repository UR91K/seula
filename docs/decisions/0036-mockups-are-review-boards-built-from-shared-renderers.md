# 0036. Mockups are one review board per view, built from shared renderers, one view at a time

- **Status:** Accepted
- **Recognized:** 2026-09-23, when the shell was approved and the first view was next
- **Decided:** 2026-09-23
- **Recorded:** 2026-09-23
- **Confidence:** decided now
- **Evidence:** the rejected first round on the `old-mockups` branch (commit `cc6c3fd`),
  whose pages each drew their own frame and drifted apart; `mockup/shell.html`,
  `mockup/shell.js` (the approved shell, one render function for every frame);
  `mockup/reference-mockups/nekko-mockups.html` (the maintainer's earlier board layout,
  gitignored and not in the repo); ADR-0029 (mockups before React); ADR-0032 (the frame
  and density the boards are checked against)

## Context

ADR-0029 puts static mockups before any React, so that layout questions are answered
where they are cheap. The first round showed how mockups go wrong. Each page was built
separately and drew its own chrome, so the same element came out slightly different on
every page, and the review could only react to the whole thing ("not quite right").

The second round started with the shell alone, drawn by one function and approved before
any view. That worked, and it is the model for the rest. What was still undecided was
how a view gets reviewed. A single screen shows each component in one state only, so a
context menu with disabled items, or an inspector with nothing in it, never gets looked
at until React is built.

The maintainer's previous contract work used a board layout (nekko, a mobile app): a
canvas of frames, each labelled with its number, its state and a note. The maintainer
asked for the same here, adapted to a desktop screen with separate components.

## Decision

**One board per view, and one view at a time.** Each view gets its own page, such as
`mockup/projects.html`. A view is built, reviewed and changed until the maintainer
approves it, before the next one starts. The order is projects, plugins, samples,
collections, stats: projects settles the table, inspector, menus and pickers that the
rest reuse.

**Board layout.** The page is one large canvas:

- The **screen column** on the left: the full window in its default state, then each other
  state of the whole screen stacked below it (selection, batch, search, archived, empty,
  and so on).
- **Component columns** beside it, one per component (inspector, context menu, batch
  toolbar, pickers, dialogs, hover lists), each with its states stacked below.
- Every frame is labelled in the nekko style: a number, the state, and a short note on
  what to look at, such as "2b · Inspector, no tasks — the empty state of each section".

**Components are drawn at their real size**, at the same root font as the screen, so
their density can be compared with the screen next to them.

**One renderer per component.** A component in a side column and the same component
inside the screen come from the same function, in a shared `mockup/components.js`, never
a copy. This is the rule that prevents the first round's drift. The shell rendering
stays in `shell.js`, and a board page does no more than choose frames and states.

**One board frame.** The shared `mockup/board.css` and `mockup/board.js` own the canvas,
the columns, the frame labels and the board controls. A board page only declares its
frames.

**Board controls** apply to the whole page:

- **Theme toggle:** switches every frame on the board between dark and light. No frame is
  drawn separately for the light theme. Both themes are the same components under a
  different `colors.css` set, and a toggle compares them on every state rather than one.
- **Root font** (8–11pt): the app's own scaling from ADR-0032, applied to every frame.
- **Zoom:** scales the whole canvas visually so a wide board fits the window. It is
  separate from the root font, so zooming never changes what the density check measures.

**Screen frames are live; component frames are fixed.** The screen frames respond to
clicks, as the shell's do. A component frame shows one state and does not change.

**The existing rules still apply to every board:**

- Colours come from `mockup/colors.css` only, and only the maintainer edits them by hand.
- Every measure is in units of the root font; the only px value allowed is a 1px hairline
  (ADR-0032).
- All data comes from `mockup/data/mock-api.js`, the snapshot of the real API that
  `mockup/data/generate.py` produces. No invented data is written into a page. If a board
  needs data the API does not provide, that is an API gap to record, not something to
  fake.
- The icons are Material Symbols Outlined.

**Location.** The mockups live in `mockup/` at the repo root, not under `docs/mockups/`
as ADR-0029 said. The first round is kept only on the `old-mockups` branch.

## Rejected alternatives

- **One page for all views.** Rejected because reviewing one view at a time is the point.
  A page that grows with every view has no moment where one of them is finished.
- **Screen states only, no component columns.** Rejected because that is how component
  states go unreviewed until React.
- **Separate light-theme frames.** Rejected by the maintainer. They double the number of
  frames, but compare the themes on only the states that happen to get a light copy. A
  toggle compares every state, and cannot drift from the dark version.
- **Components copied into their side columns.** Rejected because it recreates the first
  round's failure on a smaller scale.
- **Zooming by changing the root font.** Rejected because the root font is the density
  setting under review. Zooming with it would make the 18px row check meaningless.

## Consequences

The shell board predates these rules. Its separate light frame (1b) is removed, and the
board moves onto `board.js` and the theme toggle, when the projects board introduces
them.

A component that first appears on one board, such as the inspector on projects, is
settled there. A later board reuses it, and a change to it later shows on every board
that uses it. That is intended, but it means an approved board can change when a later
one is being worked on, so a change to a shared component should be checked on the
boards that already use it.

The boards are documentation, not a build input (ADR-0029). React components are built
from them but do not import them, so `components.js` is a working reference, not code to
carry forward.
