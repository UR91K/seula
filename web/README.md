# web/

The frontend: a Solid app in a Tauri shell over the daemon's HTTP API (ADR-0047, ADR-0048).
Right now it holds three views, projects, plugins and samples, built from the approved mockups
against the real API.
It was first built twice, in Solid and in Svelte 5, for the comparison ADR-0047 asked for;
Solid won and the Svelte copy was removed (see `docs/status.md`).

```
shared/    framework-free TypeScript: DTO types, the HTTP and SSE client, the projects and
           plugins and samples rules (columns, sorting, filtering, grouping), formatting,
           popover placement
solid/     Solid + Vite; src-tauri/ is the Tauri shell
  src/shell/     the frame, the route signal, the scan runner (ADR-0052, ADR-0053)
  src/projects/  the projects view
  src/plugins/   the plugins view
  src/samples/   the samples view
```

The stylesheets are the mockup's (`../mockup/colors.css`, `shell.css`, `components.css`),
imported unchanged. The logic lives in `shared/`, framework-free, so `solid/src/` holds only
how state is held and how the DOM is updated.

## Running

The apps read from the mock daemon that `mockup/data/generate.py` seeds, on port 50152
(override with `VITE_SEULA_URL`):

```bash
python mockup/data/generate.py seed          # once; builds mockup/data/seula-mock.db
cargo build --bin seula                      # needs protoc (PROTOC=...)
target/debug/seula --config mockup/data/mock-config.toml --server
```

`mock-config.toml` is written by `generate.py`'s `write_config`. Then, in `solid/`:

```bash
npm install
npm run dev            # in a browser, http://localhost:1420
npm run tauri dev      # in the Tauri shell
```

The shell builds into `web/target/` (`.cargo/config.toml`) and is not part of the root Cargo
workspace, so `cargo build --workspace` never compiles Tauri.

## What the views cover

**Projects.** Table with sort, paging, click / Ctrl / Shift selection, column chooser, plugin and sample
hover lists, inline rename; inspector for one or several projects with editable notes;
search through `/api/v1/search`; active and archived scope; the sharp/flat key switch; the
row context menu, whose Show in Explorer is the one Tauri IPC call.

**Plugins.** Table with sort and the tri-state status (installed, missing, not scanned);
Format, Vendor and Status filters, with a vendor picker that filters as you type; grouping
by vendor or format with foldable headers; server-side search; an inspector with the scan's
record, the projects using the plugin and its identity; a context menu (Show in Explorer
for an installed plugin, show the projects using it, copy name). "Show the projects using
it" switches to the projects view with a `plugin:"name"` search. The status bar's counts
are computed from the rows the filters list, not requested from `/plugins/stats` on every
change; the route counts the same rows.

**Samples.** Table with sort, a Format filter (AIFF covers `.aif`, `.aiff` and `.aifc`) and
a Status filter (present or missing); server-side search; an inspector with the file as the
last check measured it and the projects using the sample; a context menu (Show in Explorer,
show the projects using it, Copy path). A missing sample keeps the size it last had, dimmed.
"Show the projects using it" searches `sample:"name"`. The status bar's size says it is not
measured until a check has run. "Check samples" runs `POST /api/v1/samples/check`. There is
no Play item: it needs a Tauri command and an ADR first. The `scope` preference is not
exposed, so used-in lists stay on the daemon's default (active projects).

**Collections.** A grid of cover cards or a details table (the layout is not persisted,
ADR-0059); the server sorts, so a Sort change or a header click reloads the list; server-side
search; an inspector with the facts, the first ten tracks and every task naming its project
(read-only: the project inspector has no task toggling to share). New, edit details
(name, description, cover), duplicate and delete are dialogs on one shared component
(ADR-0056); a cover is chosen or dropped in the edit dialog and sent as bytes (ADR-0058).
Double-click or Enter opens a collection into its tracklist, which is the projects table
over a state factory (ADR-0055) with a place column and pointer-event drag handles
(ADR-0057); a sort there reorders the view only, and `#` returns to collection order.
Selected tracks can be removed from the collection. "Show in Projects" searches
`collection:"name"`. A cover the daemon cannot serve falls back to the album glyph. Not
built: adding projects to a collection from Projects (the Collection button there is
disabled), "new from selection", and the `scope=all` preference.

**Push-speed state** is a scan's progress in the status bar, for the project scan, the
plugin scan and the sample check alike. The daemon's real scan
finishes at once on the mock library (it has no project folders), so **Simulate** streams
600 events at about 40 a second in the real stream's shape. **Scan** runs the real one.

Not built: tag, collection and archive edits (the batch buttons are the mockup's, disabled),
audition playback, drag to reorder columns, and the stats view. The CSP is `null` and the icon font loads
from Google Fonts, both fine for a trial and neither for shipping.

The plugin scan needs the `vst-meta` worker next to the daemon: `cargo build -p vst-meta`.
Without it the scan fails with a clear message in the status bar.
