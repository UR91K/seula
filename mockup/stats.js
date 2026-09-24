// The stats view (ADR-0036, ADR-0045): its renderers and its live screen. One scrolling
// page from GET /api/v1/system/statistics, in either project scope; no inspector.
//
// Every value shown is the snapshotted response for that scope
// (mockup/data/generate.py). Series arrive oldest first with empty periods as zero, and
// the tempo histogram in equal 10 BPM bins, so the charts draw them as they come.

/** GET /api/v1/system/statistics, in a scope */
const statsFor = (scope = "active") => API[`/api/v1/system/statistics${scope === "all" ? "?scope=all" : ""}`];

function statsState(overrides = {}) {
  return {
    view: "stats", tauri: true, inspectorOpen: false, sidebarCollapsed: false,
    scope: "active",        // the project scope preference, shared with the other views
    popover: null,          // "scope"
    hover: null,            // the data-hid of the mark whose tooltip shows
    band: null,             // a band to scroll to on the first draw: "plugins" | "activity" | "library"
    empty: false,           // a library with nothing scanned yet
    ...overrides,
  };
}

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
const MONTHS_LONG = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const pct = (x) => `${Math.round(x * 100)}%`;
const dec1 = (x) => (Math.round(x * 10) / 10).toFixed(1);

/** A hoverable mark: its id, for the tooltip the screen places, and the tooltip itself. */
const tipAttrs = (hid, tip) => `data-hid="${esc(hid)}" data-tip="${esc(tip)}"`;

// ------------------------------------------------------------------ overview

/** One overview count: the total, a part-to-whole bar, and each part with its state.
 *  parts: [{ n, label, cls, glyph, off }] -- `off` for a part the scope leaves out. */
function statTile(label, total, parts, { note = "" } = {}) {
  const shown = parts.filter((p) => p.n > 0);
  const whole = shown.reduce((s, p) => s + p.n, 0);
  const bar = whole
    ? `<div class="st-split">${shown.map((p) => `<i class="${p.cls} ${p.off ? "off" : ""}" style="flex:${p.n}" ${tipAttrs(`${label}:${p.label}`, `${p.n} ${p.label}`)}></i>`).join("")}</div>`
    : `<div class="st-split none"></div>`;
  return `<div class="st-tile">
    <div class="st-k">${label}</div>
    <div class="st-n">${total.toLocaleString("en-US")}${note ? ` <span class="st-note">${note}</span>` : ""}</div>
    ${bar}
    <ul class="st-parts">${parts.map((p) => `<li class="${p.off ? "off" : ""}"><span class="st-glyph ${p.cls}">${p.glyph}</span><span class="n">${p.n.toLocaleString("en-US")}</span> ${p.label}${p.off ? ' <span class="faint">not counted</span>' : ""}</li>`).join("")}</ul>
  </div>`;
}

const swatch = `<i class="st-sw"></i>`;

function overviewTiles(s, scope) {
  const { projects: p, plugins: pl, samples: sa, collections: c, tags: t, tasks: k } = s;
  return `<div class="st-tiles">
    ${statTile("Projects", p.total, [
      { n: p.active, label: "active", cls: "is-accent", glyph: swatch },
      { n: p.archived, label: "archived", cls: "is-quiet", glyph: icon("inventory_2"), off: scope === "active" },
    ])}
    ${statTile("Plugins", pl.total, [
      { n: pl.installed, label: "installed", cls: "is-installed", glyph: pluginStatus(true) },
      { n: pl.missing, label: "missing", cls: "is-absent", glyph: pluginStatus(false) },
      { n: pl.not_scanned, label: "not scanned", cls: "is-unscanned", glyph: pluginStatus(null) },
    ])}
    ${statTile("Samples", sa.total, [
      { n: sa.present, label: "present", cls: "is-installed", glyph: sampleStatus(true) },
      { n: sa.missing, label: "missing", cls: "is-absent", glyph: sampleStatus(false) },
    ])}
    ${statTile("Collections", c.total, [
      { n: c.with_projects, label: "with projects", cls: "is-accent", glyph: swatch },
      { n: c.empty, label: "empty", cls: "is-quiet", glyph: icon("check_box_outline_blank") },
    ])}
    ${statTile("Tags", t.total, [
      { n: t.in_use, label: "in use", cls: "is-accent", glyph: swatch },
      { n: t.unused, label: "unused", cls: "is-quiet", glyph: icon("label_off") },
    ])}
    ${statTile("Tasks", k.total, [
      { n: k.completed, label: "completed", cls: "is-installed", glyph: icon("task_alt") },
      { n: k.pending, label: "pending", cls: "is-quiet", glyph: icon("radio_button_unchecked") },
    ], { note: k.total ? `${pct(k.completion_rate)} done` : "" })}
  </div>`;
}

