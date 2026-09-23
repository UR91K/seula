// Component renderers shared by every board (ADR-0036). A component drawn beside the
// screen and the same component inside the screen come from the same function here,
// never a copy. Everything returns an HTML string; state goes in as arguments.
//
// Depends on shell.js (data, formatting helpers, icon, esc, pathChip).

// ------------------------------------------------------------------ data helpers

const byId = (list) => new Map(list.map((x) => [x.id, x]));
const projectById = byId([...data.projects, ...data.archived]);
const collectionById = byId(data.collections);
const mediaById = byId(data.media);

const collectionsOf = (p) => p.collection_ids.map((id) => collectionById.get(id)).filter(Boolean);

/** The collection cover behind a project's play button: only when it is in exactly one. */
function projectCover(p) {
  const cols = collectionsOf(p);
  return cols.length === 1 ? coverUrl(cols[0]) : null;
}

const missingPlugins = (p) => p.plugins.filter((x) => x.installed === false).length;
const missingSamples = (p) => p.samples.filter((x) => !x.is_present).length;

/** Items common to every project in a list, by id. */
function common(projects, pick) {
  if (!projects.length) return [];
  const [first, ...rest] = projects;
  return pick(first).filter((x) => rest.every((p) => pick(p).some((y) => y.id === x.id)));
}

// ------------------------------------------------------------------ small controls

function cb(state) {
  const glyph = state === "mixed" ? icon("remove") : state ? icon("check") : "";
  return `<span class="cb ${state === "mixed" ? "mixed" : state ? "on" : ""}">${glyph}</span>`;
}

function tbBtn(ic, label, { title = label, disabled = false, caret = false, act = "", on = false } = {}) {
  return `<button class="tb-btn ${label ? "lbl" : ""} ${on ? "on" : ""}" title="${esc(title)}" ${disabled ? "disabled" : ""} ${act ? `data-act="${act}"` : ""}>${
    ic ? icon(ic) : ""}${label ? `<span>${esc(label)}</span>` : ""}${caret ? icon("expand_more", "caret") : ""}</button>`;
}

const tagChip = (t) => `<span class="tag">${esc(t.name)}</span>`;

// ------------------------------------------------------------------ projects table

/**
 * The reorderable columns. Leading cells (audition, name, tags) are fixed and not in
 * this list. `w` is the default width in --u; `hidden` columns start unchecked in the
 * column chooser.
 */
const PROJECT_COLUMNS = [
  { id: "modified", label: "Date modified", w: 56, cls: "dim", sort: (p) => p.modified_at, cell: (p) => fmtDate(p.modified_at) },
  { id: "created", label: "Date created", w: 56, cls: "dim", hidden: true, sort: (p) => p.created_at, cell: (p) => fmtDate(p.created_at) },
  { id: "length", label: "Length", w: 25, cls: "num", sort: (p) => p.duration_seconds ?? -1, cell: (p) => fmtLength(p.duration_seconds) },
  { id: "tempo", label: "Tempo", w: 26, cls: "num", sort: (p) => p.tempo, cell: (p) => fmtTempo(p.tempo) },
  { id: "key", label: "Key", w: 58, sort: (p) => fmtKey(p.key_signature), cell: (p) => esc(fmtKey(p.key_signature)) },
  { id: "time", label: "Time", w: 20, cls: "num", sort: (p) => p.time_signature.numerator / p.time_signature.denominator, cell: (p) => `${p.time_signature.numerator}/${p.time_signature.denominator}` },
  { id: "plugins", label: "Plugins", w: 27, cls: "num count", hover: "plugins", sort: (p) => p.plugins.length,
    cell: (p) => countCell(p.plugins.length, missingPlugins(p)) },
  { id: "samples", label: "Samples", w: 29, cls: "num count", hover: "samples", sort: (p) => p.samples.length,
    cell: (p) => countCell(p.samples.length, missingSamples(p)) },
  { id: "live", label: "Live", w: 32, cls: "dim", sort: (p) => [p.ableton_version.major, p.ableton_version.minor, p.ableton_version.patch].map((n) => String(n).padStart(3, "0")).join(""), cell: (p) => fmtVersion(p.ableton_version) },
];

