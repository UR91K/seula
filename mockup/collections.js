// The collections view (ADR-0036, ADR-0044): the grid and details layouts, an opened
// collection's tracklist, and their live screen. The tracklist is the projects table
// and the inspector is the shared one; only what is particular to collections is here.
//
// Every value shown is a snapshotted response (mockup/data/generate.py): the list in
// each order the Sort menu and the table headers ask for, a search, and each
// collection's detail, tracklist, tasks and statistics in both project scopes
// (ADR-0043).

const scopeQ = (scope) => (scope === "all" ? "?scope=all" : "");
/** GET /api/v1/collections, in a scope, or sorted (the snapshot sorts the active list). */
const colList = (scope = "active") => API[`/api/v1/collections?${scope === "all" ? "scope=all&" : ""}limit=10000`].collections;
const colSorted = (col, desc) => API[`/api/v1/collections?sort_by=${col}&sort_desc=${desc}&limit=10000`];
const colSearch = (q) => (API[`/api/v1/collections/search?query=${encodeURIComponent(q)}&limit=10000`] || { collections: [] }).collections;
const colStats = (id, scope) => API[`/api/v1/collections/${id}/statistics${scopeQ(scope)}`];
const colTasks = (id, scope) => API[`/api/v1/collections/${id}/tasks${scopeQ(scope)}`];
/** GET /api/v1/collections/:id/projects -- the tracklist, in collection order */
const colTracks = (id, scope) => apiGet(`/api/v1/collections/${id}/projects${scopeQ(scope)}`);
const colById = (id, scope = "active") => colList(scope).find((c) => c.id === id);

/** A collection's length: minutes and seconds, or hours when it runs past one. */
function fmtTotal(seconds) {
  if (seconds == null) return "";
  const s = Math.round(seconds);
  return s < 3600 ? fmtLength(s) : `${Math.floor(s / 3600)}:${pad2(Math.floor((s % 3600) / 60))}:${pad2(s % 60)}`;
}

const coverImg = (c, cls = "") => {
  const art = coverUrl(c);
  return art ? `<img class="${cls}" src="${art}" alt="">` : `<span class="noart ${cls}">${icon("album")}</span>`;
};

function collectionsState(overrides = {}) {
  return {
    view: "collections", tauri: true, inspectorOpen: true, sidebarCollapsed: false,
    scope: "active",        // the project scope preference, shared with plugins and samples
    layout: "grid",         // "grid" | "details", a preference (ADR-0031)
    sort: { col: "name", desc: false },  // the list's server sort_by
    query: "",
    selected: null,         // a collection id, in the list
    open: null,             // the opened collection's id (ADR-0044)
    trackSort: null,        // null: collection order; or { col, desc } over the tracklist
    order: null,            // the opened collection's ids after a drag or removal here
    tracksSel: new Set(),   // selected projects in the opened collection
    tasksSel: new Set(),
    columns: PROJECT_COLUMNS.filter((c) => !c.hidden).map((c) => c.id),
    menu: null,             // { id, kind: "card" | "track", x?, y?, hot? }
    popover: null,          // "sort" | "columns"
    dialog: null,           // "new" | "edit" | "duplicate" | "delete"
    renaming: null,
    drag: null,             // { id, to } a tracklist row being dragged
    hot: null,              // { id, kind } a count cell's hover list
    empty: false,           // no collections yet
    ...overrides,
  };
}

// ------------------------------------------------------------------ the list

/** Grid Sort ▾: what each choice asks the server for. */
const COLLECTION_SORTS = [
  { label: "Name", col: "name", desc: false },
  { label: "Most projects", col: "project_count", desc: true },
  { label: "Longest", col: "total_duration", desc: true },
  { label: "Newest", col: "created_at", desc: true },
  { label: "Recently changed", col: "modified_at", desc: true },
];