// ------------------------------------------------------------------ chart parts

/**
 * Vertical columns from one baseline. items: [{ n, label, tip, hid, sub }]. Only the
 * tallest column is labelled; the rest say their value on hover. `avg` draws a
 * reference line with its value.
 */
function columnChart(items, { height = 40, avg = null, cls = "", stacked = null } = {}) {
  const max = Math.max(1, ...items.map((i) => i.n));
  const peak = items.reduce((b, i) => (i.n > b.n ? i : b), items[0] || { n: 0 });
  const cols = items.map((i) => {
    const h = (100 * i.n) / max;
    const inner = stacked
      ? stacked(i).filter((seg) => seg.n > 0).map((seg) => `<i class="${seg.cls}" style="height:${(100 * seg.n) / max}%"></i>`).join("")
      : `<i style="height:${h}%"></i>`;
    return `<div class="st-col ${i.n ? "" : "zero"}" ${tipAttrs(i.hid, i.tip)}>
      <div class="st-bar">${i === peak && i.n ? `<span class="st-cap" style="bottom:${h}%">${i.n}</span>` : ""}${inner}</div>
      <div class="st-x">${i.label}${i.sub ? `<br><span class="faint">${i.sub}</span>` : ""}</div>
    </div>`;
  }).join("");
  const line = avg != null && avg > 0
    ? `<div class="st-avg" style="bottom:${(100 * avg) / max}%"><span>avg ${dec1(avg)}</span></div>` : "";
  return `<div class="st-cols ${cls}" style="--plot:calc(var(--u) * ${height})"><div class="st-plot">${line}</div>${cols}</div>`;
}

/**
 * Horizontal bars in rows at table density, the value in its own column. items:
 * [{ label, n, tip, hid, cls }]. The bar's length is relative to the largest item.
 */
function barList(items, { more = 0, moreLabel = "more", labelW = 50 } = {}) {
  const max = Math.max(1, ...items.map((i) => i.n));
  return `<table class="st-bars" style="--lw:calc(var(--u) * ${labelW})"><tbody>${items.map((i) => `
    <tr class="${i.cls || ""}" ${tipAttrs(i.hid, i.tip)}><td class="l">${i.label}</td><td class="b"><i style="width:${(100 * i.n) / max}%"></i></td><td class="num">${i.n}</td></tr>`).join("")}
    ${more ? `<tr class="more"><td class="l faint" colspan="3">and ${more} ${moreLabel}</td></tr>` : ""}</tbody></table>`;
}

const statPanel = (title, body, { span = 3, count = "", cls = "" } = {}) =>
  `<section class="st-panel ${cls}" style="grid-column:span ${span}"><h3>${title}${count !== "" ? ` <span class="count">${count}</span>` : ""}</h3>${body}</section>`;

const statBand = (id, title, panels) => `<h2 class="st-band" data-band="${id}">${title}</h2><div class="st-grid">${panels}</div>`;

/** A project named on the page, marked when it is archived (the `all` scope). */
const projectName = (p) => `<span class="n">${esc(p.name)}</span>${p.is_active ? "" : ` <span class="faint" title="Archived">${icon("inventory_2")}</span>`}`;

// ------------------------------------------------------------------ music

function tempoChart(s) {
  return columnChart(s.tempo_distribution.map((b) => ({
    n: b.count, label: `${b.tempo}`, hid: `tempo:${b.tempo}`,
    tip: `${b.tempo}–${b.tempo + 9} BPM · ${plural(b.count, "project")}`,
  })), { height: 44 });
}

