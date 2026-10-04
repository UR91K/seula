// The plugins view's state. Click-speed state (filters, sort, selection) is `pui`, a
// store; the list is a store reconciled by id, so a scan that changes one plugin's status
// touches only that row. The window's state and the scan's are in ../shell/shell.ts.

import { batch, createMemo, createResource, createSignal, untrack } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import {
  DEFAULT_PLUGIN_SORT, layout, nextPluginSort, pluginRows, pluginStats, type Group, type InstallState,
} from "../../../shared/plugins";
import { PAGE_SIZE, lastPage, pageOf } from "../../../shared/projects";
import type { FormatRollup, PluginDetails, PluginRow, Project, Sort, VendorRollup } from "../../../shared/types";
import { setQuery as showInProjects } from "../projects/state";
import { api, closePopups, loadChrome, runScan, setCounts, setNotice, setRoute } from "../shell/shell";

export const [pui, setPui] = createStore({
  query: "",
  vendor: null as string | null,
  format: null as string | null,
  states: [] as InstallState[],
  group: null as Group | null,
  collapsed: {} as Record<string, boolean>,   // group keys folded shut
  sort: DEFAULT_PLUGIN_SORT as Sort | null,
  page: 0,
  selected: null as string | null,
  vendorQuery: "",
});

// ---------------------------------------------------------------- data

export const [plugins, setPlugins] = createStore<PluginRow[]>([]);
export const [loadError, setLoadError] = createSignal<string | null>(null);
export const [loaded, setLoaded] = createSignal(false);
export const [vendors, setVendors] = createSignal<VendorRollup[]>([]);
export const [formats, setFormats] = createSignal<FormatRollup[]>([]);

let loadToken = 0;
export async function load() {
  const token = ++loadToken;
  try {
    const [list, v, f] = await Promise.all([
      pui.query ? api.searchPlugins(pui.query) : api.plugins(), api.vendors(), api.formats(),
    ]);
    if (token !== loadToken) return;   // a newer load has started
    batch(() => {
      setPlugins(reconcile(list, { key: "id" }));
      setVendors(v); setFormats(f);
      if (!pui.query) setCounts("plugins", list.length);
      setLoadError(null); setLoaded(true);
    });
  } catch (e) {
    if (token === loadToken) setLoadError(String(e));
  }
}

// ---------------------------------------------------------------- derived

const rows = createMemo(() => pluginRows(plugins, {
  vendor: pui.vendor, format: pui.format, states: pui.states, group: pui.group, sort: pui.sort,
}));
export const total = () => rows().length;
export const stats = createMemo(() => pluginStats(rows()));
export const filtered = () => !!(pui.query || pui.vendor || pui.format || pui.states.length);
const pageRows = createMemo(() => pageOf(rows(), pui.page));
export const entries = createMemo(() => layout(rows(), pageRows(), pui.group, pui.collapsed));
export const pageCount = () => lastPage(total()) + 1;
export const pageSize = PAGE_SIZE;

export const selectedPlugin = createMemo(() => plugins.find((p) => p.id === pui.selected));

/** The selected plugin's details and the projects that use it. `id` says which plugin they
 *  belong to, so a stale answer is never shown against the next selection. */
export interface Detail { id: string; details: PluginDetails; projects: Project[] }
export const [detail, { refetch: refetchDetail }] = createResource(
  () => pui.selected,
  async (id): Promise<Detail> => {
    const [details, projects] = await Promise.all([api.pluginDetails(id), api.pluginProjects(id)]);
    return { id, details, projects };
  },
);

// ---------------------------------------------------------------- actions

/** Any change to the filters or the search starts again from the first page. */
function refilter(change: () => void) {
  batch(() => { change(); setPui("page", 0); closePopups(); });
}

export const setFormat = (format: string | null) => refilter(() => setPui("format", format));
export const setVendor = (vendor: string | null) => refilter(() => setPui({ vendor, vendorQuery: "" }));
export const clearFilters = () => refilter(() => setPui({ vendor: null, format: null, states: [] }));
export const toggleState = (id: InstallState) => {
  setPui("states", (s) => (s.includes(id) ? s.filter((x) => x !== id) : [...s, id]));
  setPui("page", 0);
};
export const setGroup = (group: Group | null) => batch(() => { setPui({ group, collapsed: {} }); closePopups(); });
export const toggleGroup = (key: string) => setPui("collapsed", key, (c) => !c);
export const sortBy = (col: string) => batch(() => { setPui({ sort: nextPluginSort(pui.sort, col), page: 0 }); });
export const goPage = (delta: number) => batch(() => { setPui("page", (p) => p + delta); closePopups(); });
export const select = (id: string) => batch(() => { setPui("selected", id); closePopups(); });

export function setQuery(query: string) {
  if (query === pui.query) return;
  refilter(() => setPui("query", query));
  load();
}

/** Scan the plugin folders, then reload what the scan changed. Disabled while any scan
 *  runs, a project scan included (ADR-0038); the daemon answers 409 to a second one. */
export const scanPlugins = () => runScan("plugins", reloadAfterScan);

async function reloadAfterScan() {
  await Promise.all([load(), loadChrome()]);
  if (untrack(() => pui.selected)) refetchDetail();
}

/** Hand off to the projects view, searching for the projects that use this plugin. */
export function showProjectsUsing(p: PluginRow) {
  batch(() => { closePopups(); setRoute("projects"); });
  showInProjects(`plugin:"${p.name.replace(/"/g, "")}"`);
}

export async function copyName(p: PluginRow) {
  try { await navigator.clipboard.writeText(p.name); closePopups(); }
  catch (e) { setNotice(e instanceof Error ? e.message : String(e)); }
}
