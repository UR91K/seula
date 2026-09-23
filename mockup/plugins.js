// The plugins view (ADR-0036): its renderers and its live screen. Built from shell.js
// and components.js the same way the projects view is, and reusing their menu, picker,
// popover and inspector pieces rather than copies of them.
//
// Every value shown is a snapshotted response: the list, each plugin's detail and
// used-in list, the vendor and format rollups, the status-bar counts, and a real plugin
// scan's event stream (mockup/data/generate.py).

const PLUGIN_API = {
  vendors: API["/api/v1/plugins/vendors?limit=10000"].vendors,
  formats: API["/api/v1/plugins/formats"].formats,
  scanEvents: API["POST /api/v1/plugins/scan"],
};
const pluginById = byId(data.plugins);
/** GET /api/v1/plugins/:id -- { plugin, details } */
const pluginDetails = (id) => API[`/api/v1/plugins/${id}`].details;
/** GET /api/v1/plugins/:id/projects */
const pluginProjects = (id) => (apiGet(`/api/v1/plugins/${id}/projects?limit=10000`) || { projects: [] }).projects;

/** The tri-state installed flag (ADR-0012) under its install_states names (ADR-0025). */
const INSTALL_STATES = [
  { id: "installed", label: "Installed", value: true },
  { id: "absent", label: "Missing", value: false },
  { id: "unscanned", label: "Not scanned", value: null },
];
const stateOf = (installed) => installed === true ? "installed" : installed === false ? "absent" : "unscanned";
const stateLabel = (installed) => INSTALL_STATES.find((s) => s.id === stateOf(installed)).label;

function pluginsState(overrides = {}) {
  return {
    view: "plugins", tauri: true, inspectorOpen: true, sidebarCollapsed: false,
    query: "", vendor: null, format: null, states: [],
    group: null,            // null | "vendor" | "format"
    collapsed: new Set(),   // group keys folded shut
    sort: { col: "projects", desc: true }, page: 0, pageSize: 100,
    selected: null,         // one plugin id; the view has no batch actions
    menu: null,             // { id, x?, y? }
    popover: null,          // "format" | "vendor" | "states" | "group"
    scanStep: null,         // index into the scan's event stream while one runs
    scanDone: null,         // the final event, shown until the next scan
    empty: false,           // a library no scan has filled yet
    ...overrides,
  };
}

// ------------------------------------------------------------------ rows

/** Sorts after any name, so plugins with no vendor come last. */
const SORTS_LAST = String.fromCharCode(0xffff);
const versionKey = (v) => (v || "").split(".").map((n) => n.padStart(4, "0")).join(".");

const PLUGIN_COLUMNS = [
  { id: "name", label: "Name", w: 124, sort: (p) => p.name.toLowerCase(), cell: (p) => `<span class="n">${esc(p.name)}</span>` },
  { id: "status", label: "Status", w: 50, sort: (p) => INSTALL_STATES.findIndex((s) => s.id === stateOf(p.installed)), cell: (p) => pluginStateCell(p.installed) },
  { id: "vendor", label: "Vendor", w: 90, cls: "dim", sort: (p) => (p.vendor || SORTS_LAST).toLowerCase(), cell: (p) => p.vendor ? esc(p.vendor) : `<span class="faint">No vendor</span>` },
  { id: "format", label: "Format", w: 64, cls: "dim", sort: (p) => p.format, cell: (p) => esc(p.format) },
  { id: "version", label: "Version", w: 36, cls: "dim", sort: (p) => versionKey(p.version), cell: (p) => esc(p.version || "") },
  { id: "projects", label: "Projects", w: 36, cls: "num", sort: (p) => p.project_count, cell: (p) => p.project_count || "" },
];

/** Icon, colour and word: never the colour alone. */
function pluginStateCell(installed) {
  return `<span class="pstate is-${stateOf(installed)}">${pluginStatus(installed)}${stateLabel(installed)}</span>`;
}

const GROUPS = {
  vendor: { label: "Vendor", key: (p) => p.vendor, name: (k) => k || "No vendor" },
  format: { label: "Format", key: (p) => p.format, name: (k) => k },
};

