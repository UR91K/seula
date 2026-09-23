// The samples view (ADR-0036): its renderers and its live screen. The same shape as the
// plugins view, from the same shell and components.
//
// Every value shown is a snapshotted response: the list, each sample's detail and
// used-in list, the format list (ADR-0039), the status-bar counts, and a real sample
// check's event stream (ADR-0041) (mockup/data/generate.py).

const SAMPLE_API = {
  formats: API["/api/v1/samples/formats"].formats,
  checkEvents: API["POST /api/v1/samples/check"],
};
const sampleById = byId(data.samples);
/** GET /api/v1/samples/:id -- the row plus `file`, what the last check found */
const sampleDetail = (id) => API[`/api/v1/samples/${id}`];
/** GET /api/v1/samples/:id/projects */
const sampleProjects = (id) => (apiGet(`/api/v1/samples/${id}/projects?limit=10000`) || { projects: [] }).projects;

/** The format a path is in, by the extensions /api/v1/samples/formats lists for each. */
function sampleFormat(path) {
  const ext = path.split(".").pop().toLowerCase();
  return SAMPLE_API.formats.find((f) => f.extensions.includes(ext)) || { format: "other", name: "Other", extensions: [] };
}

const folderOf = (path) => path.slice(0, path.lastIndexOf("\\"));

function samplesState(overrides = {}) {
  return {
    view: "samples", tauri: true, inspectorOpen: true, sidebarCollapsed: false,
    query: "", format: null,
    presence: null,         // null | "present" | "missing"
    sort: { col: "projects", desc: true }, page: 0, pageSize: 100,
    selected: null,         // one sample id; the view has no batch actions
    menu: null,             // { id, x?, y? }
    popover: null,          // "format" | "presence"
    checkStep: null,        // index into the check's event stream while one runs
    checkDone: null,        // the final event, shown until the next check
    empty: false,           // a library with no samples yet
    ...overrides,
  };
}

// ------------------------------------------------------------------ rows

/** A folder, truncated in the middle like the inspector's path chip, without the chip. */
function folderCell(path) {
  const folder = folderOf(path);
  const cut = folder.lastIndexOf("\\");
  return `<span class="pathcell" title="${esc(folder)}"><span class="head">${esc(folder.slice(0, cut + 1))}</span><span class="tail">${esc(folder.slice(cut + 1))}</span></span>`;
}

/** Present or missing: icon, colour and word. */
function sampleStateCell(present) {
  return `<span class="pstate ${present ? "is-installed" : "is-absent"}">${sampleStatus(present)}${present ? "Present" : "Missing"}</span>`;
}

/** A missing sample shows the size it last had, dimmed; one no check has found shows none. */
function sizeCell(s) {
  if (s.size_bytes == null) return "";
  return s.is_present ? fmtBytes(s.size_bytes) : `<span class="faint" title="Its size when a check last found it">${fmtBytes(s.size_bytes)}</span>`;
}

const SAMPLE_COLUMNS = [
  { id: "name", label: "Name", w: 120, sort: (s) => s.name.toLowerCase(), cell: (s) => `<span class="n">${esc(s.name)}</span>` },
  { id: "status", label: "Status", w: 40, sort: (s) => (s.is_present ? 0 : 1), cell: (s) => sampleStateCell(s.is_present) },
  { id: "folder", label: "Folder", w: 176, sort: (s) => folderOf(s.path).toLowerCase(), cell: (s) => folderCell(s.path) },
  { id: "format", label: "Format", w: 30, cls: "dim", sort: (s) => sampleFormat(s.path).name, cell: (s) => esc(sampleFormat(s.path).name) },
  { id: "size", label: "Size", w: 32, cls: "num", sort: (s) => s.size_bytes ?? -1, cell: sizeCell },
  { id: "projects", label: "Projects", w: 32, cls: "num", sort: (s) => s.project_count, cell: (s) => s.project_count || "" },
];

/** The samples the list shows: search or the whole list, then the toolbar filters, then
 *  the sort. The server applies the same filters to the list and search routes. */
