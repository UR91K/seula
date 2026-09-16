# Frontend

Shape only. For *why*, follow the ADR links. Stack and placement are ADR-0029, the
browser-versus-native boundary is ADR-0030, preference storage is ADR-0031.

This restates `docs/archive/FRONTEND_SPEC.md` (written 2025-08-14, verified still
accurate by the maintainer on 2026-09-16) against the HTTP surface that now exists. The
archive copy is frozen; this is the mutable home.

## Status

Static HTML mockups under `docs/mockups/` come first. No React exists yet. See
`docs/status.md` for where things stand.

## What it is

A React + TypeScript single-page app, built by Vite, in `web/`, talking only to the
HTTP API from ADR-0024 on `http_port`. It has two build targets, **browser** and
**Tauri**, that differ only in whether three native-only features are shown (ADR-0030).

## Shell

Present on every view:

| Element | Behaviour |
|---|---|
| **Sidebar** (left) | One entry per view. Settings entry pinned at the bottom. |
| **Status bar** (bottom) | Explorer-style. Always shows view-specific counts. During a scan, shows a progress bar and the current status message, fed by the scan-status SSE stream at `/api/v1/system/scan-status`. When rows are selected, shows the selection count. |
| **Title bar** (top of content) | View title, search input, and on the projects view the details-panel toggle at the right. |

Icons come from an icon library. No emoji anywhere in the UI (ADR-0029).

## Views

Five. The first four are the product; stats is last.

### Projects

The main view. A data table of projects backed by `/api/v1/projects` (all) or
`/api/v1/search` (ranked results when the search box is non-empty).

**Table.** Click a header to sort. Paginated with an items-per-page selector. Column
widths resizable, columns reorderable and hideable; all of that persists via ADR-0031.
A header checkbox selects all on the page.

**Fixed leading cells** (not reorderable): play button for the audition audio, shown
only when the project has one, with the parent collection's cover art behind it when
the project is in exactly one collection; a plus button to add audio when it has none;
then the display name, with a pencil on hover to rename inline (`/api/v1/projects/:id/name`),
and the file name greyed beside it when the two differ; then tags.

**Reorderable columns:** Ableton version, created, modified, length as MM:SS, tempo,
key and scale, time signature, plugins, samples. Plugins and samples show a count that
expands on hover to a scrollable list with per-item installed/present status.

**Batch toolbar.** Appears above the table when more than one row is selected:
archive (`/api/v1/projects/batch-archive`); delete, enabled only when every selected
project is already archived (`/api/v1/projects/batch-delete`); add tags with an
autocomplete picker (`/api/v1/tags/batch-tag`); remove tags, listing only tags common
to the selection (`/api/v1/tags/batch-untag`); add to collection with a picker and a
create-new option (`/api/v1/collections/:id/batch-add`); remove from collection,
listing common collections; create collection from selection.

**Details panel.** Toggled from the title bar, docked right, Explorer-style. Shows
everything the row shows plus notes (`/api/v1/projects/:id/notes`), the collections
the project is in, and its tasks with per-task checkboxes, select-all, mark
complete/incomplete (`/api/v1/tasks/batch-update-status`) and delete selected
(`/api/v1/tasks/batch-delete`).

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

Opening a card shows the detail view: the card's data plus description, a drag-to-reorder
tracklist (`/api/v1/collections/:id/reorder`) with remove-selected, and a consolidated
task list across all contained projects (`/api/v1/collections/:id/tasks`) with the same
operations as the project details panel.

### Plugins

A paginated list from `/api/v1/plugins`. Per row: name, vendor, format, installed status
with a visual indicator, usage count, project count. Clicking usage opens the project
list at `/api/v1/plugins/:id/projects`. Hover on the name shows version and SDK details.

Filters: text search on name and vendor (`/api/v1/plugins/search`), format dropdown
(`/api/v1/plugins/formats`), vendor dropdown (`/api/v1/plugins/vendors`), and a
tri-state installed filter (all, installed, missing). The status bar shows total,
installed, missing and unique-vendor counts from `/api/v1/plugins/stats`, recomputed
for the current filter.

The installed flag is tri-state (ADR-0012). "Unknown" is a real state the UI must
render distinctly from "missing".

### Samples

Same shape as plugins over `/api/v1/samples`. Per row: name, truncated path with the
full path in a tooltip, extension, present status, usage count, project count. Filters:
search (`/api/v1/samples/search`), extension dropdown (`/api/v1/samples/extensions`),
tri-state present filter. Status bar: total, present, missing, unique-path count,
estimated total size and a breakdown by type from `/api/v1/samples/stats` and
`/api/v1/samples/analytics`. Context menu has show-in-Explorer, native-only.

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

Per ADR-0030, these exist in the Tauri target and are hidden in the browser target
until that trial resolves:

| Feature | Why the browser cannot | Where it appears |
|---|---|---|
| Open in Ableton | launches a program | project context menu |
| Show in Explorer | reveals a path | project and sample context menus |
| Import from an arbitrary path, including drag and drop | a browser drop yields bytes, not a path; `/api/v1/projects/add` needs a path | projects view import |

## Live updates

Two SSE streams from ADR-0024: scan progress at `/api/v1/system/scan-status` drives the
status bar; watcher events at `/api/v1/system/watcher/events` invalidate the projects
table.

## Preferences

Column layout, page size and the details-panel state are read and written as one blob
through the HTTP surface and stored in the `app_state` table (ADR-0031). The endpoint
for that does not exist yet; it is the one backend addition the frontend needs.

## Not covered by the API yet

Found while mapping the sketch onto the routes. Each is small; none blocks the mockups.

- A get/set pair for the UI preferences blob (ADR-0031).
- Tag rename cascade is assumed to be the existing tag update route; verify it
  rewrites `project_tags` rather than creating a new tag.
- Estimated total sample size on disk. Presence is tracked, size may not be.