/** The plugins the list shows: search or the whole list, then the toolbar filters, then
 *  the sort, grouped when grouping is on. The server applies the same filters to both
 *  the list and the search routes. */
function pluginRows(st) {
  if (st.empty) return [];
  let rows = st.query
    ? (API[`/api/v1/plugins/search?query=${encodeURIComponent(st.query)}&limit=10000`] || { plugins: [] }).plugins
    : data.plugins;
  rows = rows.filter((p) => (!st.vendor || p.vendor === st.vendor) && (!st.format || p.format === st.format)
    && (!st.states.length || st.states.includes(stateOf(p.installed))));
  const col = PLUGIN_COLUMNS.find((c) => c.id === (st.sort && st.sort.col)) || PLUGIN_COLUMNS[0];
  const dir = st.sort && st.sort.desc ? -1 : 1;
  const cmp = (x, y) => (x < y ? -1 : x > y ? 1 : 0);
  rows = [...rows].sort((a, b) => cmp(col.sort(a), col.sort(b)) * dir || cmp(a.name.toLowerCase(), b.name.toLowerCase()));
  if (st.group) {
    // Groups in name order, the no-vendor group last; the sort applies inside each.
    const g = GROUPS[st.group];
    const gk = (p) => g.key(p) == null ? SORTS_LAST : g.key(p).toLowerCase();
    rows = [...rows].sort((a, b) => cmp(gk(a), gk(b)));
  }
  return rows;
}

/** The status bar's counts: GET /api/v1/plugins/stats with the list's filters. */
function pluginStats(st) {
  const q = [];
  if (st.query) q.push(`query=${encodeURIComponent(st.query)}`);
  if (st.vendor) q.push(`vendor_filter=${encodeURIComponent(st.vendor)}`);
  if (st.format) q.push(`format_filter=${encodeURIComponent(st.format)}`);
  if (st.states.length) q.push(`install_states=${INSTALL_STATES.map((s) => s.id).filter((id) => st.states.includes(id)).join(",")}`);
  const hit = API[`/api/v1/plugins/stats${q.length ? `?${q.join("&")}` : ""}`];
  if (hit) return hit;
  // The snapshot holds each single filter and the frames' combinations. Any other
  // combination is counted from the rows the same filters list, which is what the
  // route counts.
  const rows = pluginRows({ ...st, group: null });
  const n = (v) => rows.filter((p) => p.installed === v).length;
  return {
    total_plugins: rows.length, installed_plugins: n(true), missing_plugins: n(false), unknown_plugins: n(null),
    unique_vendors: new Set(rows.map((p) => p.vendor).filter(Boolean)).size,
  };
}

const filtered = (st) => !!(st.query || st.vendor || st.format || st.states.length);

// ------------------------------------------------------------------ table

/** A group's header row: its name, and its totals from the vendor or format rollup. */
function groupHeader(st, key, shown, span) {
  const g = GROUPS[st.group];
  const roll = st.group === "vendor" ? PLUGIN_API.vendors.find((v) => v.vendor === key) : PLUGIN_API.formats.find((f) => f.format === key);
  const open = !st.collapsed.has(String(key));
  const bits = roll
    ? [`${roll.plugin_count} ${roll.plugin_count === 1 ? "plugin" : "plugins"}`,
       roll.installed_plugins && `${roll.installed_plugins} installed`,
       roll.missing_plugins && `<span class="is-absent">${roll.missing_plugins} missing</span>`,
       roll.unknown_plugins && `${roll.unknown_plugins} not scanned`,
       `used in ${roll.unique_projects_using} ${roll.unique_projects_using === 1 ? "project" : "projects"}`]
    // The vendor rollup leaves out plugins with no vendor (it groups WHERE vendor IS NOT NULL).
    : [`${shown} ${shown === 1 ? "plugin" : "plugins"}`];
  const of = roll && shown !== roll.plugin_count ? `<span class="shown">${shown} shown</span>` : "";
  return `<tr class="group" data-group="${esc(String(key))}"><td colspan="${span}">${icon(open ? "expand_more" : "chevron_right", "caret")}<span class="gname">${esc(g.name(key))}</span>${of}<span class="gstats">${bits.filter(Boolean).join(" · ")}</span></td></tr>`;
}