/** The details table's columns. `key` is the server's sort_by; description has none. */
const COLLECTION_COLUMNS = [
  { id: "name", key: "name", label: "Name", w: 110, cell: (c) => `<span class="n">${esc(c.name)}</span>` },
  { id: "description", label: "Description", w: 150, cls: "dim", cell: (c) => esc(c.description || "") },
  { id: "projects", key: "project_count", label: "Projects", w: 30, cls: "num", cell: (c) => c.project_count || "" },
  { id: "length", key: "total_duration", label: "Length", w: 32, cls: "num", cell: (c) => fmtTotal(c.total_duration_seconds) },
  { id: "created", key: "created_at", label: "Date created", w: 56, cls: "dim", cell: (c) => fmtDate(c.created_at) },
  { id: "modified", key: "modified_at", label: "Date modified", w: 56, cls: "dim", cell: (c) => fmtDate(c.modified_at) },
];

/** The collections the list shows: a search, or the list in the chosen order. */
function collectionRows(st) {
  if (st.empty) return [];
  if (st.query) return colSearch(st.query);
  const sorted = st.scope === "active" && colSorted(st.sort.col, st.sort.desc);
  if (sorted) return sorted.collections;
  // Only the active list is snapshotted in every order; the all-scope list is by name.
  const key = { name: (c) => c.name, project_count: (c) => c.project_count, total_duration: (c) => c.total_duration_seconds || 0,
    created_at: (c) => c.created_at, modified_at: (c) => c.modified_at }[st.sort.col];
  const cmp = (x, y) => (x < y ? -1 : x > y ? 1 : 0);
  return [...colList(st.scope)].sort((a, b) => cmp(key(a), key(b)) * (st.sort.desc ? -1 : 1) || cmp(a.name, b.name));
}

/** A card: cover, name, and the project count and length. */
function collectionCard(c, { selected = false, renaming = false } = {}) {
  const meta = c.project_count ? `${plural(c.project_count, "project")} · ${fmtTotal(c.total_duration_seconds)}` : "Empty";
  const name = renaming ? `<span class="namecell"><input value="${esc(c.name)}"></span>` : `<div class="cname" title="${esc(c.name)}">${esc(c.name)}</div>`;
  return `<div class="card ${selected ? "sel" : ""}" data-card="${c.id}">
    <div class="art">${coverImg(c)}</div>${name}<div class="cmeta">${meta}</div></div>`;
}

function collectionGrid(st, rows) {
  return `<div class="cards">${rows.map((c) => collectionCard(c, { selected: st.selected === c.id, renaming: st.renaming === c.id })).join("")}</div>`;
}

function collectionTable(st, rows) {
  const sortIcon = (key) => !st.query && st.sort.col === key ? icon(st.sort.desc ? "arrow_downward" : "arrow_upward") : "";
  const head = `<th class="lead" style="width:calc(var(--u) * 12)" title="Cover"></th>` + COLLECTION_COLUMNS.map((c) =>
    `<th class="${c.cls === "num" ? "num" : ""} ${!st.query && st.sort.col === c.key ? "sorted" : ""}" ${c.key ? `data-sort="${c.key}"` : ""} style="width:calc(var(--u) * ${c.w})">${c.label}${c.key ? sortIcon(c.key) : ""}<span class="grip"></span></th>`).join("");
  const body = rows.map((c) => `<tr data-card="${c.id}" class="${st.selected === c.id ? "sel" : ""}"><td class="lead">${coverImg(c, "thumb")}</td>${
    COLLECTION_COLUMNS.map((col) => `<td class="${col.cls || ""}">${col.cell(c)}</td>`).join("")}</tr>`).join("");
  return `<table class="grid fixed collections"><thead><tr>${head}</tr></thead><tbody>${body}</tbody></table>`;
}

// ------------------------------------------------------------------ an opened collection

/** The tracklist in view: collection order, less anything removed here, then any sort. */
function trackRows(st) {
  const tracks = colTracks(st.open, st.scope);
  let rows = st.order ? st.order.map((id) => tracks.find((p) => p.id === id)).filter(Boolean) : tracks;
  const s = st.trackSort;
  if (s && s.col !== "position") {
    const col = PROJECT_COLUMNS.find((c) => c.id === s.col) || { sort: (p) => p.name.toLowerCase() };
    const cmp = (x, y) => (x < y ? -1 : x > y ? 1 : 0);
    rows = [...rows].sort((a, b) => cmp(col.sort(a), col.sort(b)) * (s.desc ? -1 : 1));
  }
  return rows;
}