function sampleRows(st) {
  if (st.empty) return [];
  let rows = st.query
    ? (API[`/api/v1/samples/search?query=${encodeURIComponent(st.query)}&limit=10000`] || { samples: [] }).samples
    : data.samples;
  rows = rows.filter((s) => (!st.format || sampleFormat(s.path).format === st.format)
    && (!st.presence || s.is_present === (st.presence === "present")));
  const col = SAMPLE_COLUMNS.find((c) => c.id === (st.sort && st.sort.col)) || SAMPLE_COLUMNS[0];
  const dir = st.sort && st.sort.desc ? -1 : 1;
  const cmp = (x, y) => (x < y ? -1 : x > y ? 1 : 0);
  return [...rows].sort((a, b) => cmp(col.sort(a), col.sort(b)) * dir || cmp(a.name.toLowerCase(), b.name.toLowerCase()));
}

/** The status bar's counts: GET /api/v1/samples/stats with the list's filters. */
function sampleStats(st) {
  const q = [];
  if (st.query) q.push(`query=${encodeURIComponent(st.query)}`);
  if (st.format) q.push(`format_filter=${st.format}`);
  if (st.presence) q.push(st.presence === "present" ? "present_only=true" : "missing_only=true");
  const hit = API[`/api/v1/samples/stats${q.length ? `?${q.join("&")}` : ""}`];
  if (hit) return hit;
  // The snapshot holds each single filter and the frames' combinations. Any other
  // combination is counted from the rows the same filters list, as the route counts.
  const rows = sampleRows(st);
  const present = rows.filter((s) => s.is_present);
  return {
    total_samples: rows.length, present_samples: present.length, missing_samples: rows.length - present.length,
    sized_samples: present.filter((s) => s.size_bytes != null).length,
    total_size_bytes: present.reduce((n, s) => n + (s.size_bytes || 0), 0),
  };
}

// ------------------------------------------------------------------ table

function sampleTable(st, rows) {
  const page = pageOf(st, rows);
  const sortIcon = (id) => st.sort && st.sort.col === id ? icon(st.sort.desc ? "arrow_downward" : "arrow_upward") : "";
  const head = SAMPLE_COLUMNS.map((c) =>
    `<th class="${(c.cls || "").includes("num") ? "num" : ""} ${st.sort && st.sort.col === c.id ? "sorted" : ""}" data-sort="${c.id}" style="width:calc(var(--u) * ${c.w})">${c.label}${sortIcon(c.id)}<span class="grip"></span></th>`).join("");
  const body = page.map((s) => `<tr data-id="${s.id}" class="${st.selected === s.id ? "sel" : ""}">${
    SAMPLE_COLUMNS.map((c) => `<td class="${c.cls || ""}">${c.cell(s)}</td>`).join("")}</tr>`).join("");
  return `<table class="grid fixed samples"><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table>`;
}

// ------------------------------------------------------------------ toolbar

const PRESENCE = { present: "Present", missing: "Missing" };

function samplesViewbar(st, total) {
  if (st.empty) return `<h1>Samples</h1>${checkButton(st)}`;
  const format = st.format && SAMPLE_API.formats.find((f) => f.format === st.format);
  const search = st.query
    ? `<span class="chip">${icon("search")}“${esc(st.query)}” · ${total} ${total === 1 ? "result" : "results"}<span data-act="clearsearch">${icon("close")}</span></span>`
    : "";
  return `<h1>Samples</h1>${search}
    ${filterSelect("format", "Format", format && format.name, st.popover === "format")}
    ${filterSelect("presence", "Status", st.presence && PRESENCE[st.presence], st.popover === "presence")}
    <span class="sep"></span>
    ${checkButton(st)}
    <span class="grow"></span>
    ${pager(st, total)}`;
}

/** Disabled while any scan runs (ADR-0038, ADR-0041). */
function checkButton(st) {
  const running = st.checkStep != null;
  return tbBtn(running ? "hourglass_top" : "fact_check", running ? "Checking…" : "Check samples",
    { title: running ? "A scan is running" : "Check each sample file is still there, and measure it", disabled: running, act: "check" });
}

// ------------------------------------------------------------------ popovers