function countCell(n, missing) {
  if (!n) return "";
  return `${n}${missing ? `<span class="miss" title="${missing} missing">${icon("error")}</span>` : ""}`;
}

/** Rows for the current scope, search and sort, before paging. */
function projectRows(st) {
  let rows;
  if (st.firstRun) return [];
  if (st.query) rows = (API[`/api/v1/search?query=${st.query}&limit=200`] || { projects: [] }).projects;
  else rows = st.scope === "archived" ? data.archived : data.projects;
  if (st.sort) {
    const col = PROJECT_COLUMNS.find((c) => c.id === st.sort.col) || { sort: (p) => p.name.toLowerCase() };
    const dir = st.sort.desc ? -1 : 1;
    rows = [...rows].sort((a, b) => {
      const x = col.sort(a), y = col.sort(b);
      return (x < y ? -1 : x > y ? 1 : 0) * dir;
    });
  }
  return rows;
}

function pageOf(st, rows) {
  const start = st.page * st.pageSize;
  return rows.slice(start, start + st.pageSize);
}

function leadCells(p, { renaming = false, showEdit = false, showAdd = false } = {}) {
  const cover = projectCover(p);
  const audio = p.audio_file_id
    ? `<span class="play ${cover ? "cover" : ""}" ${cover ? `style="background-image:url('${cover}')"` : ""} title="Play audition audio">${icon("play_arrow", "fill")}</span>`
    : `<span class="addaudio ${showAdd ? "show" : ""}" title="Add audition audio">${icon("add")}</span>`;
  const stem = fileStem(p.path);
  const name = renaming
    ? `<span class="namecell"><input value="${esc(p.name)}"></span>`
    : `<span class="namecell"><span class="n">${esc(p.name)}</span>${stem !== p.name ? `<span class="f">${esc(stem)}.als</span>` : ""}<span class="edit ${showEdit ? "show" : ""}" title="Rename (F2)">${icon("edit")}</span></span>`;
  return `<td class="lead">${audio}</td><td>${name}</td><td>${p.tags.map(tagChip).join("")}</td>`;
}

/**
 * st: { scope, query, sort, page, pageSize, selected: Set, columns: [id...], renaming, hot }
 * `columns` is the visible reorderable columns in order.
 */
function projectTable(st, rows) {
  const cols = st.columns.map((id) => PROJECT_COLUMNS.find((c) => c.id === id));
  const page = pageOf(st, rows);
  const allOn = page.length && page.every((p) => st.selected.has(p.id));
  const someOn = page.some((p) => st.selected.has(p.id));
  const sortIcon = (id) => st.sort && st.sort.col === id ? icon(st.sort.desc ? "arrow_downward" : "arrow_upward") : "";
  const th = (id, label, w, cls = "") =>
    `<th class="${cls} ${st.sort && st.sort.col === id ? "sorted" : ""}" data-sort="${id}" style="width:calc(var(--u) * ${w})">${label}${sortIcon(id)}<span class="grip"></span></th>`;

  const head = `<th class="lead" style="width:calc(var(--u) * 12)" data-act="selectall" title="Select all on this page">${cb(allOn ? true : someOn ? "mixed" : false)}</th>`
    + th("name", "Name", 100) + th("tags", "Tags", 64)
    + cols.map((c) => th(c.id, c.label, c.w, (c.cls || "").includes("num") ? "num" : "")).join("");

  const body = page.map((p) => `<tr data-id="${p.id}" class="${st.selected.has(p.id) ? "sel" : ""}">${
    leadCells(p, { renaming: st.renaming === p.id })}${
    cols.map((c) => `<td class="${c.cls || ""} ${st.hot && st.hot.id === p.id && st.hot.kind === c.hover ? "hot" : ""}" ${c.hover ? `data-hover="${c.hover}"` : ""}>${c.cell(p)}</td>`).join("")
  }</tr>`).join("");

  return `<table class="grid fixed"><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table>`;
}

