# web/

The frontend, in Tauri shells over the daemon's HTTP API (ADR-0048). Right now it holds the
Solid and Svelte 5 comparison that ADR-0047 asks for: the projects screen built twice, from
the approved mockup, against the real API.

```
shared/    framework-free TypeScript both apps import: DTO types, the HTTP and SSE client,
           columns / sorting / paging / selection rules, formatting, popover placement
solid/     Solid + Vite; src/ and src-tauri/
svelte/    Svelte 5 + Vite; src/ and src-tauri/
```

The stylesheets are the mockup's (`../mockup/colors.css`, `shell.css`, `components.css`),
imported unchanged. Because the logic is shared, the two `src/` folders differ only in how
state is held and how the DOM is updated, which is what the comparison is about.

## Running

The apps read from the mock daemon that `mockup/data/generate.py` seeds, on port 50152
(override with `VITE_SEULA_URL`):

```bash
python mockup/data/generate.py seed          # once; builds mockup/data/seula-mock.db
cargo build --bin seula                      # needs protoc (PROTOC=...)
target/debug/seula --config mockup/data/mock-config.toml --server
```

`mock-config.toml` is written by `generate.py`'s `write_config`. Then, in `solid/` or
`svelte/`:

```bash
npm install
npm run dev            # in a browser, http://localhost:1420 (solid) or :1421 (svelte)
npm run tauri dev      # in the Tauri shell
```

Both shells build into one target directory, `web/target/` (`.cargo/config.toml`), and are
not part of the root Cargo workspace, so `cargo build --workspace` never compiles Tauri.

## What the screen covers

Table with sort, paging, click / Ctrl / Shift selection, column chooser, plugin and sample
hover lists, inline rename; inspector for one or several projects with editable notes;
search through `/api/v1/search`; active and archived scope; the sharp/flat key switch; the
row context menu, whose Show in Explorer is the one Tauri IPC call.

**Push-speed state** is a project scan's progress in the status bar. The daemon's real scan
finishes at once on the mock library (it has no project folders), so **Simulate** streams
600 events at about 40 a second in the real stream's shape. **Scan** runs the real one.

Not built: tag, collection and archive edits (the batch buttons are the mockup's, disabled),
audition playback, drag to reorder columns, the other four views, and ADR-0032's custom
window controls (the shell uses the native frame). The CSP is `null` and the icon font loads
from Google Fonts, both fine for a trial and neither for shipping.

ADR-0047 named the plugins view as the comparison screen, because its plugin scan streams.
Projects was chosen instead, so the plugin scan's SSE stream is not exercised.