function keyList(s) {
  const keyed = s.key_distribution.filter((k) => k.key);
  const none = s.key_distribution.find((k) => !k.key);
  const top = keyed.slice(0, 12).map((k) => ({
    label: esc(fmtKey(k.key)), n: k.count, hid: `key:${k.key.tonic}-${k.key.scale}`, tip: `${fmtKey(k.key)} · ${plural(k.count, "project")}`,
  }));
  if (none) top.push({ label: "No key", n: none.count, cls: "quiet", hid: "key:none", tip: `No key detected · ${plural(none.count, "project")}` });
  return barList(top, { more: Math.max(0, keyed.length - 12), moreLabel: "more keys", labelW: 46 });
}

function timeSignatureList(s) {
  return barList(s.time_signature_distribution.map((t) => ({
    label: `${t.numerator}/${t.denominator}`, n: t.count, hid: `ts:${t.numerator}/${t.denominator}`,
    tip: `${t.numerator}/${t.denominator} · ${plural(t.count, "project")}`,
  })), { labelW: 20 });
}

function lengthFacts(s) {
  const l = s.longest_project;
  return props([
    ["Average", fmtLength(s.average_project_duration_seconds)],
    ["Longest", l && `${projectName(l)} <span class="faint">${fmtLength(l.duration_seconds)}</span>`],
    ["Under 40 s", plural(s.projects_under_40_seconds, "project")],
  ]);
}

// ------------------------------------------------------------------ plugins and samples

function topPluginsTable(s) {
  const max = Math.max(1, ...s.top_plugins.map((p) => p.usage_count));
  return `<table class="grid st-table"><thead><tr><th>Plugin</th><th>Vendor</th><th class="num" style="width:calc(var(--u) * 60)">Projects</th></tr></thead><tbody>${
    s.top_plugins.map((p) => `<tr><td><span class="n">${esc(p.name)}</span></td><td class="dim">${esc(p.vendor)}</td><td class="num">${inlineBar(p.usage_count, max)}</td></tr>`).join("")}</tbody></table>`;
}

function topVendorsTable(s) {
  const max = Math.max(1, ...s.top_vendors.map((v) => v.usage_count));
  return `<table class="grid st-table"><thead><tr><th>Vendor</th><th class="num" style="width:calc(var(--u) * 30)">Plugins</th><th class="num" style="width:calc(var(--u) * 60)">Uses</th></tr></thead><tbody>${
    s.top_vendors.map((v) => `<tr><td><span class="n">${esc(v.vendor)}</span></td><td class="num dim">${v.plugin_count}</td><td class="num">${inlineBar(v.usage_count, max)}</td></tr>`).join("")}</tbody></table>`;
}

function topSamplesTable(s) {
  const max = Math.max(1, ...s.top_samples.map((x) => x.usage_count));
  return `<table class="grid fixed st-table"><thead><tr><th style="width:calc(var(--u) * 110)">Sample</th><th>Folder</th><th class="num" style="width:calc(var(--u) * 60)">Projects</th></tr></thead><tbody>${
    s.top_samples.map((x) => `<tr><td><span class="n">${esc(x.name)}</span></td><td>${folderCell(x.path)}</td><td class="num">${inlineBar(x.usage_count, max)}</td></tr>`).join("")}</tbody></table>`;
}

/** A count with a bar behind it, in a table's last column. */
const inlineBar = (n, max) => `<span class="st-inline"><i style="width:${(70 * n) / max}%"></i><span>${n}</span></span>`;

function perProjectFacts(s) {
  return props([
    ["Plugins", `${dec1(s.average_plugins_per_project)} <span class="faint">per project</span>`],
    ["Samples", `${dec1(s.average_samples_per_project)} <span class="faint">per project</span>`],
  ]) + `<p class="faint st-foot">Over every project counted, including those with none.</p>`;
}

// ------------------------------------------------------------------ activity

