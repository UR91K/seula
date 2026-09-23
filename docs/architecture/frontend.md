# Frontend

Shape only. For *why*, follow the ADR links. Stack and placement are ADR-0029, the
browser-versus-native boundary is ADR-0030, preference storage is ADR-0031, the shell
frame and its density are ADR-0032.

This restates `docs/archive/FRONTEND_SPEC.md` (written 2025-08-14, verified still
accurate by the maintainer on 2026-09-16) against the HTTP surface that now exists. The
archive copy is frozen; this is the mutable home. The shell and density below supersede
the sketch's account of both, per ADR-0032.

## Status

Static HTML mockups come first, in `mockup/` at the repo root (ADR-0029 named
`docs/mockups/`; the first round there was rejected and is on the `old-mockups` branch).
The shell is built alone first and reviewed before any view: `mockup/shell.html`. Then
one review board per view, one view at a time, drawn from shared component renderers
(ADR-0036). Colours live only in `mockup/colors.css`, which the maintainer edits by
hand. Mockup data is a snapshot of the real HTTP API over a seeded database, made by
`mockup/data/generate.py` from the same `src/database/schema.sql` the program compiles
in. No React exists yet. See `docs/status.md` for where things stand.

## What it is

A React + TypeScript single-page app, built by Vite, in `web/`, talking only to the HTTP
API from ADR-0024 on `http_port`. It has two build targets, **browser** and **Tauri**,
that differ only in whether three native-only features are shown (ADR-0030).

## Density

Not decoration. The target is Windows 7 Explorer, and it is stated in numbers because
"denser" is not reviewable (ADR-0032). The numbers are relative, anchored to the root
font size, so the whole UI scales by changing that one value:

| | |
|---|---|
| Root font | 9pt Segoe UI, one family everywhere. `1rem` is this size. |
| Type scale | Two sizes in the body of the app: `1rem`, and one step up for headings |
| Table row | `1.5rem` — a `7/6rem` line box, `1/6rem` padding top and bottom |
| Table header | `11/6rem` |
| Cell padding | `2/3rem` horizontal, `1/6rem` vertical |
| Bar heights | Sized to content |
| Whitespace | Structural only: it separates regions, it does not decorate rows |

At the stated root, where `1rem` is 12px, those are 18px rows on a 14px line box, a 22px
header, and 2px and 8px of padding. Each is a whole number of sixths of the root font
size, so the relative form is exact, not rounded. Author against that sixth as a token —
`--unit: calc(1rem / 6)` — and the values above are 9, 7, 1, 11 and 4 units.

**No px anywhere in the stylesheet, except `1px` hairline borders.** A pixel value is
one that will not move when the root font size does, which defeats the whole
arrangement. A hairline border is the exception, because a border's weight is a
rendering detail rather than a measure of the design, and a scaled-up hairline just
looks heavy.

The CSS is the project's own. The metrics are borrowed from 7.css, but 7.css itself is
not used, and neither is any other reference stylesheet (ADR-0032). The aim is a dense
UI inspired by Windows system software, not a Windows 7 recreation. With nothing to
inherit the metrics from, they are written and maintained by hand, which is why they are
recorded here.

## Shell

Two bars span the full window width, edge to edge. Everything else sits between them, in
three columns. This is the VS Code arrangement and it is present in every view.

```
+---------------------------------------------------------------+
| logo   File Edit View   [ search ]        settings   - [] x    |  top bar
+--------+----------------------------------------+-------------+
| side   | view toolbar: title, filters, toggles  |             |
| bar    +----------------------------------------+  inspector  |
|        | main area                              |             |
+--------+----------------------------------------+-------------+
| 412 projects | 3 selected | scanning...                        |  status bar
+---------------------------------------------------------------+
```

| Element | Behaviour |
|---|---|
| **Top bar** | Application-level and constant: logo, menu bar, the global search box, settings, and the window controls on the Tauri target. Its contents do not change between views. |
| **Sidebar** | One entry per view, icon and label. Collapses to icons only, never to zero width, so navigation is always one click away. Collapse state persists (ADR-0031). |
| **Main area** | Opens with a thin view toolbar — view title, that view's filters, the inspector toggle — then the view itself. Per-view chrome lives here, not in the top bar. |
| **Inspector** | Right-hand panel, one component across all four content views, showing whatever is selected. Same width, same toggle, same position throughout. Stats has none. |
| **Status bar** | View-specific counts. Selection count when rows are selected. During a scan, a progress bar and the current message from the SSE stream at `/api/v1/system/scan-status`. At the right end, the ♯/♭ key spelling switch (ADR-0035), where Explorer-lineage apps keep view toggles. |

Search is global and lives in the top bar. On the projects view a non-empty search
switches the table from `/api/v1/projects` to the ranked results of `/api/v1/search`;
elsewhere it drives that view's own search route.