// ------------------------------------------------------------------ view toolbar

function pager(st, total) {
  const from = total ? st.page * st.pageSize + 1 : 0;
  const to = Math.min(total, (st.page + 1) * st.pageSize);
  const last = Math.max(0, Math.ceil(total / st.pageSize) - 1);
  return `<span class="pager">${from}–${to} of ${total}
    ${tbBtn("chevron_left", "", { title: "Previous page", disabled: st.page === 0, act: "prev" })}
    ${tbBtn("chevron_right", "", { title: "Next page", disabled: st.page >= last, act: "next" })}
    <span class="select"><span class="k">Show</span> ${st.pageSize} ${icon("expand_more")}</span></span>`;
}

/** The projects view's toolbar row. Swaps its filters for batch actions at >1 selected. */
function projectsViewbar(st, total) {
  if (st.firstRun) return `<h1>Projects</h1>`;  // nothing to filter or page yet
  const n = st.selected.size;
  if (n > 1) {
    const archived = st.scope === "archived";
    return `<span class="count-sel">${n} selected</span>
      ${tbBtn("close", "", { title: "Clear selection", act: "clear" })}
      <span class="sep"></span>
      ${archived
        ? `${tbBtn("unarchive", "Reactivate", { act: "reactivate" })}
           ${tbBtn("delete_forever", "Delete…", { title: "Remove from Seula's database", act: "delete" })}`
        : `${tbBtn("sell", "Tags", { caret: true, act: "menu-tags" })}
           ${tbBtn("album", "Collection", { caret: true, act: "menu-collection" })}
           ${tbBtn("archive", "Archive", { act: "archive" })}
           ${tbBtn("delete_forever", "Delete…", { title: "Only archived projects can be deleted", disabled: true })}`}`;
  }
  const scope = st.scope === "archived" ? "Archived" : "Active";
  const search = st.query
    ? `<span class="chip">${icon("search")}“${esc(st.query)}” · ${total} ${total === 1 ? "result" : "results"}<span data-act="clearsearch">${icon("close")}</span></span>
       <span class="muted">${st.sort ? "" : "by relevance"}</span>`
    : `<span class="select" data-act="scope"><span class="k">Show</span> ${scope} ${icon("expand_more")}</span>
       <span class="select"><span class="k">Tag</span> Any ${icon("expand_more")}</span>`;
  return `<h1>Projects</h1>${search}
    <span class="sep"></span>
    ${st.tauri ? tbBtn("note_add", "", { title: "Add projects from anywhere (native-only)" }) : ""}
    ${tbBtn("view_column", "", { title: "Columns", act: "columns", on: st.popover === "columns" })}
    <span class="grow"></span>
    ${pager(st, total)}`;
}

// ------------------------------------------------------------------ menus

/** items: [{ ic, label, kbd, sub, off, hot, native, sep, head }] */
function menu(items, { tauri = true } = {}) {
  return `<div class="pop menu">${items.filter((i) => tauri || !i.native).map((i) =>
    i.sep ? `<div class="sep"></div>`
    : i.head ? `<div class="head">${esc(i.head)}</div>`
    : `<div class="mi ${i.off ? "off" : ""} ${i.hot ? "hot" : ""}" ${i.act ? `data-act="${i.act}"` : ""}>${
        i.ic ? icon(i.ic, i.fill ? "fill" : "") : icon("check", "blank")}<span class="lbl">${esc(i.label)}</span>${
        i.kbd ? `<span class="kbd">${i.kbd}</span>` : ""}${i.sub ? icon("chevron_right", "sub") : ""}</div>`).join("")}</div>`;
}