function pluginTable(st, rows) {
  const page = pageOf(st, rows);
  const sortIcon = (id) => st.sort && st.sort.col === id ? icon(st.sort.desc ? "arrow_downward" : "arrow_upward") : "";
  const head = PLUGIN_COLUMNS.map((c) =>
    `<th class="${(c.cls || "").includes("num") ? "num" : ""} ${st.sort && st.sort.col === c.id ? "sorted" : ""}" data-sort="${c.id}" style="width:calc(var(--u) * ${c.w})">${c.label}${sortIcon(c.id)}<span class="grip"></span></th>`).join("");
  const row = (p) => `<tr data-id="${p.id}" class="${st.selected === p.id ? "sel" : ""}">${
    PLUGIN_COLUMNS.map((c) => `<td class="${c.cls || ""}">${c.cell(p)}</td>`).join("")}</tr>`;

  let body = "";
  if (st.group) {
    const g = GROUPS[st.group];
    let current;
    for (const p of page) {
      const key = g.key(p);
      if (key !== current || body === "") {
        current = key;
        body += groupHeader(st, key, rows.filter((x) => g.key(x) === key).length, PLUGIN_COLUMNS.length);
      }
      if (!st.collapsed.has(String(key))) body += row(p);
    }
  } else body = page.map(row).join("");
  return `<table class="grid fixed plugins"><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table>`;
}

// ------------------------------------------------------------------ toolbar

/** A filter dropdown's face: the value when set, with a clear button. */
function filterSelect(act, k, value, open) {
  return `<span class="select ${value ? "set" : ""} ${open ? "open" : ""}" data-act="${act}"><span class="k">${k}</span> ${
    value ? esc(value) : "All"} ${value ? `<span class="x" data-act="clear-${act}" title="Clear">${icon("close")}</span>` : icon("expand_more")}</span>`;
}

function pluginsViewbar(st, total) {
  if (st.empty) return `<h1>Plugins</h1>${scanButton(st)}`;
  const states = st.states.length ? INSTALL_STATES.filter((s) => st.states.includes(s.id)).map((s) => s.label).join(", ") : null;
  const search = st.query
    ? `<span class="chip">${icon("search")}“${esc(st.query)}” · ${total} ${total === 1 ? "result" : "results"}<span data-act="clearsearch">${icon("close")}</span></span>`
    : "";
  return `<h1>Plugins</h1>${search}
    ${filterSelect("format", "Format", st.format, st.popover === "format")}
    ${filterSelect("vendor", "Vendor", st.vendor, st.popover === "vendor")}
    ${filterSelect("states", "Status", states, st.popover === "states")}
    <span class="select ${st.popover === "group" ? "open" : ""}" data-act="group"><span class="k">Group</span> ${st.group ? GROUPS[st.group].label : "None"} ${icon("expand_more")}</span>
    <span class="sep"></span>
    ${scanButton(st)}
    <span class="grow"></span>
    ${pager(st, total)}`;
}

/** Disabled while any scan runs, a project scan included (ADR-0038). */
function scanButton(st) {
  const running = st.scanStep != null;
  return tbBtn(running ? "hourglass_top" : "radar", running ? "Scanning…" : "Scan plugins",
    { title: running ? "A scan is running" : "Rescan the plugin folders", disabled: running, act: "scan" });
}

// ------------------------------------------------------------------ popovers

/** Format ▾: every format, with its plugin count from /api/v1/plugins/formats. */
function formatMenu(current) {
  const it = (v, label, n) => `<div class="it ${v === current ? "hot" : ""}" data-act="set-format" data-v="${esc(v || "")}">${
    icon("check", v === current ? "" : "blank")}<span class="grow">${esc(label)}</span>${n != null ? `<span class="n">${n}</span>` : ""}</div>`;
  return `<div class="pop picker pickmenu"><div class="list">${it(null, "All formats", data.plugins.length)}${
    PLUGIN_API.formats.map((f) => it(f.format, f.format, f.plugin_count)).join("")}</div></div>`;
}

