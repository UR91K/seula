// The projects view's state. Click-speed state is `ui`, a store: each property is
// tracked on its own, so a component that reads only the sort is not asked to re-run when
// the scope changes. The window's state and the scan's are in ../shell/shell.ts.
//
// The project list itself is a store too, reconciled by id, so an edit to one project's
// notes touches only the nodes that show that project's notes.

import { batch, createMemo, createSignal } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import {
  DEFAULT_COLUMNS, DEFAULT_SORT, clickRow, collectionsOf, lastPage, pageOf, sortRows, toggleAll, toggleRow,
  type Selection,
} from "../../../shared/projects";
import type { Collection, Project, Scope, Sort } from "../../../shared/types";
import { api, closePopups, setCounts, setNotice, setShell, shell } from "../shell/shell";

// ---------------------------------------------------------------- click-speed state

export const [ui, setUi] = createStore({
  scope: "active" as Scope,
  query: "",
  sort: DEFAULT_SORT as Sort | null,   // null while a search is showing: by relevance
  page: 0,
  columns: DEFAULT_COLUMNS,
  renaming: null as string | null,
  hot: null as null | { id: string; kind: "plugins" | "samples" },
});

const [selection, setSelection] = createSignal<Selection>({ selected: new Set(), anchor: null });
export const isSelected = (id: string) => selection().selected.has(id);
export const selectedCount = () => selection().selected.size;

// ---------------------------------------------------------------- data

export const [projects, setProjects] = createStore<Project[]>([]);
export const [loadError, setLoadError] = createSignal<string | null>(null);
export const [collections, setCollections] = createSignal<Collection[]>([]);

export const collectionById = createMemo(() => new Map(collections().map((c) => [c.id, c])));
const projectById = createMemo(() => new Map(projects.map((p) => [p.id, p])));
export const collectionsFor = (p: Project) => collectionsOf(p, collectionById());

let loadToken = 0;
export async function load() {
  const token = ++loadToken;
  try {
    const list = ui.query ? await api.search(ui.query) : await api.projects(ui.scope);
    if (token !== loadToken) return;   // a newer load has started
    batch(() => {
      setProjects(reconcile(list, { key: "id" }));
      if (!ui.query && ui.scope === "active") setCounts("projects", list.length);
      setLoadError(null);
    });
  } catch (e) {
    if (token === loadToken) setLoadError(String(e));
  }
}

export async function loadCollections() {
  try { setCollections(await api.collections()); } catch { /* the inspector shows none */ }
}

// ---------------------------------------------------------------- derived

export const sorted = createMemo(() => sortRows(projects, ui.sort, shell.spelling));
export const total = () => sorted().length;
export const pageRows = createMemo(() => pageOf(sorted(), ui.page));
export const pageIds = createMemo(() => pageRows().map((p) => p.id));
export const pageCount = () => lastPage(total()) + 1;

export const selectedProjects = createMemo(() =>
  [...selection().selected].map((id) => projectById().get(id)).filter((p): p is Project => !!p));

// ---------------------------------------------------------------- actions

export function clearSelection() { setSelection({ selected: new Set(), anchor: null }); }

function resetView() {
  batch(() => { clearSelection(); setUi({ page: 0, hot: null, renaming: null }); closePopups(); });
}

export function setScope(scope: Scope) {
  batch(() => { setUi("scope", scope); resetView(); });
}

export function setQuery(query: string) {
  if (query === ui.query) return;
  batch(() => {
    setUi({ query, sort: query ? null : DEFAULT_SORT });
    resetView();
  });
}

export function sortBy(next: Sort) { batch(() => { setUi({ sort: next, page: 0 }); }); }

export function goPage(delta: number) {
  batch(() => { setUi("page", (p) => p + delta); setUi("hot", null); setShell("menu", null); });
}

export function rowClick(id: string, mods: { ctrl: boolean; shift: boolean }) {
  batch(() => { setSelection((cur) => clickRow(cur, id, pageIds(), mods)); closePopups(); });
}
export function rowCheck(id: string) { setSelection((cur) => toggleRow(cur, id)); }
export function checkAll() { setSelection((cur) => toggleAll(cur, pageIds())); }

export function selectOnly(id: string) {
  if (!isSelected(id)) setSelection({ selected: new Set([id]), anchor: id });
}

export function toggleColumn(id: string) {
  setUi("columns", (cols) => cols.includes(id) ? cols.filter((c) => c !== id) : [...cols, id]);
}

// ---------------------------------------------------------------- edits

let notesTimer: ReturnType<typeof setTimeout> | undefined;

/** Notes show what was typed at once; the PUT follows when typing pauses. */
export function editNotes(id: string, notes: string) {
  setProjects((p) => p.id === id, "notes", notes);
  clearTimeout(notesTimer);
  notesTimer = setTimeout(() => api.setNotes(id, notes).catch((e) => setNotice(String(e))), 500);
}

export async function renameProject(id: string, name: string) {
  const p = projectById().get(id);
  setUi("renaming", null);
  if (!p || !name.trim() || name === p.name) return;
  const before = p.name;
  setProjects((x) => x.id === id, "name", name);
  try { await api.setName(id, name); }
  catch (e) { setProjects((x) => x.id === id, "name", before); setNotice(String(e)); }
}
