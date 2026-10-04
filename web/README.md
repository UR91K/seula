# web/

The frontend: a Solid app in a Tauri shell over the daemon's HTTP API (ADR-0047, ADR-0048).
Right now it holds the projects screen, built from the approved mockup against the real API.
It was first built twice, in Solid and in Svelte 5, for the comparison ADR-0047 asked for;
Solid won and the Svelte copy was removed (see `docs/status.md`).

```
shared/    framework-free TypeScript: DTO types, the HTTP and SSE client,
           columns / sorting / paging / selection rules, formatting, popover placement
solid/     Solid + Vite; src/ and src-tauri/
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