/** Vendor ▾: 25 and growing, so it filters as you type, like the tag picker. */
function vendorPicker(current, query = "") {
  const q = query.toLowerCase();
  const hits = [...PLUGIN_API.vendors].sort((a, b) => a.vendor.localeCompare(b.vendor)).filter((v) => v.vendor.toLowerCase().includes(q));
  const it = (v, label, n) => `<div class="it ${v === current ? "hot" : ""}" data-act="set-vendor" data-v="${esc(v || "")}">${
    icon("check", v === current ? "" : "blank")}<span class="grow">${v ? highlight(label, query) : esc(label)}</span>${n != null ? `<span class="n">${n}</span>` : ""}</div>`;
  const items = (query ? "" : it(null, "All vendors", null)) + hits.map((v) => it(v.vendor, v.vendor, v.plugin_count)).join("");
  return pickerShell("Find a vendor", query, items || `<div class="hint">No vendor matches</div>`).replace('class="pop picker"', 'class="pop picker pickmenu"');
}

/** Status ▾: any of the three states (ADR-0025), each with its library count. */
function statesMenu(states) {
  const all = API["/api/v1/plugins/stats"];
  const n = { installed: all.installed_plugins, absent: all.missing_plugins, unscanned: all.unknown_plugins };
  const rows = INSTALL_STATES.map((s) => `<div class="it" data-act="toggle-state" data-v="${s.id}">${cb(states.includes(s.id))}${pluginStatus(s.value)}<span class="grow">${s.label}</span><span class="n">${n[s.id]}</span></div>`).join("");
  return `<div class="pop picker pickmenu"><div class="list" style="padding-top:var(--u)">${rows}</div>
    <div class="hint">None ticked shows all. Not scanned is not missing: no scan has looked yet.</div></div>`;
}

function groupMenu(group) {
  const it = (v, label) => `<div class="it ${v === group ? "hot" : ""}" data-act="set-group" data-v="${v || ""}">${icon("check", v === group ? "" : "blank")}<span class="grow">${label}</span></div>`;
  return `<div class="pop picker pickmenu" style="width:calc(var(--u) * 70)"><div class="list" style="padding-top:var(--u)">${
    it(null, "No grouping")}${it("vendor", "By vendor")}${it("format", "By format")}</div></div>`;
}

/** The row menu. Show in Explorer needs a path, so only an installed plugin has it. */
function pluginContextMenu(p, { tauri = true, hot = null } = {}) {
  return menu([
    { ic: "folder_open", label: "Show in Explorer", native: true, off: p.installed !== true, hot: hot === "explorer" },
    { ic: "audio_file", label: `Show the ${p.project_count} ${p.project_count === 1 ? "project" : "projects"} using it`, act: "showprojects", off: !p.project_count, hot: hot === "projects" },
    { sep: true },
    { ic: "content_copy", label: "Copy name", act: "close" },
  ], { tauri });
}

// ------------------------------------------------------------------ inspector