/** Format ▾: every format Live loads, with counts; AIFF covers .aif, .aiff and .aifc. */
function sampleFormatMenu(current) {
  const it = (f) => `<div class="it ${f.format === current ? "hot" : ""} ${f.count ? "" : "off"}" data-act="set-format" data-v="${f.format}" title="${f.extensions.map((e) => `.${e}`).join(" ")}">${
    icon("check", f.format === current ? "" : "blank")}<span class="grow">${esc(f.name)} <span class="faint">${f.extensions.map((e) => `.${e}`).join(" ")}</span></span><span class="n">${f.count}</span></div>`;
  const all = `<div class="it ${!current ? "hot" : ""}" data-act="set-format" data-v="">${icon("check", current ? "blank" : "")}<span class="grow">All formats</span><span class="n">${data.samples.length}</span></div>`;
  return `<div class="pop picker pickmenu"><div class="list" style="max-height:none;padding-top:var(--u)">${all}${SAMPLE_API.formats.map(it).join("")}</div></div>`;
}

/** Status ▾: present or missing, with the library's counts. */
function presenceMenu(current) {
  const all = API["/api/v1/samples/stats"];
  const it = (v, label, n, present) => `<div class="it ${v === current ? "hot" : ""}" data-act="set-presence" data-v="${v || ""}">${
    icon("check", v === current ? "" : "blank")}${present == null ? "" : sampleStatus(present)}<span class="grow">${label}</span><span class="n">${n}</span></div>`;
  return `<div class="pop picker pickmenu" style="width:calc(var(--u) * 80)"><div class="list" style="padding-top:var(--u)">${
    it(null, "All samples", all.total_samples, null)}${it("present", "Present", all.present_samples, true)}${it("missing", "Missing", all.missing_samples, false)}</div></div>`;
}

/** The row menu. Play and Show in Explorer need the file, and a native shell (ADR-0030). */
function sampleContextMenu(s, { tauri = true, hot = null } = {}) {
  return menu([
    { ic: "play_arrow", label: "Play", native: true, off: !s.is_present, hot: hot === "play" },
    { ic: "folder_open", label: "Show in Explorer", native: true, off: !s.is_present, hot: hot === "explorer" },
    { sep: true },
    { ic: "audio_file", label: `Show the ${s.project_count} ${s.project_count === 1 ? "project" : "projects"} using it`, act: "showprojects", off: !s.project_count, hot: hot === "projects" },
    { ic: "content_copy", label: "Copy path", act: "close" },
  ], { tauri });
}

// ------------------------------------------------------------------ inspector

function sampleStateSection(s, file, tauri) {
  const when = file && fmtDate(file.checked_at);
  if (s.is_present) {
    return `<div class="insp-sec"><div class="state-line is-installed">${sampleStatus(true)}<b>Present</b></div>
      <p class="faint">${file ? `Found by the check on ${when}` : "Found when its project was scanned. No check has measured it yet."}</p>
      ${tauri ? `<button class="btn" title="Plays the file from disk (native-only)">${icon("play_arrow")}Play</button>` : ""}</div>`;
  }
  return `<div class="insp-sec"><div class="state-line is-absent">${sampleStatus(false)}<b>Missing</b></div>
    <p class="faint">${file ? `Last found by a check on ${when}` : "No check has ever found it. Its projects refer to it by path only."}</p></div>`;
}