Icons come from an icon library. No emoji anywhere in the UI (ADR-0029).

## Views

Five. The first four are the product; stats is last.

### Projects

The main view. A data table of projects backed by `/api/v1/projects` (all) or
`/api/v1/search` (ranked results when the search box is non-empty).

**Table.** Click a header to sort. Paginated with an items-per-page selector. Column
widths resizable, columns reorderable and hideable; all of that persists via ADR-0031. A
header checkbox selects all on the page.

**Fixed leading cells** (not reorderable): play button for the audition audio, shown
only when the project has one, with the parent collection's cover art behind it when the
project is in exactly one collection; a plus button to add audio when it has none; then
the display name, with a pencil on hover to rename inline (`/api/v1/projects/:id/name`),
and the file name greyed beside it when the two differ; then tags.

**Reorderable columns:** Ableton version, created, modified, length as MM:SS, tempo, key
and scale, time signature, plugins, samples. Plugins and samples show a count that
expands on hover to a scrollable list with per-item installed/present status. The same
lists appear in the inspector, which is where they belong when the panel is open.

**Batch toolbar.** Appears above the table when more than one row is selected: archive
(`/api/v1/projects/batch-archive`); delete, enabled only when every selected project is
already archived (`/api/v1/projects/batch-delete`); add tags with an autocomplete picker
(`/api/v1/tags/batch-tag`); remove tags, listing only tags common to the selection
(`/api/v1/tags/batch-untag`); add to collection with a picker and a create-new option
(`/api/v1/collections/:id/batch-add`); remove from collection, listing common
collections; create collection from selection.

**Inspector.** The shared right-hand panel, toggled from the view toolbar. For a project
it shows everything the row shows plus its audition audios, notes
(`/api/v1/projects/:id/notes`), the collections the project is in, and its tasks with
per-task checkboxes, select-all, mark complete/incomplete
(`/api/v1/tasks/batch-update-status`) and delete selected
(`/api/v1/tasks/batch-delete`).

**Audition audios** (ADR-0037). A project can hold several, listed in the inspector in
the order added (`/api/v1/projects/:id/audio-files`), each with a play button, and a
radio for the one the row's play button plays (`PUT /api/v1/projects/:id/audio-file`).
Removing one from the list (`DELETE /api/v1/projects/:id/audio-files/:media_file_id`)
hands the row to the next. The row shows the add button when nothing is set to play.

**Context menu** on a row: add tag (existing tags, new tag, manage tags); open in
Ableton; show in Explorer; rename; add audio demo; add to collection; archive, or
unarchive when already archived. The three marked below are native-only. Single-item
entries are disabled when several rows are selected.

**Archived projects.** A toggle or filter shows archived projects. From there they can
be reactivated (`/api/v1/projects/:id/reactivate`) or permanently removed from the
database with a confirmation that says the file is untouched
(`/api/v1/projects/:id/permanent`).

**Tag manager.** A modal reached from the context menu. Create, rename, delete, each
showing usage counts from `/api/v1/tags/with-usage`. Rename cascades; delete confirms.

**Import.** An add-project button and a drop target for `.als` files from outside the
watched paths, via `/api/v1/projects/add`. See the native-only note.

### Collections

A grid of cards, like a media library, with alternative list layouts available. Each
card: cover art (`/api/v1/collections/:id/cover-art`), name, project count, estimated
duration, created date. Counts and duration come from
`/api/v1/collections/:id/statistics`.

Selecting a card fills the inspector: the card's data plus description, a
drag-to-reorder tracklist (`/api/v1/collections/:id/reorder`) with remove-selected, and
a consolidated task list across all contained projects (`/api/v1/collections/:id/tasks`)
with the same operations as the project inspector. Whether opening a card *also*
navigates to a full-width detail view, or whether the inspector is the whole of it, is
the one shell question ADR-0032 leaves to the mockup.

### Plugins

A paginated list from `/api/v1/plugins`. Per row: name, vendor, format, installed status
as an icon and a colour (never colour alone), project count (ADR-0034). The list can be
grouped by vendor or by format: sorted on that column, with a header row per group that
carries the group's totals from `/api/v1/plugins/vendors` or `/api/v1/plugins/formats`
(plugins; installed, missing and not scanned; projects using them).

Selecting a row fills the inspector from `GET /api/v1/plugins/:id`, which returns the
row and `details`: everything the scan recorded (path, category, channels and buses,
presets, parameters, latency, GUI, vendor URL, the format's own extras, the VST3
classes, when it was last scanned), and the references projects know the plugin by, with
the name each project gives it. Below that, the projects using it
(`/api/v1/plugins/:id/projects`). A plugin no scan has found has no scanner data, and
the inspector says why instead of showing empty fields.

