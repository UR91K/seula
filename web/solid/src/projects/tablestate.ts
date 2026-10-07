// The state of one projects table (ADR-0058): the rows, their sort and page, the columns
// and their widths (ADR-0066), the selection, the name being edited and the hover list.
// The projects view makes one at
// module level and so does the collections view's tracklist, so each keeps its own sort
// and selection and neither is lost when its view unmounts (ADR-0055).
//
// The rows are a store reconciled by id, so an edit to one project's notes touches only
// the nodes that show that project's notes.

import { batch, createMemo, createSignal } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import {
  DEFAULT_COLUMNS, clickRow, lastPage, pageOf, sortRows, toggleAll, toggleRow, type Selection,
} from "../../../shared/projects";
import type { Project, Sort } from "../../../shared/types";
import { createWidths } from "../shell/columns";
import { api, pageSize, setNotice, shell } from "../shell/shell";

export type HotList = { id: string; kind: "plugins" | "samples" };

export function createTableState(opts: { sort: Sort | null; paged: boolean }) {
  const [items, setItems] = createStore<Project[]>([]);
  const [sort, setSort] = createSignal<Sort | null>(opts.sort);
  const [page, setPage] = createSignal(0);
  const [columns, setColumns] = createSignal(DEFAULT_COLUMNS);
  const widths = createWidths();
  const [renaming, setRenaming] = createSignal<string | null>(null);
  const [hot, setHot] = createSignal<HotList | null>(null);
  const [selection, setSelection] = createSignal<Selection>({ selected: new Set(), anchor: null });

  // ------------------------------------------------------------ derived

  /** Every row, in the sort's order; with no sort, in the order the list arrived in. */
  const sorted = createMemo(() => sortRows(items, sort(), shell.spelling));
  const total = () => sorted().length;
  const rows = createMemo(() => (opts.paged ? pageOf(sorted(), page(), pageSize()) : sorted()));
  const rowIds = createMemo(() => rows().map((p) => p.id));
  const pageCount = () => (opts.paged ? lastPage(total(), pageSize()) + 1 : 1);

  const byId = createMemo(() => new Map(items.map((p) => [p.id, p])));
  const find = (id: string) => byId().get(id);

  const isSelected = (id: string) => selection().selected.has(id);
  const selectedCount = () => selection().selected.size;
  const selectedProjects = createMemo(() =>
    [...selection().selected].map((id) => byId().get(id)).filter((p): p is Project => !!p));

  // ------------------------------------------------------------ actions

  const replace = (list: Project[]) => setItems(reconcile(list, { key: "id" }));

  const clearSelection = () => setSelection({ selected: new Set(), anchor: null });
  const rowClick = (id: string, mods: { ctrl: boolean; shift: boolean }) => setSelection((cur) => clickRow(cur, id, rowIds(), mods));
  const rowCheck = (id: string) => setSelection((cur) => toggleRow(cur, id));
  const checkAll = () => setSelection((cur) => toggleAll(cur, rowIds()));
  const selectOnly = (id: string) => { if (!isSelected(id)) setSelection({ selected: new Set([id]), anchor: id }); };
  /** Keep only the ids still listed: a reload may have removed what was selected. */
  const pruneSelection = () => setSelection((cur) => {
    const kept = [...cur.selected].filter((id) => byId().has(id));
    return kept.length === cur.selected.size ? cur : { selected: new Set(kept), anchor: cur.anchor };
  });

  /** A new sort starts from the first page. */
  const sortBy = (next: Sort | null) => batch(() => { setSort(next); setPage(0); });
  const goPage = (delta: number) => batch(() => { setPage((p) => p + delta); setHot(null); });
  /** What changes the list: back to the first page with nothing selected or being edited. */
  const reset = () => batch(() => { clearSelection(); setPage(0); setHot(null); setRenaming(null); });

  const toggleColumn = (id: string) =>
    setColumns((cols) => (cols.includes(id) ? cols.filter((c) => c !== id) : [...cols, id]));

  // ------------------------------------------------------------ hover list

  let hideTimer: ReturnType<typeof setTimeout> | undefined;
  const holdHover = () => clearTimeout(hideTimer);
  const releaseHover = () => { clearTimeout(hideTimer); hideTimer = setTimeout(() => setHot(null), 150); };

  // ------------------------------------------------------------ edits

  let notesTimer: ReturnType<typeof setTimeout> | undefined;

  /** Notes show what was typed at once; the PUT follows when typing pauses. */
  function editNotes(id: string, notes: string) {
    setItems((p) => p.id === id, "notes", notes);
    clearTimeout(notesTimer);
    notesTimer = setTimeout(() => api.setNotes(id, notes).catch((e) => setNotice(String(e))), 500);
  }

  async function renameProject(id: string, name: string) {
    const p = find(id);
    setRenaming(null);
    if (!p || !name.trim() || name === p.name) return;
    const before = p.name;
    setItems((x) => x.id === id, "name", name);
    try { await api.setName(id, name); }
    catch (e) { setItems((x) => x.id === id, "name", before); setNotice(String(e)); }
  }

  return {
    items, setItems, replace, find,
    sort, sortBy, page, goPage, firstPage: () => setPage(0), pageSize, pageCount, total, sorted, rows, rowIds,
    columns, toggleColumn, widths,
    renaming, setRenaming, renameProject, editNotes,
    hot, setHot, holdHover, releaseHover,
    isSelected, selectedCount, selectedProjects, clearSelection, rowClick, rowCheck, checkAll, selectOnly, pruneSelection,
    reset,
  };
}

/** One projects table's state; what the table, the inspector and the popovers are given. */
export type TableState = ReturnType<typeof createTableState>;