/** The row context menu. `many` disables the single-item entries (ADR: frontend.md). */
function projectContextMenu({ archived = false, many = false, tauri = true, hot = null } = {}) {
  if (archived) {
    return menu([
      { ic: "unarchive", label: many ? "Reactivate projects" : "Reactivate", act: "close" },
      { sep: true },
      { ic: "delete_forever", label: "Delete from Seula…", act: "delete" },
    ], { tauri });
  }
  return menu([
    { ic: "open_in_new", label: "Open in Ableton", kbd: "Enter", native: true, off: many, hot: hot === "open" },
    { ic: "folder_open", label: "Show in Explorer", native: true, off: many },
    { sep: true },
    { ic: "sell", label: "Tags", sub: true, hot: hot === "tags", act: "sub-tags" },
    { ic: "album", label: "Add to collection", sub: true, act: "sub-collection" },
    { sep: true },
    { ic: "edit", label: "Rename", kbd: "F2", off: many, act: "rename" },
    { ic: "music_note_add", label: "Add audition audio…", off: many },
    { sep: true },
    { ic: "archive", label: many ? "Archive projects" : "Archive", kbd: "Del", act: "close" },
  ], { tauri });
}

/** The Tags flyout: every tag, ticked where the project(s) have it. */
function tagSubmenu(projects) {
  const has = (t) => projects.filter((p) => p.tags.some((x) => x.id === t.tag_id)).length;
  const items = [...data.tags].sort((a, b) => b.project_count - a.project_count).slice(0, 10).map((t) => {
    const n = has(t);
    return { ic: n === projects.length ? "check" : n ? "remove" : null, label: t.name };
  });
  return menu([...items, { sep: true }, { ic: "add", label: "New tag…" }, { ic: "tune", label: "Manage tags…", act: "managetags" }]);
}

/** The batch toolbar's two dropdowns. */
const batchTagsMenu = () => menu([
  { ic: "add", label: "Add tags…", act: "picker-tag" },
  { ic: "remove", label: "Remove tags…", act: "picker-untag" },
  { sep: true },
  { ic: "tune", label: "Manage tags…", act: "managetags" },
]);
const batchCollectionMenu = () => menu([
  { ic: "playlist_add", label: "Add to collection…", act: "picker-collection" },
  { ic: "playlist_remove", label: "Remove from collection…", act: "picker-uncollection" },
  { sep: true },
  { ic: "library_add", label: "New collection from selection…", act: "newcollection" },
]);

// ------------------------------------------------------------------ pickers

function highlight(text, q) {
  if (!q) return esc(text);
  const i = text.toLowerCase().indexOf(q.toLowerCase());
  return i < 0 ? esc(text) : `${esc(text.slice(0, i))}<b>${esc(text.slice(i, i + q.length))}</b>${esc(text.slice(i + q.length))}`;
}

function pickerShell(placeholder, query, items, hint = "") {
  return `<div class="pop picker">
    <label class="field focus">${icon("search")}${query ? `<span>${esc(query)}</span>` : `<span style="color:var(--text-3)">${esc(placeholder)}</span>`}<span class="caret-blink"></span></label>
    <div class="list">${items}</div>${hint ? `<div class="hint">${hint}</div>` : ""}</div>`;
}

/** Add a tag to the selection. Autocompletes; offers to create when nothing matches. */
function tagPicker(query = "", hot = 0) {
  const q = query.toLowerCase();
  const hits = [...data.tags].sort((a, b) => b.project_count - a.project_count).filter((t) => t.name.toLowerCase().includes(q));
  const exact = hits.some((t) => t.name.toLowerCase() === q);
  const items = hits.map((t, i) => `<div class="it ${i === hot ? "hot" : ""}">${icon("sell")}<span class="grow">${highlight(t.name, query)}</span><span class="n">${t.project_count}</span></div>`).join("")
    + (query && !exact ? `<div class="it create ${hits.length === 0 ? "hot" : ""}">${icon("add")}<span class="grow">Create tag “${esc(query)}”</span></div>` : "");
  return pickerShell("Add a tag", query, items, "Enter adds to all selected projects");
}