Filters, in the view toolbar: format dropdown, vendor dropdown, and an installed filter
that takes any of installed, missing and not scanned (`install_states`, ADR-0025). The
top-bar search goes to `/api/v1/plugins/search` on this view (name, vendor, format). The
status bar shows total, installed, missing, not-scanned and unique-vendor counts from
`/api/v1/plugins/stats`, passed the same filters so it counts what the list shows.

"Scan plugins" in the toolbar starts `POST /api/v1/plugins/scan` (ADR-0038). Progress
shows in the status bar like a project scan's, and when the stream ends the list and the
counts are fetched again. The button is disabled while any scan runs.

Context menu: Show in Explorer (installed plugins, native-only), Show projects using it
(the projects view, searching `plugin:` for it), Copy name.

The installed flag is tri-state (ADR-0012). "Not scanned" is a real state the UI must
render distinctly from "missing".

### Samples

Same shape as plugins over `/api/v1/samples`. Per row: name, the folder as a truncated
path with the full path in a tooltip, format, present status, size, project count
(ADR-0034). No grouping; a folder tree is proposed for later (ADR-0042).

Selecting a row fills the inspector from `/api/v1/samples/:id`: the full path, format,
size and when a check last measured it, and the projects using it
(`/api/v1/samples/:id/projects`). A missing sample shows the size it last had.

Filters, in the view toolbar: format dropdown from `/api/v1/samples/formats`, where AIFF
covers `.aif`, `.aiff` and `.aifc` (ADR-0039), and a present filter (all, present,
missing). The top-bar search goes to `/api/v1/samples/search` on this view (name and
path). The status bar shows total, present, missing and measured size from
`/api/v1/samples/stats`, passed the same filters. Until a check has measured the
library, the size says so rather than showing zero.

"Check samples" in the toolbar starts `POST /api/v1/samples/check` (ADR-0041): whether
each file is still there, and its size. Progress shows in the status bar like the other
scans; the button is disabled while any scan runs.

Context menu: Show in Explorer and Play, both native-only (ADR-0030), Show projects
using it, Copy path.

Project counts and used-in lists here and in the plugins view take `scope=active|all`
(ADR-0040). The choice is a preference, active by default; under `all`, archived
projects in a used-in list are marked.

### Stats

Lowest priority. A dashboard from `/api/v1/system/statistics`, with CSV export via
`/api/v1/system/statistics/export`. Sections, top to bottom: overview cards (counts of
projects, collections, plugins, samples, tags, tasks with completion rate); musical
analytics (tempo histogram, key and time-signature distributions, average duration);
plugin and sample analytics (top ten of each, top vendors, averages per project);
activity (projects per year and per month, recent activity); insights (most complex
projects, longest project, share under forty seconds, Ableton version distribution);
collection and tag analytics; task completion. A date-range selector and filters by
collection or tag apply across sections.

## Native-only features

Per ADR-0030, these exist in the Tauri target and are hidden in the browser target until
that trial resolves:

| Feature | Why the browser cannot | Where it appears |
|---|---|---|
| Open in Ableton | launches a program | project context menu |
| Show in Explorer | reveals a path | project, plugin and sample context menus |
| Play a sample | the file is on the local disk, and the API serves only stored media (ADR-0033) | sample context menu and inspector |
| Import from an arbitrary path, including drag and drop | a browser drop yields bytes, not a path; `/api/v1/projects/add` needs a path | projects view import |

## Live updates

Two SSE streams from ADR-0024. Scan progress streams from the request that starts the
scan (`POST /api/v1/system/scan`, `POST /api/v1/plugins/scan`), and
`GET /api/v1/system/scan-status` answers the current state to a window that did not
start it; one scan runs at a time (ADR-0038). Watcher events at
`/api/v1/system/watcher/events` invalidate the projects table.

## Preferences

Column layout, page size, the sidebar collapse state and the inspector state are read
and written as one blob through the HTTP surface and stored in the `app_state` table
(ADR-0031). The endpoint for that does not exist yet; it is the one backend addition the
frontend needs.

## Not covered by the API yet

Found while mapping the sketch onto the routes. Each is small; none blocks the mockups.

- A get/set pair for the UI preferences blob (ADR-0031).
- Tag rename cascade is assumed to be the existing tag update route; verify it rewrites
  `project_tags` rather than creating a new tag.
- `GET /api/v1/media/:id` serves media bytes, streamed and with `Range` support
  (ADR-0033). Image and audio URLs are built from `cover_art_id` and `audio_file_id`.

## Keys

Every key the API sends, whether on a project, in the statistics or as a collection's
most common key, has four fields: `tonic` and `scale` as enum names for the filters, and
`sharp` and `flat` as display strings such as "F♯ Minor" and "G♭ Minor" (ADR-0035). The
UI shows one according to a sharp/flat switch, stored with the other preferences
(ADR-0031). It never formats a key itself.
