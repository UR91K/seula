# 0052. The frontend picks its view with a signal, not a router

- **Status:** Accepted — implemented on branch `tauri-app`
- **Recognized:** 2026-10-04, building the second view (plugins), which is the first time
  the sidebar had anything to switch between
- **Decided:** 2026-10-04, accepted by the maintainer after review (proposed the same day)
- **Recorded:** 2026-10-04
- **Confidence:** decided now; drafted by the assistant while implementing, then reviewed
  against the code and accepted by the maintainer
- **Evidence:** ADR-0047 ("Not decided here: routing"); ADR-0048 (one Tauri window, no
  browser target); ADR-0031 (preferences live in the database); the `web/solid/src/shell/`
  directory (the `route` signal in shell.ts, the `View` contract in view.ts) and the
  App component beside it

## Context

ADR-0047 left routing open. Until the plugins view there was one screen, so nothing
needed it. The shell now has a sidebar of five entries, and two are implemented as views (projects
and plugins).

A router earns its place by giving a page a URL: deep links, the back button, reload to
the same place, sharing. The frontend is a single Tauri window that loads one page
(ADR-0048). It has no address bar and no browser history UI, and no other program links
into it. None of those four benefits applies today.

## Decision

**The active view is a signal.** `route`, in shell.ts, holds `"projects"` or
`"plugins"`. The sidebar sets it. There is no router library and no URL handling.

**A view is an object with the same few members.** The `View` type, in view.ts, is the
toolbar row, the main area, the inspector (or none), its status-bar segments, its
popovers, and the top-bar search it owns. The App component renders the active view's
members inside the one frame. The frame knows nothing about any view's data.

**Only the active view is mounted, and it loads its data when it mounts.** Each view's
state lives in module-level stores, so leaving a view and coming back finds its filters,
sort, search and selection as they were. Its list is fetched again on every visit. Nothing
enforces this: the `View` type does not require it, so a view must keep its state outside
its `Content` component or it will be lost on unmount.

**Crossing views is a function that sets the route and the target's query.** "Show the
projects using this plugin" switches to projects and searches `plugin:"<name>"`, using
the search syntax the daemon already has.

## Rejected alternatives

- **`@solidjs/router`.** Rejected because nothing it provides applies in one Tauri window
  with no address bar (above), and it is a dependency whose behaviour has to be understood
  to use correctly, which ADR-0048 says the maintainer does not want in the background.
  Cheap to adopt later if the premise changes.
- **A hash-based router written by hand.** Rejected as the same machinery for the same
  absent benefits.
- **Every view mounted at once, the inactive ones hidden.** Would keep scroll position and
  focus across switches for free. Rejected because each hidden view would still hold
  streams and effects open (the scan and watcher feeds, ADR-0038), and every view would
  load its data at startup whether or not it is visited.

## Consequences

No deep links, no back or forward, and no return to the last view after a restart. If the
last view should persist, it is a preference and belongs in the database (ADR-0031), not a
URL.

Views currently reach into each other: the plugins state calls `setRoute` and a
projects-side `showInProjects`. That is fine with two views. With five, crossing views
should go through one function in the shell rather than views importing each other.

Scroll position inside a view is lost when switching away from it, because its DOM is
discarded. Selection and filters survive. Whether the scroll loss is noticeable has not
been tested at a real library size.

Each visit re-fetches the view's list. Against the loopback daemon this was not
noticeable on the 240-project mock library; it is unmeasured on a large one.

## Notes

ADR-0044 has a collection open into its own view, which needs a parameter (which
collection). The first step is to widen `route` from a name to `{ view, id }`, still a
signal. This is small but not free: `RouteId`, the `counts` store keyed by it, and the
`VIEWS` map in the App component all change, so ADR-0044's "the shell does not change" is
slightly optimistic.

A router is worth reconsidering if back and forward inside the app are wanted, or if
ADR-0048's Tauri trial fails and a browser build is made, since the address bar and URLs
then exist and the premise above no longer holds.