/** Remove a tag: only tags every selected project has. */
function untagPicker(projects) {
  const tags = common(projects, (p) => p.tags);
  const items = tags.length
    ? tags.map((t, i) => `<div class="it ${i === 0 ? "hot" : ""}">${icon("sell")}<span class="grow">${esc(t.name)}</span></div>`).join("")
    : `<div class="hint">No tag is on all ${projects.length} projects</div>`;
  return pickerShell("Remove a tag", "", items, tags.length ? `Tags on all ${projects.length} selected projects` : "");
}

/** Add to a collection, with create-new at the top. */
function collectionPicker(query = "") {
  const q = query.toLowerCase();
  const hits = data.collections.filter((c) => c.name.toLowerCase().includes(q));
  const art = (c) => coverUrl(c) ? `<img src="${coverUrl(c)}" alt="">` : `<span class="noart"></span>`;
  const create = `<div class="it create ${query && !hits.length ? "hot" : ""}">${icon("library_add")}<span class="grow">${query ? `New collection “${esc(query)}”` : "New collection…"}</span></div>`;
  const items = create + hits.map((c, i) => `<div class="it ${!query && i === 0 ? "hot" : ""}">${art(c)}<span class="grow">${highlight(c.name, query)}</span><span class="n">${c.project_count}</span></div>`).join("");
  return pickerShell("Find a collection", query, items);
}

/** Remove from a collection: only collections every selected project is in. */
function uncollectionPicker(projects) {
  const cols = common(projects.map((p) => ({ ...p, cols: collectionsOf(p) })), (p) => p.cols);
  const items = cols.length
    ? cols.map((c) => `<div class="it">${icon("album")}<span class="grow">${esc(c.name)}</span></div>`).join("")
    : `<div class="hint">No collection holds all ${projects.length} projects</div>`;
  return pickerShell("Remove from collection", "", items);
}

// ------------------------------------------------------------------ popovers

/** The list behind a plugins or samples count cell, on hover. */
function countHoverList(p, kind) {
  const plugins = kind === "plugins";
  const items = plugins ? p.plugins : p.samples;
  const missing = plugins ? missingPlugins(p) : missingSamples(p);
  const unknown = plugins ? p.plugins.filter((x) => x.installed == null).length : 0;
  const rows = items.map((x) => plugins
    ? `<li>${pluginStatus(x.installed)}<span class="grow">${esc(x.name)}</span><span class="faint">${esc(x.vendor || "")}</span></li>`
    : `<li>${sampleStatus(x.is_present)}<span class="grow" title="${esc(x.path)}">${esc(x.name)}</span></li>`).join("");
  return `<div class="pop hoverlist"><div class="title">${plugins ? "Plugins" : "Samples"} · ${items.length}${
    missing ? `<span class="miss">${missing} missing</span>` : ""}${unknown ? `<span class="faint">${unknown} not scanned</span>` : ""}</div><ul class="plain">${rows}</ul></div>`;
}

/** Show, hide and reorder columns. Fixed columns are listed but locked. */
function columnChooser(visible) {
  const fixed = ["Audition", "Name", "Tags"].map((l) => `<div class="it" style="color:var(--text-3)">${cb(true)}<span class="grow">${l}</span>${icon("lock")}</div>`).join("");
  const order = [...visible, ...PROJECT_COLUMNS.map((c) => c.id).filter((id) => !visible.includes(id))];
  const rest = order.map((id) => {
    const c = PROJECT_COLUMNS.find((x) => x.id === id);
    return `<div class="it">${cb(visible.includes(id))}<span class="grow">${c.label}</span><span class="faint">${icon("drag_indicator")}</span></div>`;
  }).join("");
  return `<div class="pop picker"><div class="list" style="max-height:none;padding-top:var(--u)">${fixed}<div class="menu"><div class="sep"></div></div>${rest}</div>
    <div class="hint">Drag to reorder · <span style="color:var(--accent)">Reset</span></div></div>`;
}

// ------------------------------------------------------------------ dialogs