/** The facts line, from /collections/:id/statistics and /tasks, leaving out what is absent. */
function collectionFacts(stats, tasks) {
  return [
    plural(stats.project_count, "project"),
    fmtTotal(stats.total_duration_seconds),
    stats.average_tempo != null && `${Math.round(stats.average_tempo)} BPM average`,
    stats.most_common_key && esc(fmtKey(stats.most_common_key)),
    stats.most_common_time_signature,
    stats.total_plugins && plural(stats.total_plugins, "plugin"),
    stats.total_samples && plural(stats.total_samples, "sample"),
    tasks.total_tasks && `${tasks.completed_tasks} of ${plural(tasks.total_tasks, "task")} done`,
  ].filter(Boolean);
}

/** The band above an opened collection's tracklist: cover, name, description, facts. */
function collectionHeader(c, scope) {
  const facts = collectionFacts(colStats(c.id, scope), colTasks(c.id, scope));
  return `<div class="colhead">${coverImg(c, "big")}<div class="txt">
    <h2>${esc(c.name)}</h2>
    ${c.description ? `<p class="desc">${esc(c.description)}</p>` : ""}
    <p class="facts">${facts.map((f) => `<span>${f}</span>`).join("")}</p></div></div>`;
}

function tracksEmpty() {
  return `<div class="empty tracks-empty">${icon("playlist_add")}<h2>Nothing in this collection yet</h2>
    <p>Add projects from the Projects view: select them, then Collection ▾ › Add to collection.</p></div>`;
}

/** The tracklist: the projects table, numbered by place, draggable in collection order. */
function collectionTracks(st, rows) {
  const tracks = st.order || colTracks(st.open, st.scope).map((p) => p.id);
  const position = new Map(tracks.map((id, i) => [id, i + 1]));
  const inOrder = !st.trackSort || st.trackSort.col === "position";
  const tst = { ...st, sort: st.trackSort || { col: "position", desc: false }, selected: st.tracksSel, page: 0, pageSize: 1000 };
  return projectTable(tst, rows, { position, drag: inOrder, dragging: st.drag, archived: st.scope === "all" });
}

// ------------------------------------------------------------------ toolbars

const layoutSwitch = (layout) => `<span class="layoutswitch">${
  tbBtn("grid_view", "", { title: "Grid", act: "layout-grid", on: layout === "grid" })}${
  tbBtn("view_list", "", { title: "Details", act: "layout-details", on: layout === "details" })}</span>`;

function collectionsViewbar(st, total) {
  const add = tbBtn("library_add", "New collection", { act: "new" });
  if (st.empty) return `<h1>Collections</h1>${add}`;
  const sort = COLLECTION_SORTS.find((s) => s.col === st.sort.col && s.desc === st.sort.desc) || COLLECTION_SORTS[0];
  const lead = st.query
    ? `<span class="chip">${icon("search")}“${esc(st.query)}” · ${total} ${total === 1 ? "result" : "results"}<span data-act="clearsearch">${icon("close")}</span></span>`
    : st.layout === "grid"
      ? `<span class="select ${st.popover === "sort" ? "open" : ""}" data-act="sort"><span class="k">Sort</span> ${sort.label} ${icon("expand_more")}</span>`
      : "";
  return `<h1>Collections</h1>${lead}<span class="sep"></span>${add}<span class="grow"></span>${layoutSwitch(st.layout)}`;
}