function monthChart(s) {
  const n = s.projects_per_month.length;
  const avg = n ? s.projects_per_month.reduce((a, m) => a + m.count, 0) / n : 0;
  return columnChart(s.projects_per_month.map((m, i) => ({
    n: m.count, label: MONTHS[m.month - 1], sub: i === 0 || m.month === 1 ? m.year : "",
    hid: `month:${m.year}-${m.month}`, tip: `${MONTHS_LONG[m.month - 1]} ${m.year} · ${plural(m.count, "project")} created`,
  })), { height: 44, avg });
}

function yearChart(s) {
  return columnChart(s.projects_per_year.map((y) => ({
    n: y.count, label: `${y.year}`, hid: `year:${y.year}`, tip: `${y.year} · ${plural(y.count, "project")} created`,
  })), { height: 44 });
}

/** The last thirty days as two strips, created and modified: small multiples on one
 *  day axis, so neither needs a legend. */
function activityStrips(s) {
  const days = s.recent_activity;
  const max = Math.max(1, ...days.map((d) => Math.max(d.projects_created, d.projects_modified)));
  const date = (d) => new Date(Date.UTC(d.year, d.month - 1, d.day));
  const day = (d) => date(d).toLocaleDateString("en-GB", { weekday: "short", day: "numeric", month: "short", timeZone: "UTC" });
  const strip = (key, label) => `<div class="st-strip"><span class="st-sl">${label}</span>${days.map((d) =>
    `<span class="st-cell ${d[key] ? "" : "zero"}" ${tipAttrs(`${key}:${d.year}-${d.month}-${d.day}`, `${day(d)} · ${d.projects_created} created, ${d.projects_modified} modified`)}><i style="height:${(100 * d[key]) / max}%"></i></span>`).join("")}</div>`;
  const ticks = `<div class="st-strip axis"><span class="st-sl"></span>${days.map((d, i) =>
    `<span class="st-cell">${(days.length - 1 - i) % 7 === 0 ? `${d.day} ${MONTHS[d.month - 1]}` : ""}</span>`).join("")}</div>`;
  const created = days.reduce((a, d) => a + d.projects_created, 0);
  const modified = days.reduce((a, d) => a + d.projects_modified, 0);
  return `${strip("projects_created", `Created <span class="faint">${created}</span>`)}${strip("projects_modified", `Modified <span class="faint">${modified}</span>`)}${ticks}`;
}

// ------------------------------------------------------------------ library

function complexTable(s) {
  return `<table class="grid st-table"><thead><tr><th>Project</th><th class="num" style="width:calc(var(--u) * 30)">Plugins</th><th class="num" style="width:calc(var(--u) * 30)">Samples</th><th class="num" style="width:calc(var(--u) * 26)">Total</th></tr></thead><tbody>${
    s.most_complex_projects.map((c) => `<tr class="${c.project.is_active ? "" : "archived"}"><td>${projectName(c.project)}</td><td class="num dim">${c.plugin_count}</td><td class="num dim">${c.sample_count}</td><td class="num">${c.complexity_score}</td></tr>`).join("")}</tbody></table>`;
}

function versionList(s) {
  const top = s.ableton_versions.slice(0, 8);
  return barList(top.map((v) => ({ label: esc(v.version), n: v.count, hid: `ver:${v.version}`, tip: `Live ${v.version} · ${plural(v.count, "project")}` })),
    { more: s.ableton_versions.length - top.length, moreLabel: "older versions", labelW: 30 });
}

function tagList(s) {
  return barList(s.top_tags.map((t) => ({ label: tagChip(t), n: t.usage_count, hid: `tag:${t.name}`, tip: `${t.name} · ${plural(t.usage_count, "project")}` })), { labelW: 40 });
}

function statsCollectionFacts(s) {
  const c = s.largest_collection;
  const cover = c && coverUrl(c);
  const largest = c
    ? `<div class="st-largest">${cover ? `<img src="${cover}" alt="">` : `<span class="noart"></span>`}<div><div class="n">${esc(c.name)}</div><div class="faint">${plural(c.project_count, "project")} · ${fmtLength(c.total_duration_seconds)}</div></div></div>`
    : '<span class="faint">No collection holds a project</span>';
  return props([["Average", `${dec1(s.average_projects_per_collection)} <span class="faint">projects each</span>`]])
    + `<div class="st-sub">Largest</div>${largest}`;
}

