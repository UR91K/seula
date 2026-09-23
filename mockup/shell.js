// The shell, rendered from one function so every frame and every view page gets the
// same frame. Views supply only `viewbar`, `content` and `inspector` HTML; they never
// build the top bar, sidebar or status bar themselves.
//
// Data comes from data/mock-api.js (window.SEULA_API), a snapshot of the real HTTP API
// over a seeded database. Keys are request paths.

const API = window.SEULA_API;
const BIG = "limit=10000";

const data = {
  projects: API[`/api/v1/projects?${BIG}`].projects,
  archived: API[`/api/v1/projects?scope=deleted&${BIG}`].projects,
  collections: API[`/api/v1/collections?${BIG}`].collections,
  plugins: API[`/api/v1/plugins?${BIG}`].plugins,
  samples: API[`/api/v1/samples?${BIG}`].samples,
  media: API[`/api/v1/media?${BIG}`].media_files,
  tags: API[`/api/v1/tags/with-usage?${BIG}`].tags,
  system: API["/api/v1/system/info"],
  config: API["/api/v1/config/status"],
};

/** One snapshotted response. A plugin's "used in" list is stored as project ids to
 *  keep the file small (the generator checks each equals the main list's copy), and is
 *  put back here, so callers get the response as the server sent it. */
function apiGet(path) {
  const body = API[path];
  if (!body || !body.project_ids) return body;
  const byId = apiGet.byId || (apiGet.byId = new Map(data.projects.map((p) => [p.id, p])));
  const { project_ids, ...rest } = body;
  return { ...rest, projects: project_ids.map((id) => byId.get(id)) };
}

// ------------------------------------------------------------------ formatting

const pad2 = (n) => String(n).padStart(2, "0");

/** 2026/09/21 02:50 -- the date style of the reference screenshots. */
function fmtDate(epoch) {
  const d = new Date(epoch * 1000);
  return `${d.getFullYear()}/${pad2(d.getMonth() + 1)}/${pad2(d.getDate())} ${pad2(d.getHours())}:${pad2(d.getMinutes())}`;
}

function fmtLength(seconds) {
  if (seconds == null) return "";
  const s = Math.round(seconds);
  return `${Math.floor(s / 60)}:${pad2(s % 60)}`;
}

/** The API sends both spellings (ADR-0035); the sharp/flat switch picks one. It is
 *  app-wide, so flipping it redraws every frame on the board. */
let keySpelling = "sharp";
function fmtKey(key) {
  return key ? key[keySpelling] : "";
}

document.addEventListener("click", (e) => {
  const b = e.target.closest("[data-keyspell]");
  if (!b || b.dataset.keyspell === keySpelling) return;
  keySpelling = b.dataset.keyspell;
  if (typeof refreshBoard === "function") refreshBoard();
});

/** The ♯/♭ switch that sits at the right end of the status bar. */
function keySwitch() {
  const b = (v, label, title) =>
    `<button class="${keySpelling === v ? "on" : ""}" data-keyspell="${v}" title="${title}">${label}</button>`;
  return `<span class="keyswitch">${b("sharp", "♯", "Show keys with sharps")}${b("flat", "♭", "Show keys with flats")}</span>`;
}

function fmtTempo(t) {
  return Number.isInteger(t) ? `${t}` : t.toFixed(2);
}

function fmtVersion(v) {
  return `${v.major}.${v.minor}.${v.patch}${v.beta ? " beta" : ""}`;
}

function fmtBytes(n) {
  if (n >= 1e9) return `${(n / 1e9).toFixed(1)} GB`;
  if (n >= 1e6) return `${(n / 1e6).toFixed(1)} MB`;
  return `${Math.round(n / 1e3)} KB`;
}

const esc = (s) => String(s ?? "").replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[c]);

const icon = (name, cls = "") => `<span class="ms ${cls}">${name}</span>`;

/**
 * A file path as a small block truncated in the middle, so the drive and the file name
 * both stay visible. Full path on hover; click copies it.
 */
function pathChip(path) {
  const cut = path.lastIndexOf("\\");
  const head = path.slice(0, cut + 1);
  const tail = path.slice(cut + 1);
  return `<span class="path" title="${esc(path)}" data-copy="${esc(path)}"><span class="head">${esc(head)}</span><span class="tail">${esc(tail)}</span></span>`;
}

// One listener for every path chip on the page, present and future.
document.addEventListener("click", (e) => {
  const chip = e.target.closest("[data-copy]");
  if (!chip) return;
  navigator.clipboard?.writeText(chip.dataset.copy).catch(() => {});
  chip.classList.add("copied");
  setTimeout(() => chip.classList.remove("copied"), 1200);
});

/** File name of an .als path, without extension. */
function fileStem(path) {
  return path.split("\\").pop().replace(/\.als$/i, "");
}

/** Cover art for a collection, resolved through the media list to a thumbnail. */
function coverUrl(collection) {
  if (!collection || !collection.cover_art_id) return null;
  const m = data.media.find((x) => x.id === collection.cover_art_id);
  return m ? `images/thumbs/${m.original_filename}` : null;
}

/** Status of a plugin: installed true/false, or null when no scan has looked. */
function pluginStatus(installed) {
  if (installed === true) return `<span class="status-dot is-ok">${icon("check_circle", "fill")}</span>`;
  if (installed === false) return `<span class="status-dot is-missing">${icon("error", "fill")}</span>`;
  return `<span class="status-dot is-unknown">${icon("help", "")}</span>`;
}