const yesNo = (b) => b == null ? null : b ? "Yes" : "No";
const plural = (n, one, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;

/** A property sheet from [label, value] pairs, leaving out what the scan did not report. */
function props(pairs) {
  const rows = pairs.filter(([, v]) => v != null && v !== "");
  return rows.length ? `<dl class="props">${rows.map(([k, v]) => `<dt>${k}</dt><dd>${v}</dd>`).join("")}</dl>` : "";
}

/** What the installed flag means for this plugin, and when a scan last said so. */
function pluginStateSection(p, d) {
  const when = d.last_scanned_at ? fmtDate(d.last_scanned_at) : null;
  if (p.installed === true) {
    return `<div class="insp-sec"><div class="state-line is-installed">${pluginStatus(true)}<b>Installed</b></div><p class="faint">Found by the scan on ${when}</p></div>`;
  }
  if (p.installed === false) {
    return `<div class="insp-sec"><div class="state-line is-absent">${pluginStatus(false)}<b>Missing</b></div><p class="faint">Not found by the scan on ${when}</p>
      ${d.path ? `<p class="faint" style="margin-top:calc(var(--u) * 2)">Last found at</p>${pathChip(d.path)}` : `<p class="faint">No scan has ever found it on this machine. Projects know it by name only.</p>`}</div>`;
  }
  return `<div class="insp-sec"><div class="state-line is-unscanned">${pluginStatus(null)}<b>Not scanned</b></div>
    <p class="faint">No plugin scan has looked for it yet, so it may be installed or not. Projects know it by name only.</p>
    <button class="btn" data-act="scan">${icon("radar")}Scan plugins</button></div>`;
}

function pluginAudio(d) {
  if (d.audio_in_channels == null && d.audio_out_channels == null) return null;
  const buses = d.audio_in_buses != null ? ` <span class="faint">(${plural(d.audio_in_buses, "bus", "buses")} in, ${d.audio_out_buses} out)</span>` : "";
  return `${d.audio_in_channels} in · ${d.audio_out_channels} out${buses}`;
}

function pluginMidi(d) {
  if (d.has_midi_input == null && d.has_midi_output == null) return null;
  return d.has_midi_input && d.has_midi_output ? "In and out" : d.has_midi_input ? "In" : d.has_midi_output ? "Out" : "None";
}

function pluginInspector(p) {
  const d = pluginDetails(p.id);
  const projects = pluginProjects(p.id);
  const vst3 = d.plugin_kind === "VST3";
  const scanned = d.path != null;
  const link = d.vendor_url ? `<a class="link" title="${esc(d.vendor_url)}">${esc(d.vendor_url.replace(/^https?:\/\//, ""))}</a>` : null;

  const sheet = scanned ? props([
    ["Category", d.category && esc(d.category.replace(/\|/g, " · "))],
    ["Type", d.is_instrument == null ? null : d.is_instrument ? "Instrument" : "Effect"],
    ["Audio", pluginAudio(d)],
    ["MIDI", pluginMidi(d)],
    ["Latency", d.latency_samples == null ? null : d.latency_samples ? `${d.latency_samples} samples` : "None"],
    ["Presets", d.presets],
    ["Parameters", d.parameters],
    ["Editor", yesNo(d.has_gui)],
    ["Website", link],
  ]) : "";

  const format = !scanned ? "" : vst3
    ? `<div class="insp-sec"><h3>Classes <span class="count">${d.classes.length || ""}</span></h3>
        ${d.classes.length ? inspectorList(d.classes, (c) => `<li>${icon(c.category === "Audio Module Class" ? "graphic_eq" : "tune")}<span class="grow">${esc(c.name)}</span><span class="faint">${esc(c.category.replace(/ Class$/, ""))}</span></li>`) : '<span class="faint">None reported</span>'}
      </div>
      <div class="insp-sec"><h3>Buses <span class="count">${d.buses.length || ""}</span></h3>
        ${d.buses.length ? inspectorList(d.buses, (b) => `<li>${icon(b.direction === "input" ? "input" : "output")}<span class="grow">${esc(b.name)}</span><span class="faint">${b.media === "event" ? "MIDI" : `${b.channel_count} ch`}${b.bus_type === 1 ? " · aux" : ""}</span></li>`, 10) : '<span class="faint">None reported</span>'}
      </div>`
    : `<div class="insp-sec"><h3>VST2</h3>${props([
        ["Unique ID", d.fourcc && `<code>${esc(d.fourcc)}</code>`],
        ["MIDI channels", d.midi_in_channels ? `${d.midi_in_channels} in` : "None"],
        ["Preset chunks", yesNo(d.preset_chunks)],
        ["64-bit audio", yesNo(d.f64_precision)],
        ["Silent when stopped", yesNo(d.silent_when_stopped)],
      ])}</div>`;

  const used = projects.length
    ? inspectorList([...projects].sort((a, b) => b.modified_at - a.modified_at), (x) => `<li>${icon("audio_file")}<span class="grow">${esc(x.name)}</span><span class="faint">${fmtDate(x.modified_at).slice(0, 10)}</span></li>`, 10)
      + (projects.length > 10 ? `<button class="linkbtn" data-act="showprojects">Show all ${projects.length} in Projects ${icon("arrow_forward")}</button>` : "")
    : '<span class="faint">No project uses it</span>';

  return `
    <div class="insp-head">
      <span class="glyph">${icon(p.format.includes("Instrument") ? "piano" : "graphic_eq")}</span>
      <div><h2>${esc(p.name)}</h2><div class="faint">${esc(p.vendor || "No vendor")} · ${esc(p.format)}${p.version ? ` · ${esc(p.version)}` : ""}</div>
      ${p.installed === true && d.path ? pathChip(d.path) : ""}</div>
    </div>
    ${pluginStateSection(p, d)}
    <div class="insp-sec"><h3>Used in <span class="count">${projects.length || ""}</span></h3>${used}</div>
    ${scanned ? `<div class="insp-sec"><h3>Plugin${p.installed === false ? ' <span class="count">as last scanned</span>' : ""}</h3>${sheet}</div>` : ""}
    ${format}
    <div class="insp-sec"><h3>Identity</h3>${props([
      ["Kind", d.plugin_kind],
      ["UID", `<code class="uid" data-copy="${esc(d.uid)}" title="${esc(d.uid)} · click to copy">${esc(d.uid)}</code>`],
    ])}
      <p class="faint" style="margin-top:var(--u)">Projects refer to it as</p>
      ${inspectorList(d.references, (r) => `<li>${icon("link")}<span class="grow" title="${esc(r.dev_identifier)}">${esc(r.ableton_name || r.dev_identifier)}</span><span class="faint">${r.ableton_format === "instr" ? "instrument" : "effect"} · matched by ${r.resolved_via === "created" ? "name only" : r.resolved_via.replace("_", " ")}</span></li>`)}
    </div>`;
}

const pluginInspectorEmpty = () => `<div class="empty">${icon("right_panel_open")}<p>Select a plugin to see what the scan recorded and which projects use it.</p></div>`;

// ------------------------------------------------------------------ empty states

function pluginsEmpty(st) {
  if (st.empty) {
    return `<div class="empty">${icon("plug_connect")}<h2>No plugins yet</h2>
      <p>A plugin scan lists what is installed in your plugin folders. Plugins your projects use are added as the projects are scanned.</p>
      <button class="btn primary" data-act="scan">${icon("radar")}Scan plugins</button></div>`;
  }
  const what = [st.query && `“${esc(st.query)}”`, st.format, st.vendor && esc(st.vendor),
    st.states.length && INSTALL_STATES.filter((s) => st.states.includes(s.id)).map((s) => s.label.toLowerCase()).join(" or ")].filter(Boolean);
  return `<div class="empty">${icon("search_off")}<h2>No plugins match</h2><p>${what.join(" · ")}</p>
    <button class="btn" data-act="clearfilters">Clear filters</button></div>`;
}

// ------------------------------------------------------------------ status bar

function pluginStatusSegments(st) {
  const s = pluginStats(st);
  const total = s.total_plugins;
  return [
    filtered(st) ? `${total} of ${data.plugins.length} plugins` : plural(total, "plugin"),
    `${pluginStatus(true)} ${s.installed_plugins} installed`,
    `${pluginStatus(false)} ${s.missing_plugins} missing`,
    `${pluginStatus(null)} ${s.unknown_plugins} not scanned`,
    plural(s.unique_vendors, "vendor"),
    ...(st.scanDone ? [`<span class="${st.scanDone.status === "completed" ? "is-installed" : "is-absent"}">${icon(st.scanDone.status === "completed" ? "task_alt" : "error")}</span> ${esc(st.scanDone.message)}`] : []),
  ];
}

// ------------------------------------------------------------------ the live screen

function pluginsScreen(st) {
  let timer = null;

  function draw(el) {
    const rows = pluginRows(st);
    const sel = st.selected && pluginById.get(st.selected);
    const box = el.querySelector(".content");
    const scroll = box ? [box.scrollTop, box.scrollLeft] : null;
    const ev = st.scanStep != null ? PLUGIN_API.scanEvents[st.scanStep] : null;

    el.innerHTML = renderShell({
      ...st,
      viewbar: pluginsViewbar(st, rows.length),
      content: rows.length ? pluginTable(st, rows) : pluginsEmpty(st),
      inspector: sel ? pluginInspector(sel) : pluginInspectorEmpty(),
      status: st.empty ? ["No plugins"] : pluginStatusSegments(st),
      scan: ev ? { done: ev.completed, total: ev.total, message: ev.message } : null,
      // A library no scan has filled has nothing in it at all yet.
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
      place(win, pluginContextMenu(pluginById.get(st.menu.id), { tauri: st.tauri }), { x, y });
    }
    const anchor = (act) => el.querySelector(`.viewbar [data-act="${act}"]`);
    if (st.popover === "format") place(win, formatMenu(st.format), { anchor: anchor("format") });
    if (st.popover === "vendor") place(win, vendorPicker(st.vendor, st.vendorQuery || ""), { anchor: anchor("vendor") });
    if (st.popover === "states") place(win, statesMenu(st.states), { anchor: anchor("states") });
    if (st.popover === "group") place(win, groupMenu(st.group), { anchor: anchor("group") });
  }

  /** Replays the real scan's event stream, as the SSE would deliver it. */
  function startScan(el) {
    st.scanStep = 0; st.scanDone = null;
    clearInterval(timer);
    timer = setInterval(() => {
      st.scanStep += 1;
      const ev = PLUGIN_API.scanEvents[st.scanStep];
      if (!ev || ev.status !== "scanning_plugins") {
        clearInterval(timer);
        st.scanStep = null; st.scanDone = ev || null;
      }
      draw(el);
    }, 450);
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
      const group = e.target.closest("tr.group");
      const inPop = e.target.closest(".pop");
      const v = hit?.dataset.v || null;

      if (act && act.startsWith("clear-")) {
        const what = act.slice(6);
        if (what === "states") st.states = []; else st[what] = null;
        st.page = 0; closeAll(); return redraw();
      }
      if (["format", "vendor", "states", "group"].includes(act)) {
        st.popover = st.popover === act ? null : act; st.menu = null; st.vendorQuery = ""; return redraw();
      }
      if (act === "set-format") { st.format = v; st.page = 0; closeAll(); return redraw(); }
      if (act === "set-vendor") { st.vendor = v; st.page = 0; closeAll(); return redraw(); }
      if (act === "set-group") { st.group = v; st.collapsed = new Set(); closeAll(); return redraw(); }
      if (act === "toggle-state") {
        st.states = st.states.includes(v) ? st.states.filter((x) => x !== v) : [...st.states, v];
        st.page = 0; return redraw();
      }
      if (act === "scan") { closeAll(); startScan(el); return redraw(); }
      if (act === "clearsearch") { st.query = ""; st.page = 0; return redraw(); }
      if (act === "clearfilters") { st.query = ""; st.vendor = null; st.format = null; st.states = []; st.page = 0; return redraw(); }
      if (act === "sidebar") { st.sidebarCollapsed = !st.sidebarCollapsed; return redraw(); }
      if (act === "inspector") { st.inspectorOpen = !st.inspectorOpen; return redraw(); }
      if (act === "prev" || act === "next") { st.page += act === "next" ? 1 : -1; el.querySelector(".content").scrollTop = 0; return redraw(); }
      if (act === "close" || act === "showprojects") { closeAll(); return redraw(); }
      if (inPop) return;

      if (group) {
        const k = group.dataset.group;
        st.collapsed.has(k) ? st.collapsed.delete(k) : st.collapsed.add(k);
        return redraw();
      }
      if (th) {
        const col = th.dataset.sort;
        st.sort = st.sort && st.sort.col === col ? { col, desc: !st.sort.desc } : { col, desc: col === "projects" };
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