function taskChart(s) {
  return `<div class="st-legend"><span><i class="is-installed"></i>Completed</span><span><i class="is-quiet"></i>Pending</span></div>`
    + columnChart(s.task_completion_trends.map((t, i) => ({
      n: t.total_tasks, label: MONTHS[t.month - 1].slice(0, 1), sub: i === 0 || t.month === 1 ? `${t.year}`.slice(2) : "",
      hid: `task:${t.year}-${t.month}`,
      tip: t.total_tasks ? `${MONTHS_LONG[t.month - 1]} ${t.year} · ${t.completed_tasks} of ${t.total_tasks} completed (${pct(t.completion_rate)})` : `${MONTHS_LONG[t.month - 1]} ${t.year} · no tasks created`,
    })), {
      height: 30, cls: "narrow",
      stacked: (i) => {
        const t = s.task_completion_trends.find((x) => `task:${x.year}-${x.month}` === i.hid);
        return [{ cls: "is-quiet", n: t.total_tasks - t.completed_tasks }, { cls: "is-installed", n: t.completed_tasks }];
      },
    });
}

// ------------------------------------------------------------------ the page

function statsPage(st) {
  const s = statsFor(st.scope);
  return `<div class="st-page">
    ${overviewTiles(s, st.scope)}
    ${statBand("music", "Music", [
      statPanel("Tempo", tempoChart(s), { span: 4, count: "BPM, 10 per bar" }),
      statPanel("Length", lengthFacts(s), { span: 2 }),
      statPanel("Keys", keyList(s), { span: 3, count: s.key_distribution.filter((k) => k.key).length }),
      statPanel("Time signatures", timeSignatureList(s), { span: 3, count: s.time_signature_distribution.length }),
    ].join(""))}
    ${statBand("plugins", "Plugins and samples", [
      statPanel("Most used plugins", topPluginsTable(s), { span: 3 }),
      statPanel("Top vendors", topVendorsTable(s), { span: 3 }),
      statPanel("Most used samples", topSamplesTable(s), { span: 4 }),
      statPanel("Averages", perProjectFacts(s), { span: 2 }),
    ].join(""))}
    ${statBand("activity", "Activity", [
      statPanel("Created per month", monthChart(s), { span: 4, count: "last 12 months" }),
      statPanel("Created per year", yearChart(s), { span: 2 }),
      statPanel("Last 30 days", activityStrips(s), { span: 6 }),
    ].join(""))}
    ${statBand("library", "Library", [
      statPanel("Most complex projects", complexTable(s), { span: 3, count: "plugins + samples" }),
      statPanel("Ableton versions", versionList(s), { span: 3, count: s.ableton_versions.length }),
      statPanel("Top tags", tagList(s), { span: 2 }),
      statPanel("Collections", statsCollectionFacts(s), { span: 2 }),
      statPanel("Tasks per month", taskChart(s), { span: 2, count: "by month created" }),
    ].join(""))}
  </div>`;
}

// ------------------------------------------------------------------ toolbar

const SCOPES = { active: "Active projects", all: "All projects" };

function statsViewbar(st) {
  if (st.empty) return `<h1>Stats</h1>`;
  return `<h1>Stats</h1>
    <span class="select ${st.popover === "scope" ? "open" : ""}" data-act="scope" title="The project scope, shared with Plugins, Samples and Collections"><span class="k">Counting</span> ${SCOPES[st.scope]} ${icon("expand_more")}</span>
    <span class="sep"></span>
    ${tbBtn("download", "Export CSV", { title: `Every figure on this page, as a CSV file (${SCOPES[st.scope].toLowerCase()})`, act: "export" })}`;
}