function sampleStatus(present) {
  return present
    ? `<span class="status-dot is-ok">${icon("check_circle", "fill")}</span>`
    : `<span class="status-dot is-missing">${icon("error", "fill")}</span>`;
}

// ------------------------------------------------------------------ shell

/** From /api/v1/system/info. The mock daemon watches nothing, so this reads "off". */
function watcherStatus() {
  const n = data.system.watch_paths.length;
  return data.system.watcher_active ? `Watching ${n} folder${n === 1 ? "" : "s"}` : "Watcher off";
}

const VIEWS = [
  { id: "projects", label: "Projects", icon: "audio_file", count: () => data.projects.length },
  { id: "collections", label: "Collections", icon: "album", count: () => data.collections.length },
  { id: "plugins", label: "Plugins", icon: "plug_connect", count: () => data.plugins.length },
  { id: "samples", label: "Samples", icon: "earthquake", count: () => data.samples.length },
  { id: "stats", label: "Stats", icon: "bar_chart", count: () => "" },
];

// vectors/seula-logo.svg, with its fixed fill swapped for currentColor so the logo
// takes --accent from .logo.
const LOGO = `<svg class="logo" viewBox="0 0 216.54 406" role="img" aria-label="Seula">
  <path fill="currentColor" transform="translate(-141.73 -47)" d="M207.81,453c-21.55,0-43.79-4.79-66.08-14.35l20.5-47.79c67.24,28.84,113.35-9,132.4-50.09C318.08,290.16,307.8,222,242.77,189.54c-31.1-15.51-49.44-40.78-50.3-69.32-.78-25.72,13.39-50.8,36.09-63.89,23-13.25,50.92-12.34,74.73,2.42L275.89,103c-9.59-5.94-16.9-4.14-21.35-1.57-6.24,3.6-10.3,10.54-10.1,17.26.27,8.92,7.92,17.57,21.53,24.36,43.79,21.83,73.55,56.62,86,100.61,11,38.77,7.4,81-10.19,119-16.57,35.75-44,63.58-77.19,78.35A138.78,138.78,0,0,1,207.81,453Z"/>
</svg>`;

/**
 * state: {
 *   view, sidebarCollapsed, inspectorOpen, tauri,
 *   viewbar: html, content: html, inspector: html,
 *   status: [html, ...]   left-hand status segments
 *   scan: null | { done, total, message }   a scan's progress event; the message is
 *                                           the server's own and carries its count
 *   overlay: html          menus, popovers and dialogs, positioned against the window
 *   counts: { view: n }    sidebar counts for a state the snapshot does not hold (a fresh install)
 * }
 */
/** The status bar alone: the view's segments, a running scan, the watcher, ♯/♭. */
function statusBar(s) {
  const scan = s.scan
    ? `<span class="grow"><span class="progress"><i style="width:${(100 * (s.scan.total ? s.scan.done / s.scan.total : 0)).toFixed(1)}%"></i></span>${esc(s.scan.message)}</span>`
    : `<span class="grow"></span>`;
  return `<footer class="statusbar">
      ${(s.status || []).map((x) => `<span>${x}</span>`).join("")}
      ${scan}
      <span>${watcherStatus()}</span>
      <span class="ks">${keySwitch()}</span>
    </footer>`;
}

function renderShell(s) {
  const hasInspector = s.view !== "stats";
  const inspectorShown = hasInspector && s.inspectorOpen;
  const cls = ["win", s.sidebarCollapsed && "sidebar-collapsed", !inspectorShown && "no-inspector"]
    .filter(Boolean).join(" ");

  const nav = VIEWS.map((v) => `
    <div class="nav-item ${v.id === s.view ? "active" : ""}" data-nav="${v.id}" title="${v.label}">
      ${icon(v.icon, v.id === s.view ? "fill" : "")}<span class="label">${v.label}</span><span class="count">${s.counts && v.id in s.counts ? s.counts[v.id] : v.count()}</span>
    </div>`).join("");

  return `
  <div class="${cls}">
    <header class="topbar">
      ${LOGO}
      <nav class="menubar"><span>File</span><span>Edit</span><span>View</span><span>Tools</span><span>Help</span></nav>
      <label class="search">${icon("search")}<input placeholder="Search projects, plugins, samples, tags" value="${esc(s.query || "")}"></label>
      <button class="tb-btn" title="Settings">${icon("settings")}</button>
      ${s.tauri ? `<div class="winctl"><span>${icon("remove")}</span><span>${icon("crop_square")}</span><span class="close">${icon("close")}</span></div>` : `<span style="width:calc(var(--u) * 2)"></span>`}
    </header>

    <div class="cols">
      <aside class="sidebar">
        ${nav}
        <div class="spacer"></div>
        <button class="tb-btn collapse" data-act="sidebar" title="${s.sidebarCollapsed ? "Expand" : "Collapse"} sidebar">
          ${icon(s.sidebarCollapsed ? "left_panel_open" : "left_panel_close")}
        </button>
      </aside>

      <section class="main">
        <div class="viewbar">
          ${s.viewbar}
          <span class="grow"></span>
          ${hasInspector ? `<button class="tb-btn ${inspectorShown ? "on" : ""}" data-act="inspector" title="Inspector">${icon(inspectorShown ? "right_panel_close" : "right_panel_open")}</button>` : ""}
        </div>
        <div class="content">${s.content}</div>
      </section>

      ${inspectorShown ? `<aside class="inspector">${s.inspector}</aside>` : ""}
    </div>

    ${statusBar(s)}
    ${s.overlay || ""}
  </div>`;
}
