# 0047. The frontend is built with Solid, not React, with Svelte 5 trialled against it first

- **Status:** Accepted, with a Solid and Svelte 5 comparison pending
- **Recognized:** 2026-09-24, from the mockups' redraw workarounds
- **Decided:** 2026-09-24
- **Recorded:** 2026-09-24
- **Confidence:** decided now
- **Evidence:** ADR-0029 (the stack this revises); `mockup/screens.js` (`draw()` rebuilds
  the whole shell through `innerHTML`, then restores scroll position and refocuses the
  rename field by hand), and the same `innerHTML` redraw in `mockup/plugins.js`,
  `mockup/samples.js`, `mockup/collections.js` and `mockup/stats.js`; ADR-0024 and
  ADR-0038 (the SSE streams); `src/watcher/file_watcher.rs` (the watcher's create, modify
  and rename events)

## Context

ADR-0029 chose React without weighing any other framework: React was what ADR-0028 had
named, and it has the widest pool of reference material. That ADR also said the choice
was cheap to reverse before the first view is written and expensive after. No view has
been written.

The desktop app is a Tauri shell around the axum daemon (ADR-0048). The daemon pushes
state the UI has to reflect live, and it arrives on schedules the UI does not control:

- **Scan progress** over SSE (ADR-0024). The project scan parses many projects in
  parallel (ADR-0002) and reports as one stream that can tick several times a second.
  The plugin scan and the sample check report the same way (ADR-0038, ADR-0041). One
  scan runs at a time.
- **Watcher events**, the create, modify and rename stream from the file watcher. These
  are independent of any scan and can arrive while one is running.

So the UI's state changes at two very different speeds: clicks, and pushes.

The interactive mockups proved the visual and interaction design with a hand-rolled
render loop. Each screen has one state object, a pure function turns it into an HTML
string, and `el.innerHTML = ...` runs on every change. That works for click-driven
interaction. Two things about it do not survive push-driven state:

- **Every change rebuilds the whole shell.** A single scan-progress tick rebuilds the
  sidebar, the table and the inspector along with the progress bar, whether or not any
  of them read anything the tick changed.
- **Anything not in the state object is destroyed on every redraw:** text-input focus
  and caret position, scroll offset, an in-flight selection. `mockup/screens.js`
  already carries hand-written workarounds for two of these. It saves and restores the
  scroll position around `draw()`, and refocuses the rename input after it has been
  torn down and rebuilt. Every future piece of live state would need the same kind of
  workaround, written by hand again.

A React component tree re-rendering from the top faces the same problem, handled
through memoisation and keys.

The maintainer's priorities, as stated: full traceability and fine-grained control over
familiarity, and no work happening in the background whose inner workings are not
understood.

A first draft of this decision was written outside the repository and argued for Solid
over Svelte 5 on three grounds: that Svelte has a runtime scheduler and Solid does not,
that Solid has the longer track record, and that Solid's core is a few hundred lines.
None of the three held up when checked, so none of them is part of this decision. What
remains is one real difference between the two, set out below, and the margin it gives
Solid is narrow.

## Decision

**Solid, with TypeScript, built by Vite with `vite-plugin-solid`,** replacing the
mockups' state-object, `draw()` and `wire()` pattern. Everything else in ADR-0029
stands: `web/` in this repository, hand-written CSS, no component library, an icon
library and never emoji.

**Before the first view is written, one screen is built in both Solid and Svelte 5**
and compared in practice. Solid is the expected outcome, not a foregone one. If the
comparison favours Svelte, a new ADR supersedes this one.

**Why a fine-grained framework at all.** Solid and Svelte 5 both compile markup into
code that builds the DOM directly, with no virtual DOM and no diff pass. A signal write
updates only the DOM nodes that read that signal, and nothing else is invoked to check
whether it needed to. A scan-progress event changes the progress text and nothing more,
and a table's scroll position and a field's focus are never at risk, because nothing
around them is rebuilt. That is what the mockups' workarounds are missing, and it is
common to both.

**Why Solid over Svelte 5.** In Solid, the only code the compiler transforms is the
markup. State logic, event handlers and derived values run as written, and reactivity
is ordinary function calls: `setCount(count() + 1)` is what executes, and it can be
stepped through in a debugger and read in the library source. Svelte 5's compiler also
rewrites the component's script. With runes, `count += 1` on a `$state` variable is not
what executes; the compiler turns it into a call into Svelte's runtime. That rewrite
can be inspected in the compiled output, but it is still a translation layer between
what was written and what runs. The maintainer does not want work happening in the
background that they do not fully understand, and this is the axis on which the two
frameworks differ.

**The cost Solid's approach carries, and why it is accepted.** Because the script is not
rewritten, Solid can only track reads that happen inside a tracking scope (JSX, a memo,
an effect). Destructuring props, or reading a signal outside a tracking scope, reads the
value once and never again: the UI silently shows a frozen value. That is a discipline,
and it is a mechanical one. `eslint-plugin-solid` catches the common cases and is part of
the setup. When a case slips through, the result is a visibly stale UI, not a hidden
slowdown, which is the trade the maintainer prefers.

