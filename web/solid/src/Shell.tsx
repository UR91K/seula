// The frame (ADR-0032): top bar, sidebar, status bar. The status bar is where push-speed
// state lands, and it is the only reader of `scan`.

import { For, Show } from "solid-js";
import { LOGO_PATH, LOGO_TRANSFORM, LOGO_VIEWBOX } from "../../shared/logo";
import { plural } from "../../shared/projects";
import { Icon, TbBtn } from "./parts";
import {
  activeTotal, loadError, notice, scan, selectedCount, setQuery, setUi, system, total, ui,
} from "./state";

const VIEWS = [
  { id: "projects", label: "Projects", icon: "audio_file" },
  { id: "collections", label: "Collections", icon: "album" },
  { id: "plugins", label: "Plugins", icon: "plug_connect" },
  { id: "samples", label: "Samples", icon: "earthquake" },
  { id: "stats", label: "Stats", icon: "bar_chart" },
];

export function Topbar() {
  let timer: ReturnType<typeof setTimeout> | undefined;
  // Search waits for typing to pause, then asks the server (FTS5, ADR-0024).
  const onInput = (value: string) => { clearTimeout(timer); timer = setTimeout(() => setQuery(value.trim()), 250); };
  return (
    <header class="topbar">
      <svg class="logo" viewBox={LOGO_VIEWBOX} role="img" aria-label="Seula">
        <path fill="currentColor" transform={LOGO_TRANSFORM} d={LOGO_PATH} />
      </svg>
      <nav class="menubar"><span>File</span><span>Edit</span><span>View</span><span>Tools</span><span>Help</span></nav>
      <label class="search">
        <Icon name="search" />
        <input placeholder="Search projects, plugins, samples, tags" value={ui.query}
          onInput={(e) => onInput(e.currentTarget.value)} />
      </label>
      <TbBtn icon="settings" title="Settings" />
      <span style={{ width: "calc(var(--u) * 2)" }} />
    </header>
  );
}

export function Sidebar() {
  return (
    <aside class="sidebar">
      <For each={VIEWS}>{(v) => (
        <div class="nav-item" classList={{ active: v.id === "projects" }} title={v.label}>
          <Icon name={v.icon} fill={v.id === "projects"} />
          <span class="label">{v.label}</span>
          <span class="count">{v.id === "projects" ? (activeTotal() ?? "") : ""}</span>
        </div>
      )}</For>
      <div class="spacer" />
      <button class="tb-btn collapse" title={`${ui.sidebarCollapsed ? "Expand" : "Collapse"} sidebar`}
        onClick={() => setUi("sidebarCollapsed", (c) => !c)}>
        <Icon name={ui.sidebarCollapsed ? "left_panel_open" : "left_panel_close"} />
      </button>
    </aside>
  );
}

/** The scan segment. Its own component so a progress tick re-runs this and nothing else. */
function ScanSegment() {
  return (
    <span class="grow">
      <Show when={scan()}>{(s) => (
        <>
          <span class="progress"><i style={{ width: `${(100 * (s().total ? s().completed / s().total : 0)).toFixed(1)}%` }} /></span>
          {s().message}
        </>
      )}</Show>
      <Show when={!scan() && (notice() || loadError())}>
        <span style={{ color: "var(--danger)" }}>{notice() ?? loadError()}</span>
      </Show>
    </span>
  );
}

export function Statusbar() {
  const count = () => ui.query
    ? plural(total(), "result")
    : `${total()} ${ui.scope === "archived" ? "archived" : "projects"}`;
  const watcher = () => {
    const s = system();
    if (!s) return "";
    const n = s.watch_paths.length;
    return s.watcher_active ? `Watching ${plural(n, "folder")}` : "Watcher off";
  };
  return (
    <footer class="statusbar">
      <span>{count()}</span>
      <Show when={selectedCount()}><span>{selectedCount()} selected</span></Show>
      <ScanSegment />
      <span>{watcher()}</span>
      <span class="ks">
        <span class="keyswitch">
          <button classList={{ on: ui.spelling === "sharp" }} title="Show keys with sharps" onClick={() => setUi("spelling", "sharp")}>♯</button>
          <button classList={{ on: ui.spelling === "flat" }} title="Show keys with flats" onClick={() => setUi("spelling", "flat")}>♭</button>
        </span>
      </span>
    </footer>
  );
}