function dialog(title, body, buttons, { width = 170 } = {}) {
  return `<div class="pop dialog" style="width:calc(var(--u) * ${width})">
    <div class="dh"><span class="grow">${esc(title)}</span><span class="x" data-act="dismiss">${icon("close")}</span></div>
    <div class="db">${body}</div>
    <div class="df">${buttons}</div></div>`;
}
const btn = (label, cls = "", act = "dismiss") => `<button class="btn ${cls}" data-act="${act}">${esc(label)}</button>`;

/** Tag manager. mode: null | { rename: tag_id } | { confirmDelete: tag_id } */
function tagManager(mode = null) {
  if (mode && mode.confirmDelete) {
    const t = data.tags.find((x) => x.tag_id === mode.confirmDelete);
    return dialog("Delete tag", `<div class="warn">${icon("warning", "fill")}<div>
      <p>Delete the tag <b>${esc(t.name)}</b>?</p>
      <p class="dim">It is removed from ${t.project_count} ${t.project_count === 1 ? "project" : "projects"}. The projects themselves are not changed.</p></div></div>`,
      btn("Cancel") + btn("Delete tag", "danger"), { width: 150 });
  }
  const rows = [...data.tags].sort((a, b) => a.name.localeCompare(b.name)).map((t) => {
    const renaming = mode && mode.rename === t.tag_id;
    return `<tr class="${renaming ? "sel" : ""}"><td>${renaming ? `<span class="namecell"><input value="${esc(t.name)}"></span>` : `${icon("sell")} ${esc(t.name)}`}</td>
      <td class="num">${t.project_count}</td>
      <td style="text-align:right" class="faint">${icon("edit")} ${icon("delete")}</td></tr>`;
  }).join("");
  return dialog("Manage tags", `
    <div style="display:flex;gap:calc(var(--u) * 3)"><label class="field" style="flex:1">${icon("sell")}<span style="color:var(--text-3)">New tag name</span></label>${btn("Create", "", "none")}</div>
    <div class="gridbox"><table class="grid"><thead><tr><th>Tag</th><th class="num" style="width:calc(var(--u) * 30)">Projects</th><th style="width:calc(var(--u) * 24)"></th></tr></thead><tbody>${rows}</tbody></table></div>
    ${mode && mode.rename ? `<p class="dim" style="margin-top:calc(var(--u) * 3)">Renaming applies to every project with this tag.</p>` : ""}`,
    btn("Close", "primary"), { width: 180 });
}

/** Permanent removal from the database. The file on disk is never touched. */
function deletePermanentDialog(projects) {
  const n = projects.length;
  const what = n === 1 ? `<b>${esc(projects[0].name)}</b>` : `${n} projects`;
  return dialog("Delete from Seula", `<div class="warn">${icon("warning", "fill")}<div>
    <p>Remove ${what} from Seula's database?</p>
    <p class="dim">${n === 1 ? "Its" : "Their"} tags, notes, tasks and collection places are deleted with ${n === 1 ? "it" : "them"}. The .als ${n === 1 ? "file stays" : "files stay"} on disk, untouched; a rescan of ${n === 1 ? "its folder" : "their folders"} would find ${n === 1 ? "it" : "them"} again.</p></div></div>`,
    btn("Cancel") + btn(n === 1 ? "Delete project" : `Delete ${n} projects`, "danger"), { width: 160 });
}

function newCollectionDialog(projects) {
  return dialog("New collection", `
    <label>Name</label><label class="field focus"><span class="caret-blink"></span></label>
    <label>Description <span class="faint">(optional)</span></label><textarea class="notes" placeholder="What ties these together?"></textarea>
    <p class="dim" style="margin-top:calc(var(--u) * 3)">${projects.length} selected projects will be added, in the order shown.</p>`,
    btn("Cancel") + btn("Create collection", "primary"), { width: 160 });
}

// ------------------------------------------------------------------ inspector

function inspectorList(items, render, max = 8) {
  return `<ul class="plain">${items.slice(0, max).map(render).join("")}${
    items.length > max ? `<li class="faint">and ${items.length - max} more</li>` : ""}</ul>`;
}

