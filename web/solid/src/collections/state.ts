// The collections view's state. Click-speed state is `cui`, a store; the list is a store
// reconciled by id, so renaming one collection touches only its own card. The window's state
// and the scan's are in ../shell/shell.ts.
//
// The server sorts this list (ADR-0044), so a change of sort or search reloads it; the layout
// is a module-level store and is not persisted yet (ADR-0062).

import { batch, createMemo, createResource, createSignal, untrack } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import {
  DEFAULT_COLLECTION_SORT, nextCollectionSort, type CollectionLayout, type CollectionSort,
} from "../../../shared/collections";
import type { CollectionRow, CollectionSortKey, CollectionStats, CollectionTask, Project } from "../../../shared/types";
import { createTableState } from "../projects/tablestate";
import { api, closePopups, loadCollectionNames, setCounts, setNotice, setShell, showProjectsMatching } from "../shell/shell";

export type DialogKind = "new" | "edit" | "duplicate" | "delete";

export const [cui, setCui] = createStore({
  layout: "grid" as CollectionLayout,
  sort: DEFAULT_COLLECTION_SORT as CollectionSort,
  query: "",
  selected: null as string | null,
  renaming: null as string | null,
  dialog: null as DialogKind | null,
  /** The opened collection (ADR-0044): view state, not a route. */
  open: null as string | null,
});

/** The opened collection's tracklist: the projects table over its projects. No sort means
 *  collection order, the only one that can be dragged (ADR-0060). */
export const tracks = createTableState({ sort: null, paged: false });

// ---------------------------------------------------------------- data

export const [collections, setCollections] = createStore<CollectionRow[]>([]);
export const [loadError, setLoadError] = createSignal<string | null>(null);
export const [loaded, setLoaded] = createSignal(false);

let loadToken = 0;
export async function load() {
  const token = ++loadToken;
  try {
    const list = cui.query ? await api.searchCollections(cui.query) : await api.collectionList(cui.sort.col, cui.sort.desc);
    if (token !== loadToken) return;   // a newer load has started
    batch(() => {
      setCollections(reconcile(list, { key: "id" }));
      if (!cui.query) setCounts("collections", list.length);
      setLoadError(null); setLoaded(true);
    });
  } catch (e) {
    if (token === loadToken) setLoadError(String(e));
  }
}

// ---------------------------------------------------------------- derived

export const selectedCollection = createMemo(() => collections.find((c) => c.id === cui.selected));
export const collectionById = (id: string) => collections.find((c) => c.id === id);

/** What the inspector shows of the selected collection. `id` says which collection they
 *  belong to, so a stale answer is never shown against the next selection. */
export interface Detail { id: string; stats: CollectionStats; tasks: CollectionTask[]; totalTasks: number; tracks: Project[] }
export const [detail, { refetch: refetchDetail }] = createResource(
  () => cui.open ?? cui.selected,
  async (id): Promise<Detail> => {
    const [stats, tasks, list] = await Promise.all([
      api.collectionStats(id), api.collectionTasks(id), api.collectionProjects(id),
    ]);
    // An opened collection's list is the tracklist's rows too.
    if (id === untrack(() => cui.open)) batch(() => { tracks.replace(list); tracks.pruneSelection(); });
    return { id, stats, tasks: tasks.tasks, totalTasks: tasks.total_tasks, tracks: list };
  },
);

// ---------------------------------------------------------------- actions

export const select = (id: string | null) => batch(() => { setCui("selected", id); closePopups(); });

/** Replace the grid with the collection's tracklist. */
export function openCollection(id: string) {
  batch(() => {
    tracks.replace([]);
    tracks.reset();
    tracks.sortBy(null);
    setCui({ open: id, selected: id, renaming: null });
    closePopups();
  });
  refetchDetail();   // the source may not have changed (the card was already selected)
}

export function closeCollection() {
  batch(() => { setCui("open", null); tracks.reset(); closePopups(); });
}

/** Take the selected projects out of the opened collection. They stay in Seula. */
export async function removeSelected() {
  const id = cui.open;
  const ids = tracks.selectedProjects().map((p) => p.id);
  if (!id || !ids.length) return;
  await change(() => api.removeFromCollection(id, ids));
}

/** Save the order a drag produced. It shows at once; a failure puts the stored order back. */
export async function reorderTracks(ids: string[]) {
  const id = cui.open;
  if (!id) return;
  const byId = new Map(tracks.items.map((p) => [p.id, p]));
  tracks.replace(ids.map((x) => byId.get(x)).filter((p): p is Project => !!p));
  try { await api.reorderCollection(id, ids); }
  catch (e) { setNotice(e instanceof Error ? e.message : String(e)); }
  refetchDetail();
}

export const setLayout = (layout: CollectionLayout) => batch(() => { setCui("layout", layout); closePopups(); });

/** A sort change asks the server again. */
export function setSort(sort: CollectionSort) {
  batch(() => { setCui("sort", sort); closePopups(); });
  load();
}
export const sortByColumn = (col: CollectionSortKey) => setSort(nextCollectionSort(cui.sort, col));

export function setQuery(query: string) {
  if (query === cui.query) return;
  // A search is of the list, so it leaves an opened collection.
  batch(() => { setCui({ query, open: null }); closePopups(); });
  load();
}

export const openDialog = (dialog: DialogKind, id?: string) =>
  batch(() => { if (id) setCui("selected", id); setCui("dialog", dialog); setShell({ menu: null, popover: null }); });
export const closeDialog = () => setCui("dialog", null);

/** Run a write, then reload what it changed. A failure goes to the status bar. */
async function change(write: () => Promise<unknown>): Promise<boolean> {
  try {
    await write();
    await load();
    // Not for a collection the write just deleted, or the request would 404.
    const id = untrack(() => cui.open ?? cui.selected);
    if (id && collectionById(id)) refetchDetail();
    loadCollectionNames();   // the project inspector's lists
    return true;
  } catch (e) {
    setNotice(e instanceof Error ? e.message : String(e));
    return false;
  }
}

const nullIfBlank = (s: string) => (s.trim() ? s.trim() : null);

export async function createCollection(name: string, description: string) {
  let made: CollectionRow | undefined;
  const ok = await change(async () => { made = await api.createCollection(name.trim(), nullIfBlank(description)); });
  if (ok && made) select(made.id);
  return ok;
}

export const editCollection = (id: string, name: string, description: string) =>
  change(() => api.updateCollection(id, { name: name.trim(), description: description.trim() }));

/** A cover is its own write: it applies when the file is chosen, not when the dialog saves. */
export const setCover = (id: string, file: File) => change(() => api.setCollectionCover(id, file));
export const removeCover = (id: string) => change(() => api.removeCollectionCover(id));

export async function renameCollection(id: string, name: string) {
  const c = collectionById(id);
  setCui("renaming", null);
  if (!c || !name.trim() || name.trim() === c.name) return;
  await change(() => api.updateCollection(id, { name: name.trim() }));
}

export async function duplicateCollection(id: string, newName: string) {
  let made: CollectionRow | undefined;
  const ok = await change(async () => { made = await api.duplicateCollection(id, newName.trim()); });
  if (ok && made) select(made.id);
  return ok;
}

export async function deleteCollection(id: string) {
  const ok = await change(() => api.deleteCollection(id));
  if (ok && cui.selected === id) select(null);
  return ok;
}

/** Hand off to the projects view, searching for this collection's projects. */
export const showProjectsIn = (c: CollectionRow) => showProjectsMatching(`collection:"${c.name.replace(/"/g, "")}"`);
