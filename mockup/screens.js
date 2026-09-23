// Live screens (ADR-0036): a view's full window, driven by a state object and wired
// for clicks, built only from the renderers in components.js. A board frame gets one
// with `render: projectsScreen({...start state})`.
//
// Popovers (menus, pickers, hover lists) are placed after the window renders, against
// the element they belong to. Positions are measured, so they are unzoomed px set at
// runtime -- geometry, not a stylesheet measure.

function projectsState(overrides = {}) {
  return {
    view: "projects", tauri: true, inspectorOpen: true, sidebarCollapsed: false,
    scope: "active", query: "", sort: { col: "modified", desc: true }, page: 0, pageSize: 100,
    selected: new Set(), tasksSel: new Set(),
    columns: PROJECT_COLUMNS.filter((c) => !c.hidden).map((c) => c.id),
    menu: null,      // { id, x?, y?, sub? }   row context menu
    hot: null,       // { id, kind }           count-cell hover list
    popover: null,   // "columns" | "menu-tags" | "menu-collection" | "picker-*"
    dialog: null,    // "tags" | "delete" | "newcollection"
    dialogMode: null,
    renaming: null,
    ...overrides,
  };
}

/** Place a popover inside the window, below (or beside) an anchor, kept on screen. */
function place(win, html, { anchor = null, align = "left", beside = false, x = null, y = null } = {}) {
  const z = win.getBoundingClientRect().width / win.offsetWidth || 1;
  const w = win.getBoundingClientRect();
  const holder = document.createElement("div");
  holder.innerHTML = html.trim();
  const pop = holder.firstElementChild;
  win.appendChild(pop);
  let left = x, top = y;
  if (anchor) {
    const a = anchor.getBoundingClientRect();
    if (beside) { left = (a.right - w.left) / z; top = (a.top - w.top) / z - 3; }
    else {
      left = ((align === "right" ? a.right : a.left) - w.left) / z - (align === "right" ? pop.offsetWidth : 0);
      top = (a.bottom - w.top) / z;
    }
  }
  pop.style.left = `${Math.max(0, Math.min(left, win.offsetWidth - pop.offsetWidth - 2))}px`;
  pop.style.top = `${Math.max(0, Math.min(top, win.offsetHeight - pop.offsetHeight - 2))}px`;
  return pop;
}