function auditionSection(p) {
  if (!p.audio_file_id) {
    return `<div class="insp-sec"><h3>Audition</h3><div class="audition"><span class="faint" style="flex:1">No audition audio</span><button class="btn">${icon("music_note_add")}Add audio…</button></div></div>`;
  }
  const m = mediaById.get(p.audio_file_id);
  return `<div class="insp-sec"><h3>Audition</h3><div class="audition">
    <span class="big">${icon("play_arrow", "fill")}</span>
    <div class="track"><div class="bar"><i></i></div>
    <div class="meta"><span class="fn">${esc(m ? m.original_filename : "")}</span><span>${m ? fmtBytes(m.file_size_bytes) : ""}</span></div></div></div></div>`;
}

/** tasksSel: Set of selected task ids, for the bulk task actions. */
function tasksSection(p, tasksSel = new Set()) {
  const done = p.tasks.filter((t) => t.completed).length;
  const any = tasksSel.size > 0;
  const all = p.tasks.length && p.tasks.every((t) => tasksSel.has(t.id));
  const acts = p.tasks.length ? `<span class="acts">
      <span title="Select all">${cb(all ? true : any ? "mixed" : false)}</span>
      ${tbBtn("check_circle", "", { title: "Mark done", disabled: !any })}
      ${tbBtn("radio_button_unchecked", "", { title: "Mark not done", disabled: !any })}
      ${tbBtn("delete", "", { title: "Delete selected", disabled: !any })}</span>` : "";
  const rows = p.tasks.map((t) => `<li data-task="${t.id}" class="${t.completed ? "done" : ""} ${tasksSel.has(t.id) ? "sel" : ""}">${cb(tasksSel.has(t.id))}${
    icon(t.completed ? "check_circle" : "radio_button_unchecked", `state ${t.completed ? "fill" : ""}`)}<span class="grow">${esc(t.description)}</span></li>`).join("");
  return `<div class="insp-sec"><h3>Tasks <span class="count">${p.tasks.length ? `${done}/${p.tasks.length}` : ""}</span>${acts}</h3>
    ${p.tasks.length ? `<ul class="plain tasks">${rows}</ul>` : ""}
    <label class="field add">${icon("add")}Add a task</label></div>`;
}

function projectInspector(p, { tasksSel } = {}) {
  const cols = collectionsOf(p);
  const cover = projectCover(p);
  const pm = missingPlugins(p), sm = missingSamples(p);
  return `
    <div class="insp-head">
      ${cover ? `<img src="${cover}" alt="">` : ""}
      <div><h2>${esc(p.name)}</h2>${pathChip(p.path)}</div>
    </div>
    ${auditionSection(p)}
    <div class="insp-sec"><h3>Project</h3>
      <dl class="props">
        <dt>Tempo</dt><dd>${fmtTempo(p.tempo)} BPM</dd>
        <dt>Key</dt><dd>${esc(fmtKey(p.key_signature)) || '<span class="faint">None detected</span>'}</dd>
        <dt>Time</dt><dd>${p.time_signature.numerator}/${p.time_signature.denominator}</dd>
        <dt>Length</dt><dd>${fmtLength(p.duration_seconds) || '<span class="faint">Unknown</span>'}</dd>
        <dt>Live</dt><dd>${fmtVersion(p.ableton_version)}</dd>
        <dt>Created</dt><dd>${fmtDate(p.created_at)}</dd>
        <dt>Modified</dt><dd>${fmtDate(p.modified_at)}</dd>
      </dl>
    </div>
    <div class="insp-sec"><h3>Tags</h3><div class="tags">${p.tags.map(tagChip).join("")}<span class="tag-add" title="Add a tag">${icon("add")}</span></div></div>
    <div class="insp-sec"><h3>Notes</h3><textarea class="notes" placeholder="Add notes">${esc(p.notes)}</textarea></div>
    ${tasksSection(p, tasksSel)}
    <div class="insp-sec"><h3>Collections <span class="count">${cols.length || ""}</span></h3>
      ${cols.length ? inspectorList(cols, (c) => `<li>${icon("album")}<span class="grow">${esc(c.name)}</span></li>`) : '<span class="faint">In no collection</span>'}</div>
    <div class="insp-sec"><h3>Plugins <span class="count">${p.plugins.length || ""}</span>${pm ? `<span class="count is-missing">${pm} missing</span>` : ""}</h3>
      ${p.plugins.length ? inspectorList(p.plugins, (x) => `<li>${pluginStatus(x.installed)}<span class="grow">${esc(x.name)}</span><span class="faint">${esc(x.vendor || "")}</span></li>`, 12) : '<span class="faint">None</span>'}</div>
    <div class="insp-sec"><h3>Samples <span class="count">${p.samples.length || ""}</span>${sm ? `<span class="count is-missing">${sm} missing</span>` : ""}</h3>
      ${p.samples.length ? inspectorList(p.samples, (x) => `<li>${sampleStatus(x.is_present)}<span class="grow" title="${esc(x.path)}">${esc(x.name)}</span></li>`) : '<span class="faint">None</span>'}</div>`;
}