function sampleInspector(s, { tauri = true } = {}) {
  const { file } = sampleDetail(s.id);
  const projects = sampleProjects(s.id);
  const fmt = sampleFormat(s.path);
  const ext = s.path.split(".").pop().toLowerCase();
  const used = projects.length
    ? inspectorList([...projects].sort((a, b) => b.modified_at - a.modified_at), (x) => `<li>${icon("audio_file")}<span class="grow">${esc(x.name)}</span><span class="faint">${fmtDate(x.modified_at).slice(0, 10)}</span></li>`, 10)
      + (projects.length > 10 ? `<button class="linkbtn" data-act="showprojects">Show all ${projects.length} in Projects ${icon("arrow_forward")}</button>` : "")
    : '<span class="faint">No project uses it</span>';
  return `
    <div class="insp-head">
      <span class="glyph">${icon("graphic_eq")}</span>
      <div><h2>${esc(s.name)}</h2><div class="faint">${esc(fmt.name)}${file ? ` · ${fmtBytes(file.size_bytes)}` : ""}</div>${pathChip(s.path)}</div>
    </div>
    ${sampleStateSection(s, file, tauri)}
    <div class="insp-sec"><h3>Used in <span class="count">${projects.length || ""}</span></h3>${used}</div>
    <div class="insp-sec"><h3>File${s.is_present ? "" : ' <span class="count">as last found</span>'}</h3>${props([
      ["Format", `${esc(fmt.name)} <span class="faint">.${esc(ext)}</span>`],
      ["Size", file && `${fmtBytes(file.size_bytes)} <span class="faint">${file.size_bytes.toLocaleString("en-US")} bytes</span>`],
      ["Modified", file && file.modified_at && fmtDate(file.modified_at)],
      ["Folder", `<span title="${esc(folderOf(s.path))}">${esc(folderOf(s.path).split("\\").pop())}</span>`],
    ])}${file ? "" : '<p class="faint">No check has measured this file.</p>'}</div>`;
}

const sampleInspectorEmpty = () => `<div class="empty">${icon("right_panel_open")}<p>Select a sample to see its file and the projects that use it.</p></div>`;

// ------------------------------------------------------------------ empty states

function samplesEmpty(st) {
  if (st.empty) {
    return `<div class="empty">${icon("earthquake")}<h2>No samples yet</h2>
      <p>Samples are listed as the projects that use them are scanned.</p></div>`;
  }
  const what = [st.query && `“${esc(st.query)}”`, st.format && SAMPLE_API.formats.find((f) => f.format === st.format).name,
    st.presence && PRESENCE[st.presence].toLowerCase()].filter(Boolean);
  return `<div class="empty">${icon("search_off")}<h2>No samples match</h2><p>${what.join(" · ")}</p>
    <button class="btn" data-act="clearfilters">Clear filters</button></div>`;
}

// ------------------------------------------------------------------ status bar

function sampleStatusSegments(st) {
  const s = sampleStats(st);
  const f = !!(st.query || st.format || st.presence);
  // A size over fewer samples than are present says how many it covers.
  const size = s.sized_samples < s.present_samples
    ? `${fmtBytes(s.total_size_bytes)} <span class="faint">(${s.sized_samples} of ${s.present_samples} measured)</span>`
    : fmtBytes(s.total_size_bytes);
  return [
    f ? `${s.total_samples} of ${data.samples.length} samples` : `${s.total_samples} samples`,
    `${sampleStatus(true)} ${s.present_samples} present`,
    `${sampleStatus(false)} ${s.missing_samples} missing`,
    `${icon("hard_drive", "sb")} ${size}`,
    ...(st.checkDone ? [`<span class="${st.checkDone.status === "completed" ? "is-installed" : "is-absent"}">${icon(st.checkDone.status === "completed" ? "task_alt" : "error")}</span> ${esc(st.checkDone.message)}`] : []),
  ];
}

// ------------------------------------------------------------------ the live screen