function openedViewbar(st, c) {
  const n = st.tracksSel.size;
  const s = st.trackSort && st.trackSort.col !== "position" && (PROJECT_COLUMNS.find((x) => x.id === st.trackSort.col) || { label: "Name" });
  return `${tbBtn("arrow_back", "", { title: "Back to collections", act: "back" })}
    <span class="crumb" data-act="back">Collections</span>${icon("chevron_right", "crumbsep")}<h1>${esc(c.name)}</h1>
    ${n ? `<span class="sep"></span><span class="count-sel">${n} selected</span>${tbBtn("close", "", { title: "Clear selection", act: "clear" })}
      ${tbBtn("playlist_remove", "Remove from collection", { act: "removetracks" })}` : ""}
    <span class="sep"></span>
    ${tbBtn("view_column", "", { title: "Columns", act: "columns", on: st.popover === "columns" })}
    ${s ? `<span class="muted">Sorted by ${esc(s.label.toLowerCase())}. Sort by # to drag.</span>` : ""}`;
}

/** Sort ▾ for the grid. */
function collectionSortMenu(sort) {
  const it = (s, i) => {
    const on = s.col === sort.col && s.desc === sort.desc;
    return `<div class="it ${on ? "hot" : ""}" data-act="set-sort" data-v="${i}">${icon("check", on ? "" : "blank")}<span class="grow">${s.label}</span></div>`;
  };
  return `<div class="pop picker pickmenu" style="width:calc(var(--u) * 80)"><div class="list" style="max-height:none;padding-top:var(--u)">${COLLECTION_SORTS.map(it).join("")}</div></div>`;
}

// ------------------------------------------------------------------ menus

/** A card's menu, or a row's in the details table. Nothing here is native-only. */
function collectionContextMenu({ hot = null } = {}) {
  return menu([
    { ic: "open_in_full", label: "Open", kbd: "Enter", act: "open", hot: hot === "open" },
    { ic: "audio_file", label: "Show in Projects", act: "close", hot: hot === "projects" },
    { sep: true },
    { ic: "edit", label: "Rename", kbd: "F2", act: "rename" },
    { ic: "edit_note", label: "Edit details…", act: "edit", hot: hot === "edit" },
    { ic: "content_copy", label: "Duplicate…", act: "duplicate" },
    { sep: true },
    { ic: "delete", label: "Delete…", kbd: "Del", act: "delete" },
  ]);
}

// ------------------------------------------------------------------ dialogs

function editCollectionDialog(c) {
  const art = coverUrl(c);
  return dialog("Edit collection", `
    <div class="coveredit"><div class="art">${coverImg(c)}</div><div>
      <div class="row">${btn(art ? "Change image…" : "Choose image…", "", "none")}${art ? btn("Remove", "", "none") : ""}</div>
      <p class="faint">Or drop an image here. It shows on the card, and behind the play button of any project in only this collection.</p></div></div>
    <label>Name</label><label class="field focus"><input value="${esc(c.name)}"></label>
    <label>Description</label><textarea class="notes" placeholder="What ties these together?">${esc(c.description || "")}</textarea>`,
    btn("Cancel") + btn("Save", "primary"), { width: 180 });
}

function duplicateCollectionDialog(c) {
  return dialog("Duplicate collection", `
    <label>Name of the copy</label><label class="field focus"><input value="${esc(c.name)} copy"></label>
    <p class="dim" style="margin-top:calc(var(--u) * 3)">The copy has the same ${plural(c.project_count, "project")} in the same order, and the same description and cover.</p>`,
    btn("Cancel") + btn("Duplicate", "primary"), { width: 160 });
}

function deleteCollectionDialog(c) {
  return dialog("Delete collection", `<div class="warn">${icon("warning", "fill")}<div>
    <p>Delete the collection <b>${esc(c.name)}</b>?</p>
    <p class="dim">${c.project_count ? `Its ${plural(c.project_count, "project")} ${c.project_count === 1 ? "is" : "are"} not touched: ${c.project_count === 1 ? "it stays" : "they stay"} in Seula and in any other collection.` : "It holds no projects."} Only the list and its order are deleted.</p></div></div>`,
    btn("Cancel") + btn("Delete collection", "danger"), { width: 160 });
}

// ------------------------------------------------------------------ inspector

/** A collection in the inspector: from the grid, or an opened one with no row selected,
 *  where the tracklist is already on screen and is left out. */