/** Several projects selected: what they share, not a list of them. */
function multiInspector(projects) {
  const n = projects.length;
  const secs = projects.reduce((s, p) => s + (p.duration_seconds || 0), 0);
  const tempos = projects.map((p) => p.tempo);
  const tags = common(projects, (p) => p.tags);
  const cols = common(projects.map((p) => ({ ...p, cols: collectionsOf(p) })), (p) => p.cols);
  const pm = projects.reduce((s, p) => s + missingPlugins(p), 0);
  return `<div class="insp-head insp-multi"><div><h2>${n} projects</h2><div class="faint">Selected</div></div></div>
    <div class="insp-sec"><h3>Together</h3><dl class="props">
      <dt>Length</dt><dd>${fmtLength(secs)}</dd>
      <dt>Tempo</dt><dd>${fmtTempo(Math.min(...tempos))}–${fmtTempo(Math.max(...tempos))} BPM</dd>
      <dt>Plugins</dt><dd>${new Set(projects.flatMap((p) => p.plugins.map((x) => x.id))).size} distinct${pm ? ` · <span class="is-missing">${pm} missing</span>` : ""}</dd>
      <dt>Samples</dt><dd>${new Set(projects.flatMap((p) => p.samples.map((x) => x.id))).size} distinct</dd>
    </dl></div>
    <div class="insp-sec"><h3>Tags on all ${n}</h3>${tags.length ? `<div class="tags">${tags.map(tagChip).join("")}</div>` : '<span class="faint">None in common</span>'}</div>
    <div class="insp-sec"><h3>Collections holding all ${n}</h3>${cols.length ? inspectorList(cols, (c) => `<li>${icon("album")}<span class="grow">${esc(c.name)}</span></li>`) : '<span class="faint">None in common</span>'}</div>
    <div class="insp-sec"><h3>Projects</h3>${inspectorList(projects, (p) => `<li>${icon("audio_file")}<span class="grow">${esc(p.name)}</span></li>`, 12)}</div>`;
}

const inspectorEmpty = () => `<div class="empty">${icon("right_panel_open")}<p>Select a project to see its details here.</p></div>`;

// ------------------------------------------------------------------ empty states

function projectsEmpty(st) {
  if (st.query) {
    return `<div class="empty">${icon("search_off")}<h2>No projects match “${esc(st.query)}”</h2>
      <p>Search covers names, paths, plugins, samples, tags, notes, keys and tempos.</p></div>`;
  }
  if (data.config.needs_setup) {
    return `<div class="empty">${icon("create_new_folder")}<h2>No project folders yet</h2>
      <p>${esc(data.config.status_message)}</p>
      <button class="btn primary">${icon("add")}Add a folder…</button></div>`;
  }
  return `<div class="empty">${icon("inventory_2")}<h2>No archived projects</h2></div>`;
}