function samplesScreen(st) {
  let timer = null;

  function draw(el) {
    const rows = sampleRows(st);
    const sel = st.selected && sampleById.get(st.selected);
    const box = el.querySelector(".content");
    const scroll = box ? [box.scrollTop, box.scrollLeft] : null;
    const ev = st.checkStep != null ? SAMPLE_API.checkEvents[st.checkStep] : null;

    el.innerHTML = renderShell({
      ...st,
      viewbar: samplesViewbar(st, rows.length),
      content: rows.length ? sampleTable(st, rows) : samplesEmpty(st),
      inspector: sel ? sampleInspector(sel, { tauri: st.tauri }) : sampleInspectorEmpty(),
      status: st.empty ? ["No samples"] : sampleStatusSegments(st),
      scan: ev ? { done: ev.completed, total: ev.total, message: ev.message } : null,
      counts: st.empty ? { projects: 0, collections: 0, plugins: 0, samples: 0 } : null,
    });

    const win = el.querySelector(".win");
    const content = el.querySelector(".content");
    if (scroll) [content.scrollTop, content.scrollLeft] = scroll;
    else if (st.selected) {
      const first = el.querySelector("tr.sel");
      if (first) content.scrollTop = first.offsetTop - content.clientHeight / 3;
    }

    if (st.menu) {
      const row = el.querySelector(`tr[data-id="${st.menu.id}"]`);
      let { x, y } = st.menu;
      if (x == null && row) {
        const r = row.querySelector(".n").getBoundingClientRect(), w = win.getBoundingClientRect(), z = w.width / win.offsetWidth;
        x = (r.left - w.left) / z + 40; y = (r.top - w.top) / z + 9;
      }
      place(win, sampleContextMenu(sampleById.get(st.menu.id), { tauri: st.tauri }), { x, y });
    }
    const anchor = (act) => el.querySelector(`.viewbar [data-act="${act}"]`);
    if (st.popover === "format") place(win, sampleFormatMenu(st.format), { anchor: anchor("format") });
    if (st.popover === "presence") place(win, presenceMenu(st.presence), { anchor: anchor("presence") });
  }

  /** Replays the real check's event stream, as the SSE would deliver it. */
  function startCheck(el) {
    st.checkStep = 0; st.checkDone = null;
    clearInterval(timer);
    timer = setInterval(() => {
      st.checkStep += 1;
      const ev = SAMPLE_API.checkEvents[st.checkStep];
      if (!ev || ev.status !== "checking_samples") {
        clearInterval(timer);
        st.checkStep = null; st.checkDone = ev || null;
      }
      draw(el);
    }, 250);
  }

  function wire(el) {
    const redraw = () => draw(el);
    const closeAll = () => { st.menu = null; st.popover = null; };

    el.addEventListener("contextmenu", (e) => {
      const row = e.target.closest("tr[data-id]");
      if (!row) return;
      e.preventDefault();
      st.selected = row.dataset.id;
      const win = el.querySelector(".win"), w = win.getBoundingClientRect(), z = w.width / win.offsetWidth;
      closeAll();
      st.menu = { id: row.dataset.id, x: (e.clientX - w.left) / z, y: (e.clientY - w.top) / z };
      redraw();
    });

    el.addEventListener("click", (e) => {
      const hit = e.target.closest("[data-act]");
      const act = hit?.dataset.act;
      const th = e.target.closest("th[data-sort]");
      const row = e.target.closest("tr[data-id]");
      const inPop = e.target.closest(".pop");
      const v = hit?.dataset.v || null;

      if (act === "clear-format") { st.format = null; st.page = 0; closeAll(); return redraw(); }
      if (act === "clear-presence") { st.presence = null; st.page = 0; closeAll(); return redraw(); }
      if (act === "format" || act === "presence") { st.popover = st.popover === act ? null : act; st.menu = null; return redraw(); }
      if (act === "set-format") { st.format = v; st.page = 0; closeAll(); return redraw(); }
      if (act === "set-presence") { st.presence = v; st.page = 0; closeAll(); return redraw(); }
      if (act === "check") { closeAll(); startCheck(el); return redraw(); }
      if (act === "clearsearch") { st.query = ""; st.page = 0; return redraw(); }
      if (act === "clearfilters") { st.query = ""; st.format = null; st.presence = null; st.page = 0; return redraw(); }
      if (act === "sidebar") { st.sidebarCollapsed = !st.sidebarCollapsed; return redraw(); }
      if (act === "inspector") { st.inspectorOpen = !st.inspectorOpen; return redraw(); }
      if (act === "prev" || act === "next") { st.page += act === "next" ? 1 : -1; el.querySelector(".content").scrollTop = 0; return redraw(); }
      if (act === "close" || act === "showprojects") { closeAll(); return redraw(); }
      if (inPop) return;

      if (th) {
        const col = th.dataset.sort;
        st.sort = st.sort && st.sort.col === col ? { col, desc: !st.sort.desc } : { col, desc: col === "projects" || col === "size" };
        return redraw();
      }
      if (row) { st.selected = row.dataset.id; closeAll(); return redraw(); }
      if (st.menu || st.popover) { closeAll(); redraw(); }
    });
  }

  return (el) => {
    if (!el.dataset.wired) { el.dataset.wired = "1"; wire(el); }
    draw(el);
  };
}