**State is split by the speed it changes at:**

- **Click-speed state** (filters, sort, selection) lives in a `createStore`. A store
  tracks each property on its own, so a component that reads only the sort is not
  asked to re-run when the vendor filter changes.
- **Push-speed state** (a scan's progress, a watcher event) lives in its own
  `createSignal`, read only by the component that displays it. It is kept out of the
  store on purpose, so nothing that reads the store is ever asked whether a push
  concerned it.

## The comparison

One screen, built twice from the same mockup, against the real HTTP API. The screen
should carry both kinds of state, so the plugins view is the natural choice: a sortable,
filterable table with an inspector, and a plugin scan whose progress streams over SSE.
Worth comparing:

- how each reads, and how far the code that runs is from the code that was written;
- stepping through a state change in the debugger;
- what each gets wrong silently: Solid's tracking scope, Svelte's equivalents;
- how the two feel against the ADR-0032 density and the shared stylesheets.

No pass mark is set. The maintainer judges the result, and it is recorded in
`docs/status.md`.

## Rejected alternatives

| Option | Why not |
|---|---|
| **React (ADR-0029's choice)** | Its unit of update is the component re-render, reconciled against a virtual DOM. Updating only what changed takes deliberate memoisation (`memo`, `useMemo`, `useCallback`, stable keys), applied correctly everywhere it matters. That is a discipline to keep up, not a property the system guarantees, and working out *why* a given render happened is its own debugging problem. |
| **Preact with `@preact/signals`** | The closest to Solid in behaviour, and its React-shaped API means most tutorials carry over. Rejected because the fine-grained path applies only when a signal is read directly inside JSX. Read `.value` in a plain expression, destructure it or pass it through a variable, and it silently falls back to an ordinary component re-render and diff. That failure raises no error and shows only in profiling, where Solid's equivalent failure shows on screen. |
| **Vue (Composition API), or MobX** | Vue renders through a virtual DOM, so an update still runs a component's render and a diff, which is the step this decision removes. Its compiler-driven Vapor mode, which skips the virtual DOM, was not weighed. MobX is a state library, not a renderer, so it needs one, typically React. |
| **`lit-html` on its own, keeping the mockups' architecture** | Would solve the DOM-patching half of the problem, and leave conditional rendering, list diffing and component composition to be built by hand. Full Lit (`LitElement` with Shadow DOM) was ruled out separately, because Shadow DOM's style encapsulation fights the global stylesheets rather than working with them. |
| **Keep the mockups' pattern, or hand-roll a signals layer with no framework** | Legitimate for a solo project with no deadline, and the first is what produces the workarounds above. Rejected in favour of a framework that already does list diffing and conditional rendering, rather than writing and maintaining that layer by hand. |

Svelte 5 is not in this table. It is not rejected. It goes into the comparison as the
alternative that could still win.

## Consequences

Scan progress, watcher events and any future push-driven state bind directly to the DOM
nodes that display them. The scroll-restore and refocus workarounds in
`mockup/screens.js` have no equivalent in the port.

The stylesheets (`colors.css`, `shell.css`, `components.css`) and the `--u` unit from
ADR-0032 (`calc(1rem / 6)`, defined in `mockup/shell.css`) carry over unchanged. Neither
Solid nor Svelte needs style encapsulation to work.

Costs:

- **The comparison costs a screen built twice.** That is cheap next to discovering the
  wrong choice after every view is written.
- **A smaller ecosystem and community than React.** A small cost here, since ADR-0029
  already rejected component libraries and the mockups build every control from scratch.
- **A compile step.** Vite and `vite-plugin-solid`, where the mockups run from plain
  `<script>` tags.
- **The tracking-scope discipline** above, with `eslint-plugin-solid` as its guard.

The mockups' state-object, `draw()` and `wire()` code is reference material for the port,
not something upgraded in place. Their component renderers (`mockup/components.js`) are
HTML-string functions and carry across as structure and CSS, not as code. The review
boards (ADR-0036) stay, as the reference each ported screen is checked against.

## Not decided here

Routing, the HTTP and SSE client layer, build tooling beyond Vite, and the testing
strategy each get their own decision when they come up.

One fact the client-layer decision will need: the scan streams answer the `POST` that
starts the scan (`POST /api/v1/system/scan`, `POST /api/v1/plugins/scan`,
`POST /api/v1/samples/check`). The browser's `EventSource` can only make `GET`
requests, so it cannot read them. Either the client reads the SSE stream from a `fetch`
response body, or the daemon adds a `GET` route to follow a running scan. The watcher's
stream is a `GET` and has no such problem.

## Notes

Cheap to reverse until the first view is written in `web/`, as ADR-0029 said of React.
After that, a framework change is a rewrite of every component. That is why the
comparison comes first.
