// All of the screen's state, split by the speed it changes at (ADR-0047), as the Solid
// app does:
//
//   click-speed  `ui`: a $state object; each property is its own signal underneath.
//   push-speed   `push`: its own $state object, read only by the status bar.
//
// A .svelte.ts module is compiled for runes. `$state` and `$derived` cannot be exported
// as reassignable bindings, so derived values go out through getters on `view`.

import { Api, DEFAULT_URL, simulatedScan } from "../../shared/api";
import {
  DEFAULT_COLUMNS, DEFAULT_SORT, clickRow, collectionsOf, lastPage, pageOf, sortRows, toggleAll, toggleRow,
  type Selection,
} from "../../shared/projects";
import { revealInExplorer } from "../../shared/os";
import type { Collection, KeySpelling, Project, ScanProgress, Scope, Sort, SystemInfo } from "../../shared/types";

export const api = new Api(import.meta.env.VITE_SEULA_URL ?? DEFAULT_URL);

// ---------------------------------------------------------------- click-speed state

export const ui = $state({
  scope: "active" as Scope,
  query: "",
  sort: DEFAULT_SORT as Sort | null,   // null while a search is showing: by relevance
  page: 0,
  columns: [...DEFAULT_COLUMNS],
  inspectorOpen: true,
  sidebarCollapsed: false,
  spelling: "sharp" as KeySpelling,
  popover: null as null | "columns",
  renaming: null as string | null,
  menu: null as null | { id: string; x: number; y: number },
  hot: null as null | { id: string; kind: "plugins" | "samples" },
});

const sel = $state<Selection>({ selected: new Set(), anchor: null });
const setSelection = (next: Selection) => { sel.selected = next.selected; sel.anchor = next.anchor; };
export const isSelected = (id: string) => sel.selected.has(id);

// ---------------------------------------------------------------- data

export const data = $state({
  projects: [] as Project[],
  collections: [] as Collection[],
  system: null as SystemInfo | null,
  /** The active project count, kept while the list shows archived ones or a search. */
  activeTotal: null as number | null,
  loadError: null as string | null,
  notice: null as string | null,
});

const collectionById = $derived(new Map(data.collections.map((c) => [c.id, c])));
const projectById = $derived(new Map(data.projects.map((p) => [p.id, p])));
export const collectionsFor = (p: Project) => collectionsOf(p, collectionById);
export const collectionMap = () => collectionById;

let loadToken = 0;
export async function load() {
  const token = ++loadToken;
  try {
    const list = ui.query ? await api.search(ui.query) : await api.projects(ui.scope);
    if (token !== loadToken) return;   // a newer load has started
    data.projects = list;
    if (!ui.query && ui.scope === "active") data.activeTotal = list.length;
    data.loadError = null;
  } catch (e) {
    if (token === loadToken) data.loadError = String(e);
  }
}

export async function loadSidecars() {
  const [c, s] = await Promise.allSettled([api.collections(), api.systemInfo()]);
  if (c.status === "fulfilled") data.collections = c.value;
  if (s.status === "fulfilled") data.system = s.value;
}

// ---------------------------------------------------------------- derived

const sorted = $derived(sortRows(data.projects, ui.sort, ui.spelling));
const pageRows = $derived(pageOf(sorted, ui.page));
const selectedProjects = $derived(
  [...sel.selected].map((id) => projectById.get(id)).filter((p): p is Project => !!p));

export const view = {
  get sorted() { return sorted; },
  get total() { return sorted.length; },
  get pageRows() { return pageRows; },
  get pageIds() { return pageRows.map((p) => p.id); },
  get pageCount() { return lastPage(sorted.length) + 1; },
  get selectedProjects() { return selectedProjects; },
  get selectedCount() { return sel.selected.size; },
};

// ---------------------------------------------------------------- actions

export const clearSelection = () => setSelection({ selected: new Set(), anchor: null });

function resetView() {
  clearSelection();
  ui.page = 0; ui.menu = null; ui.hot = null; ui.popover = null; ui.renaming = null;
}

export function setScope(scope: Scope) { ui.scope = scope; resetView(); }

export function setQuery(query: string) {
  if (query === ui.query) return;
  ui.query = query;
  ui.sort = query ? null : DEFAULT_SORT;
  resetView();
}

export function sortBy(next: Sort) { ui.sort = next; ui.page = 0; }

export function goPage(delta: number) { ui.page += delta; ui.menu = null; ui.hot = null; }

export function rowClick(id: string, mods: { ctrl: boolean; shift: boolean }) {
  setSelection(clickRow(sel, id, view.pageIds, mods));
  ui.menu = null; ui.popover = null;
}
export const rowCheck = (id: string) => setSelection(toggleRow(sel, id));
export const checkAll = () => setSelection(toggleAll(sel, view.pageIds));

export function selectOnly(id: string) {
  if (!isSelected(id)) setSelection({ selected: new Set([id]), anchor: id });
}

export function toggleColumn(id: string) {
  ui.columns = ui.columns.includes(id) ? ui.columns.filter((c) => c !== id) : [...ui.columns, id];
}

// ---------------------------------------------------------------- edits

let notesTimer: ReturnType<typeof setTimeout> | undefined;

/** Notes show what was typed at once; the PUT follows when typing pauses. */
export function editNotes(id: string, notes: string) {
  const p = projectById.get(id);
  if (p) p.notes = notes;
  clearTimeout(notesTimer);
  notesTimer = setTimeout(() => api.setNotes(id, notes).catch((e) => { data.notice = String(e); }), 500);
}

export async function renameProject(id: string, name: string) {
  const p = projectById.get(id);
  ui.renaming = null;
  if (!p || !name.trim() || name === p.name) return;
  const before = p.name;
  p.name = name;
  try { await api.setName(id, name); }
  catch (e) { p.name = before; data.notice = String(e); }
}

export async function showInExplorer(path: string) {
  try { await revealInExplorer(path); data.notice = null; }
  catch (e) { data.notice = e instanceof Error ? e.message : String(e); }
}

// ---------------------------------------------------------------- push-speed state

export const push = $state({ scan: null as ScanProgress | null });
let scanAbort: AbortController | null = null;

/** Follow a scan to its end, writing each event to `push.scan` and nothing else. */
export async function runScan(simulate: boolean) {
  if (scanAbort) return;
  scanAbort = new AbortController();
  const { signal } = scanAbort;
  try {
    const events = simulate ? simulatedScan(600, 40, signal) : api.scan(signal);
    for await (const ev of events) push.scan = ev;
    await load();
  } catch (e) {
    if (!signal.aborted) data.notice = String(e);
  } finally {
    scanAbort = null;
    setTimeout(() => { if (!scanAbort) push.scan = null; }, 1500);
  }
}