/** Counting ▾: the scope preference, with how many projects each counts. */
function scopeMenu(current) {
  const p = statsFor("all").projects;
  const it = (v, label, n) => `<div class="it ${v === current ? "hot" : ""}" data-act="set-scope" data-v="${v}">${
    icon("check", v === current ? "" : "blank")}<span class="grow">${label}</span><span class="n">${n}</span></div>`;
  return `<div class="pop picker pickmenu" style="width:calc(var(--u) * 110)"><div class="list" style="max-height:none;padding-top:var(--u)">${
    it("active", "Active projects", p.active)}${it("all", "All projects, archived too", p.total)}</div>
    <div class="hint">The same setting as in Plugins, Samples and Collections</div></div>`;
}

// ------------------------------------------------------------------ status bar

function statsStatusSegments(st) {
  if (st.empty) return ["No projects"];
  const p = statsFor(st.scope).projects;
  return st.scope === "all"
    ? [`${p.total} projects`, `${icon("inventory_2", "sb")} Counting archived projects`]
    : [`${p.active} active projects`, `<span class="faint">${p.archived} archived, not counted</span>`];
}

const statsEmpty = () => `<div class="empty">${icon("bar_chart")}<h2>Nothing to count yet</h2>
  <p>Statistics fill in as your projects are scanned.</p></div>`;

// ------------------------------------------------------------------ the live screen

function statsScreen(st) {
  function draw(el) {
    const box = el.querySelector(".content");
    const scroll = box ? box.scrollTop : null;

    el.innerHTML = renderShell({
      ...st,
      viewbar: statsViewbar(st),
      content: st.empty ? statsEmpty() : statsPage(st),
      inspector: "",
      status: statsStatusSegments(st),
      counts: st.empty ? { projects: 0, collections: 0, plugins: 0, samples: 0 } : null,
    });

    const win = el.querySelector(".win");
    const content = el.querySelector(".content");
    if (scroll != null) content.scrollTop = scroll;
    else if (st.band) {
      const h = el.querySelector(`[data-band="${st.band}"]`);
      if (h) content.scrollTop = h.offsetTop - content.offsetTop;
    }

    if (st.popover === "scope") place(win, scopeMenu(st.scope), { anchor: el.querySelector('.viewbar [data-act="scope"]') });
    if (st.hover) {
      const mark = el.querySelector(`[data-hid="${CSS.escape(st.hover)}"]`);
      if (mark) {
        mark.classList.add("hot");
        const tip = place(win, `<div class="pop tip">${esc(mark.dataset.tip)}</div>`, { anchor: mark });
        // Above the mark, centred on it, where it does not cover what it describes.
        const z = win.getBoundingClientRect().width / win.offsetWidth || 1;
        const a = mark.getBoundingClientRect(), w = win.getBoundingClientRect();
        const left = (a.left + a.width / 2 - w.left) / z - tip.offsetWidth / 2;
        const top = (a.top - w.top) / z - tip.offsetHeight - 4;
        tip.style.left = `${Math.max(0, Math.min(left, win.offsetWidth - tip.offsetWidth - 2))}px`;
        tip.style.top = `${Math.max(0, top)}px`;
      }
    }
  }

  function wire(el) {
    const redraw = () => draw(el);

    el.addEventListener("mouseover", (e) => {
      const mark = e.target.closest("[data-hid]");
      const hid = mark ? mark.dataset.hid : null;
      if (hid === st.hover || e.target.closest(".pop")) return;
      st.hover = hid;
      redraw();
    });
    el.addEventListener("mouseleave", () => { if (st.hover) { st.hover = null; redraw(); } });

    el.addEventListener("click", (e) => {
      const hit = e.target.closest("[data-act]");
      const act = hit?.dataset.act;
      const v = hit?.dataset.v || null;
      if (act === "scope") { st.popover = st.popover ? null : "scope"; return redraw(); }
      if (act === "set-scope") { st.scope = v; st.popover = null; return redraw(); }
      if (act === "sidebar") { st.sidebarCollapsed = !st.sidebarCollapsed; return redraw(); }
      if (act === "export") return;
      if (e.target.closest(".pop")) return;
      if (st.popover) { st.popover = null; redraw(); }
    });
  }

  return (el) => {
    if (!el.dataset.wired) { el.dataset.wired = "1"; wire(el); }
    draw(el);
  };
}