function collectionInspector(c, { scope = "active", tasksSel = new Set(), opened = false } = {}) {
  const stats = colStats(c.id, scope);
  const tasks = colTasks(c.id, scope);
  const tracks = colTracks(c.id, scope);
  const archivedN = tracks.filter((p) => !p.is_active).length;
  const tracklist = tracks.length
    ? `<ul class="plain tracklist">${tracks.slice(0, 10).map((p, i) => `<li class="${p.is_active ? "" : "archived"}"><span class="num">${i + 1}</span><span class="grow">${esc(p.name)}</span>${
        p.is_active ? "" : `<span class="faint" title="Archived">${icon("inventory_2")}</span>`}<span class="faint">${fmtLength(p.duration_seconds)}</span></li>`).join("")}</ul>
      <button class="linkbtn" data-act="open" data-id="${c.id}">${tracks.length > 10 ? `Open to see all ${tracks.length}` : "Open"} ${icon("arrow_forward")}</button>`
    : '<span class="faint">No projects yet</span>';
  return `
    <div class="insp-head insp-col">${coverImg(c, "cover-lg")}<div>
      <h2>${esc(c.name)}</h2>${c.description ? `<p class="desc">${esc(c.description)}</p>` : ""}</div></div>
    <div class="insp-sec"><h3>Collection</h3>${props([
      ["Projects", stats.project_count ? `${stats.project_count}${archivedN ? ` <span class="faint">(${archivedN} archived)</span>` : ""}` : '<span class="faint">None yet</span>'],
      ["Length", fmtTotal(stats.total_duration_seconds)],
      ["Tempo", stats.average_tempo == null ? null : `${Math.round(stats.average_tempo)} BPM <span class="faint">average</span>`],
      ["Key", stats.most_common_key && `${esc(fmtKey(stats.most_common_key))} <span class="faint">most common</span>`],
      ["Time", stats.most_common_time_signature && `${stats.most_common_time_signature} <span class="faint">most common</span>`],
      ["Plugins", stats.total_plugins || null],
      ["Samples", stats.total_samples || null],
      ["Tags", stats.total_tags || null],
      ["Created", fmtDate(c.created_at)],
      ["Modified", fmtDate(c.modified_at)],
    ])}</div>
    ${opened ? "" : `<div class="insp-sec"><h3>Tracklist <span class="count">${tracks.length || ""}</span></h3>${tracklist}</div>`}
    ${tasksSection({ tasks: tasks.tasks }, tasksSel, { showProject: true })}`;
}

const collectionInspectorEmpty = () => `<div class="empty">${icon("right_panel_open")}<p>Select a collection to see its tracklist and tasks here. Double-click to open it.</p></div>`;

// ------------------------------------------------------------------ empty states

function collectionsEmpty(st) {
  if (st.query) {
    return `<div class="empty">${icon("search_off")}<h2>No collections match “${esc(st.query)}”</h2>
      <p>Search covers collection names, descriptions and notes. To find projects, search from Projects.</p></div>`;
  }
  return `<div class="empty">${icon("album")}<h2>No collections yet</h2>
    <p>A collection is an ordered list of projects: an EP, a set, a batch of ideas. Select projects in Projects and choose Collection ▾ › New collection from selection, or start an empty one.</p>
    <button class="btn primary" data-act="new">${icon("library_add")}New collection</button></div>`;
}

// ------------------------------------------------------------------ status bar

function collectionStatusSegments(st, rows) {
  const scope = st.scope === "all" ? [`${icon("inventory_2", "sb")} Counting archived projects`] : [];
  if (st.open) {
    const secs = rows.reduce((n, p) => n + (p.duration_seconds || 0), 0);
    return [plural(rows.length, "project"), ...(rows.length ? [fmtTotal(secs)] : []),
      ...(st.tracksSel.size ? [`${st.tracksSel.size} selected`] : []), ...scope];
  }
  if (st.empty) return ["No collections"];
  const all = colList(st.scope).length;
  return [st.query ? `${rows.length} of ${all} collections` : plural(rows.length, "collection"), ...scope];
}