function projectsScreen(st) {
  const selected = () => [...st.selected].map((id) => projectById.get(id)).filter(Boolean);

  function draw(el) {
    const rows = projectRows(st);
    const sel = selected();
    const box = el.querySelector(".content");
    const scroll = box ? [box.scrollTop, box.scrollLeft] : null;

    const modal = st.dialog === "tags" ? tagManager(st.dialogMode)
      : st.dialog === "delete" ? deletePermanentDialog(sel)
      : st.dialog === "newcollection" ? newCollectionDialog(sel) : "";
    const count = st.query ? `${rows.length} ${rows.length === 1 ? "result" : "results"}`
      : `${rows.length} ${st.scope === "archived" ? "archived" : "projects"}`;

    el.innerHTML = renderShell({
      ...st,
      viewbar: projectsViewbar(st, rows.length),
      content: rows.length ? projectTable(st, rows) : projectsEmpty(st),
      inspector: sel.length === 1 ? projectInspector(sel[0], { tasksSel: st.tasksSel })
        : sel.length > 1 ? multiInspector(sel) : inspectorEmpty(),
      status: [count, ...(sel.length ? [`${sel.length} selected`] : [])],
      overlay: modal ? `<div class="scrim">${modal}</div>` : "",
    });

    const win = el.querySelector(".win");
    const content = el.querySelector(".content");
    if (scroll) [content.scrollTop, content.scrollLeft] = scroll;
    else if (st.selected.size) {
      // First draw: bring the first selected row into view, a third of the way down.
      const first = el.querySelector("tr.sel");
      if (first) content.scrollTop = first.offsetTop - content.clientHeight / 3;
    }

    // Anchored popovers, in stacking order.
    if (st.menu) {
      const row = el.querySelector(`tr[data-id="${st.menu.id}"]`);
      const ctx = projectContextMenu({ archived: st.scope === "archived", many: sel.length > 1, tauri: st.tauri, hot: st.menu.sub ? "tags" : null });
      let { x, y } = st.menu;
      if (x == null && row) {
        // Fixed frames: open where a right-click on the name would.
        const r = row.children[1].getBoundingClientRect(), w = win.getBoundingClientRect(), z = w.width / win.offsetWidth;
        x = (r.left - w.left) / z + 40; y = (r.top - w.top) / z + 9;
      }
      const m = place(win, ctx, { x, y });
      if (st.menu.sub === "tags") place(win, tagSubmenu(sel.length ? sel : [projectById.get(st.menu.id)]), { anchor: m.querySelector('[data-act="sub-tags"]'), beside: true });
    }
    if (st.hot) {
      const row = el.querySelector(`tr[data-id="${st.hot.id}"]`);
      const cell = row && row.querySelector(`td[data-hover="${st.hot.kind}"]`);
      if (cell) place(win, countHoverList(projectById.get(st.hot.id) || rows.find((p) => p.id === st.hot.id), st.hot.kind), { anchor: cell, align: "right" });
    }
    const popAnchor = (act) => el.querySelector(`.viewbar [data-act="${act}"]`);
    if (st.popover === "columns") place(win, columnChooser(st.columns), { anchor: popAnchor("columns") });
    if (st.popover === "menu-tags" || st.popover === "picker-tag" || st.popover === "picker-untag") {
      const html = st.popover === "menu-tags" ? batchTagsMenu() : st.popover === "picker-tag" ? tagPicker(st.pickerQuery || "") : untagPicker(sel);
      place(win, html, { anchor: popAnchor("menu-tags") });
    }
    if (st.popover === "menu-collection" || st.popover === "picker-collection" || st.popover === "picker-uncollection") {
      const html = st.popover === "menu-collection" ? batchCollectionMenu() : st.popover === "picker-collection" ? collectionPicker(st.pickerQuery || "") : uncollectionPicker(sel);
      place(win, html, { anchor: popAnchor("menu-collection") });
    }
    if (st.renaming) {
      const input = el.querySelector(".namecell input");
      if (input && st.live) { input.focus(); input.select(); }
    }
  }

  function wire(el) {
    const redraw = () => draw(el);
    const closeAll = () => { st.menu = null; st.popover = null; st.hot = null; };

    el.addEventListener("contextmenu", (e) => {
      const row = e.target.closest("tr[data-id]");
      if (!row) return;
      e.preventDefault();
      if (!st.selected.has(row.dataset.id)) st.selected = new Set([row.dataset.id]);
      const win = el.querySelector(".win"), w = win.getBoundingClientRect(), z = w.width / win.offsetWidth;
      closeAll();
      st.menu = { id: row.dataset.id, x: (e.clientX - w.left) / z, y: (e.clientY - w.top) / z };
      redraw();
    });

    el.addEventListener("click", (e) => {
      st.live = true;  // from here on, focus follows the user (e.g. into a rename field)
      const act = e.target.closest("[data-act]")?.dataset.act;
      const th = e.target.closest("th[data-sort]");
      const row = e.target.closest("tr[data-id]");
      const task = e.target.closest("li[data-task]");
      const inPop = e.target.closest(".pop");

      if (act === "sub-tags") { st.menu.sub = "tags"; return redraw(); }
      if (act === "rename") { st.renaming = st.menu.id; closeAll(); return redraw(); }
      if (act === "managetags") { closeAll(); st.dialog = "tags"; st.dialogMode = null; return redraw(); }
      if (act === "delete") { closeAll(); st.dialog = "delete"; return redraw(); }
      if (act === "newcollection") { closeAll(); st.dialog = "newcollection"; return redraw(); }
      if (act === "dismiss") { st.dialog = null; return redraw(); }
      if (act && act.startsWith("picker-")) { st.popover = act; st.pickerQuery = ""; return redraw(); }
      if (act === "menu-tags" || act === "menu-collection" || act === "columns") {
        st.popover = st.popover === act ? null : act; st.menu = null; return redraw();
      }
      if (act === "sidebar") { st.sidebarCollapsed = !st.sidebarCollapsed; return redraw(); }
      if (act === "inspector") { st.inspectorOpen = !st.inspectorOpen; return redraw(); }
      if (act === "prev" || act === "next") { st.page += act === "next" ? 1 : -1; el.querySelector(".content").scrollTop = 0; return redraw(); }
      if (act === "clear" || act === "reactivate" || act === "archive") { st.selected = new Set(); closeAll(); return redraw(); }
      if (act === "clearsearch") { st.query = ""; st.sort = { col: "modified", desc: true }; st.page = 0; return redraw(); }
      if (act === "scope") { st.scope = st.scope === "archived" ? "active" : "archived"; st.selected = new Set(); st.page = 0; return redraw(); }
      if (act === "selectall") {
        const page = pageOf(st, projectRows(st));
        const all = page.every((p) => st.selected.has(p.id));
        st.selected = new Set(all ? [] : page.map((p) => p.id));
        return redraw();
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
        st.sort = st.sort && st.sort.col === col ? { col, desc: !st.sort.desc } : { col, desc: col === "modified" || col === "created" };
        return redraw();
      }
      if (row && !e.target.closest("input")) {
        const id = row.dataset.id;
        if (e.ctrlKey || e.metaKey) st.selected.has(id) ? st.selected.delete(id) : st.selected.add(id);
        else if (e.shiftKey && st.anchor) {
          const page = pageOf(st, projectRows(st)).map((p) => p.id);
          const [a, b] = [page.indexOf(st.anchor), page.indexOf(id)].sort((x, y) => x - y);
          st.selected = new Set(page.slice(a, b + 1));
        } else st.selected = new Set([id]);
        if (!e.shiftKey) st.anchor = id;
        st.tasksSel = new Set();
        if (e.target.closest(".edit")) st.renaming = id;
        closeAll();
        return redraw();
      }
      if (st.menu || st.popover) { closeAll(); redraw(); }
    });

    el.addEventListener("mouseover", (e) => {
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