// ------------------------------------------------------------------ the live screen

function collectionsScreen(st) {
  let place_ = null;  // what the content showed last, so a scroll position is not carried across

  function draw(el) {
    const where = `${st.open}|${st.layout}`;
    const box = el.querySelector(".content");
    const scroll = box && place_ === where ? [box.scrollTop, box.scrollLeft] : null;
    place_ = where;

    let viewbar, content, inspector, rows;
    if (st.open) {
      const c = colById(st.open, st.scope);
      rows = trackRows(st);
      const sel = [...st.tracksSel].map((id) => rows.find((p) => p.id === id)).filter(Boolean);
      viewbar = openedViewbar(st, c);
      content = collectionHeader(c, st.scope) + (rows.length ? collectionTracks(st, rows) : tracksEmpty());
      inspector = sel.length === 1 ? projectInspector(sel[0]) : sel.length > 1 ? multiInspector(sel)
        : collectionInspector(c, { scope: st.scope, tasksSel: st.tasksSel, opened: true });
    } else {
      rows = collectionRows(st);
      const sel = st.selected && rows.find((c) => c.id === st.selected);
      viewbar = collectionsViewbar(st, rows.length);
      content = rows.length ? (st.layout === "grid" ? collectionGrid(st, rows) : collectionTable(st, rows)) : collectionsEmpty(st);
      inspector = sel ? collectionInspector(sel, { scope: st.scope, tasksSel: st.tasksSel }) : collectionInspectorEmpty();
    }

    const target = st.open ? colById(st.open, st.scope) : st.selected && colById(st.selected, st.scope);
    const modal = st.dialog === "new" ? newCollectionDialog([])
      : st.dialog === "edit" ? editCollectionDialog(target)
      : st.dialog === "duplicate" ? duplicateCollectionDialog(target)
      : st.dialog === "delete" ? deleteCollectionDialog(target) : "";

    el.innerHTML = renderShell({
      ...st,
      viewbar, content, inspector,
      status: collectionStatusSegments(st, rows),
      overlay: modal ? `<div class="scrim">${modal}</div>` : "",
      counts: st.empty ? { projects: data.projects.length, collections: 0 } : null,
    });

    const win = el.querySelector(".win");
    const content_ = el.querySelector(".content");
    if (scroll) [content_.scrollTop, content_.scrollLeft] = scroll;

    if (st.menu) {
      const sel = `[data-${st.menu.kind === "track" ? "id" : "card"}="${st.menu.id}"]`;
      const item = el.querySelector(`.content ${sel}`);
      let { x, y } = st.menu;
      if (x == null && item) {
        // Fixed frames: open where a right-click on the name would.
        const r = (item.querySelector(".cname, .n, .namecell") || item).getBoundingClientRect(), w = win.getBoundingClientRect(), z = w.width / win.offsetWidth;
        x = (r.left - w.left) / z + 30; y = (r.top - w.top) / z + 9;
      }
      const many = st.tracksSel.size > 1;
      place(win, st.menu.kind === "track"
        ? projectContextMenu({ many, tauri: st.tauri, inCollection: true, hot: st.menu.hot })
        : collectionContextMenu({ hot: st.menu.hot }), { x, y });
    }
    if (st.hot) {
      const cell = el.querySelector(`tr[data-id="${st.hot.id}"] td[data-hover="${st.hot.kind}"]`);
      if (cell) place(win, countHoverList(rows.find((p) => p.id === st.hot.id), st.hot.kind), { anchor: cell, align: "right" });
    }
    const anchor = (act) => el.querySelector(`.viewbar [data-act="${act}"]`);
    if (st.popover === "sort") place(win, collectionSortMenu(st.sort), { anchor: anchor("sort") });
    if (st.popover === "columns") place(win, columnChooser(st.columns), { anchor: anchor("columns") });
    if (st.renaming) {
      const input = el.querySelector(".card .namecell input");
      if (input && st.live) { input.focus(); input.select(); }
    }
  }

  const tracksNow = () => st.order || colTracks(st.open, st.scope).map((p) => p.id);

  function openCollection(id) {
    Object.assign(st, { open: id, selected: id, trackSort: null, order: null, tracksSel: new Set(), tasksSel: new Set(), menu: null, popover: null, hot: null });
  }

  /** Dragging a row by its handle: the row it would land above follows the pointer, and
   *  letting go moves it there (PUT /collections/:id/reorder with the new order). */
  function startDrag(el, id, e) {
    e.preventDefault();
    st.drag = { id, to: null };
    const move = (ev) => {
      const tr = document.elementFromPoint(ev.clientX, ev.clientY)?.closest(".content tr[data-id]");
      const tbody = el.querySelector(".content tbody");
      let to = st.drag.to;
      if (tr) {
        const r = tr.getBoundingClientRect();
        const after = ev.clientY > r.top + r.height / 2;
        const next = after ? tr.nextElementSibling : tr;
        to = next && next.dataset.id ? next.dataset.id : "end";
      } else if (tbody && ev.clientY > tbody.getBoundingClientRect().bottom) to = "end";
      if (to !== st.drag.to) { st.drag.to = to; draw(el); }
    };
    const up = () => {
      document.removeEventListener("pointermove", move);
      document.removeEventListener("pointerup", up);
      const { id: moving, to } = st.drag;
      st.drag = null;
      if (to && to !== moving) {
        const ids = tracksNow().filter((x) => x !== moving);
        const at = to === "end" ? ids.length : ids.indexOf(to);
        ids.splice(at, 0, moving);
        st.order = ids;
      }
      draw(el);
    };
    document.addEventListener("pointermove", move);
    document.addEventListener("pointerup", up);
    draw(el);
  }

  function wire(el) {
    const redraw = () => draw(el);
    const closeAll = () => { st.menu = null; st.popover = null; st.hot = null; };

    el.addEventListener("pointerdown", (e) => {
      const handle = e.target.closest(".content td.pos .handle");
      if (handle) startDrag(el, handle.closest("tr").dataset.id, e);
    });

    el.addEventListener("contextmenu", (e) => {
      const track = st.open && e.target.closest(".content tr[data-id]");
      const card = !st.open && e.target.closest(".content [data-card]");
      if (!track && !card) return;
      e.preventDefault();
      const win = el.querySelector(".win"), w = win.getBoundingClientRect(), z = w.width / win.offsetWidth;
      closeAll();
      if (track) {
        if (!st.tracksSel.has(track.dataset.id)) st.tracksSel = new Set([track.dataset.id]);
        st.menu = { id: track.dataset.id, kind: "track" };
      } else {
        st.selected = card.dataset.card;
        st.menu = { id: card.dataset.card, kind: "card" };
      }
      Object.assign(st.menu, { x: (e.clientX - w.left) / z, y: (e.clientY - w.top) / z });
      redraw();
    });

    el.addEventListener("click", (e) => {
      st.live = true;
      const hit = e.target.closest("[data-act]");
      const act = hit?.dataset.act;
      const th = e.target.closest("th[data-sort]");
      const card = e.target.closest(".content [data-card]");
      const row = e.target.closest(".content tr[data-id]");
      const task = e.target.closest("li[data-task]");
      const inPop = e.target.closest(".pop");

      if (act === "open") { openCollection(hit.dataset.id || st.menu?.id || st.selected); return redraw(); }
      if (act === "back") { Object.assign(st, { open: null, tracksSel: new Set(), tasksSel: new Set(), trackSort: null, order: null }); closeAll(); return redraw(); }
      if (act === "layout-grid" || act === "layout-details") { st.layout = act.slice(7); closeAll(); return redraw(); }
      if (act === "sort" || act === "columns") { st.popover = st.popover === act ? null : act; st.menu = null; return redraw(); }
      if (act === "set-sort") { const s = COLLECTION_SORTS[+hit.dataset.v]; st.sort = { col: s.col, desc: s.desc }; closeAll(); return redraw(); }
      if (act === "new" || act === "edit" || act === "duplicate" || act === "delete") { closeAll(); st.dialog = act; return redraw(); }
      if (act === "dismiss") { st.dialog = null; return redraw(); }
      if (act === "rename") { st.renaming = st.menu.id; closeAll(); return redraw(); }
      if (act === "removetracks") {
        st.order = tracksNow().filter((id) => !st.tracksSel.has(id));
        st.tracksSel = new Set(); closeAll(); return redraw();
      }
      if (act === "clear") { st.tracksSel = new Set(); return redraw(); }
      if (act === "clearsearch") { st.query = ""; st.selected = null; return redraw(); }
      if (act === "sidebar") { st.sidebarCollapsed = !st.sidebarCollapsed; return redraw(); }
      if (act === "inspector") { st.inspectorOpen = !st.inspectorOpen; return redraw(); }
      if (act === "selectall") {
        const rows = trackRows(st);
        st.tracksSel = new Set(rows.every((p) => st.tracksSel.has(p.id)) ? [] : rows.map((p) => p.id));
        return redraw();
      }
      if (act === "rowcheck") {
        const id = row.dataset.id;
        st.tracksSel.has(id) ? st.tracksSel.delete(id) : st.tracksSel.add(id);
        st.anchor = id; closeAll(); return redraw();
      }
      if (act === "close") { closeAll(); return redraw(); }
      if (inPop || e.target.closest(".scrim")) return;

      if (task) {
        const id = task.dataset.task;
        st.tasksSel.has(id) ? st.tasksSel.delete(id) : st.tasksSel.add(id);
        return redraw();
      }
      if (th) {
        const col = th.dataset.sort;
        if (st.open) {
          const cur = st.trackSort || { col: "position", desc: false };
          st.trackSort = col === "position" ? null : cur.col === col ? { col, desc: !cur.desc } : { col, desc: false };
        } else {
          st.sort = st.sort.col === col ? { col, desc: !st.sort.desc } : { col, desc: col !== "name" };
        }
        closeAll(); return redraw();
      }
      if (st.open && row && !e.target.closest("input")) {
        const id = row.dataset.id;
        if (e.ctrlKey || e.metaKey) st.tracksSel.has(id) ? st.tracksSel.delete(id) : st.tracksSel.add(id);
        else if (e.shiftKey && st.anchor) {
          const ids = trackRows(st).map((p) => p.id);
          const [a, b] = [ids.indexOf(st.anchor), ids.indexOf(id)].sort((x, y) => x - y);
          st.tracksSel = new Set(ids.slice(a, b + 1));
        } else st.tracksSel = new Set([id]);
        if (!e.shiftKey) st.anchor = id;
        closeAll(); return redraw();
      }
      // A double-click opens. The first click redraws, so the second lands on a new
      // element and no dblclick fires; the click's own count survives the redraw.
      if (!st.open && card && !e.target.closest("input")) {
        if (e.detail >= 2) openCollection(card.dataset.card);
        else { st.selected = card.dataset.card; st.tasksSel = new Set(); closeAll(); }
        return redraw();
      }
      if (st.menu || st.popover) { closeAll(); redraw(); }
    });

    el.addEventListener("mouseover", (e) => {
      if (!st.open || st.drag) return;
      const cell = e.target.closest("td[data-hover]");
      if (cell) {
        const id = cell.parentElement.dataset.id;
        if (!st.hot || st.hot.id !== id || st.hot.kind !== cell.dataset.hover) { st.hot = { id, kind: cell.dataset.hover }; redraw(); }
      } else if (st.hot && !e.target.closest(".hoverlist")) { st.hot = null; redraw(); }
    });

    el.addEventListener("keydown", (e) => {
      if (e.target.closest(".namecell input") && (e.key === "Enter" || e.key === "Escape")) { st.renaming = null; redraw(); }
    });
  }

  return (el) => {
    if (!el.dataset.wired) { el.dataset.wired = "1"; wire(el); }
    draw(el);
  };
}
