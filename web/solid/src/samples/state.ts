// The samples view's state, shaped like the plugins view's: click-speed state is `sui`, a
// store; the list is a store reconciled by id, so a check that changes one sample's status
// touches only that row. The window's state and the scan's are in ../shell/shell.ts.

import { batch, createMemo, createResource, createSignal, untrack } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { PAGE_SIZE, lastPage, pageOf } from "../../../shared/projects";
import { DEFAULT_SAMPLE_SORT, nextSampleSort, sampleRows, sampleStats, type Presence } from "../../../shared/samples";
import type { Project, SampleFile, SampleFormat, SampleRow, Sort } from "../../../shared/types";
import { api, closePopups, loadChrome, runScan, setCounts, setNotice, showProjectsMatching } from "../shell/shell";

export const [sui, setSui] = createStore({
  query: "",
  format: null as string | null,
  presence: null as Presence | null,
  sort: DEFAULT_SAMPLE_SORT as Sort | null,
  page: 0,
  selected: null as string | null,
});

// ---------------------------------------------------------------- data

export const [samples, setSamples] = createStore<SampleRow[]>([]);
export const [loadError, setLoadError] = createSignal<string | null>(null);
export const [loaded, setLoaded] = createSignal(false);
export const [formats, setFormats] = createSignal<SampleFormat[]>([]);

let loadToken = 0;
export async function load() {
  const token = ++loadToken;
  try {
    const [list, f] = await Promise.all([sui.query ? api.searchSamples(sui.query) : api.samples(), api.sampleFormats()]);
    if (token !== loadToken) return;   // a newer load has started
    batch(() => {
      setSamples(reconcile(list, { key: "id" }));
      setFormats(f);
      if (!sui.query) setCounts("samples", list.length);
      setLoadError(null); setLoaded(true);
    });
  } catch (e) {
    if (token === loadToken) setLoadError(String(e));
  }
}

// ---------------------------------------------------------------- derived

const rows = createMemo(() => sampleRows(samples, { format: sui.format, presence: sui.presence, sort: sui.sort }, formats()));
export const total = () => rows().length;
export const stats = createMemo(() => sampleStats(rows()));
export const filtered = () => !!(sui.query || sui.format || sui.presence);
export const pageRows = createMemo(() => pageOf(rows(), sui.page));
export const pageCount = () => lastPage(total()) + 1;
export const pageSize = PAGE_SIZE;

export const selectedSample = createMemo(() => samples.find((s) => s.id === sui.selected));

/** The selected sample's file record and the projects that use it. `id` says which sample
 *  they belong to, so a stale answer is never shown against the next selection. */
export interface Detail { id: string; file: SampleFile | null; projects: Project[] }
export const [detail, { refetch: refetchDetail }] = createResource(
  () => sui.selected,
  async (id): Promise<Detail> => {
    const [file, projects] = await Promise.all([api.sampleFile(id), api.sampleProjects(id)]);
    return { id, file, projects };
  },
);

// ---------------------------------------------------------------- actions

/** Any change to the filters or the search starts again from the first page. */
function refilter(change: () => void) {
  batch(() => { change(); setSui("page", 0); closePopups(); });
}

export const setFormat = (format: string | null) => refilter(() => setSui("format", format));
export const setPresence = (presence: Presence | null) => refilter(() => setSui("presence", presence));
export const clearFilters = () => refilter(() => setSui({ format: null, presence: null }));
export const sortBy = (col: string) => setSui({ sort: nextSampleSort(sui.sort, col), page: 0 });
export const goPage = (delta: number) => batch(() => { setSui("page", (p) => p + delta); closePopups(); });
export const select = (id: string) => batch(() => { setSui("selected", id); closePopups(); });

export function setQuery(query: string) {
  if (query === sui.query) return;
  refilter(() => setSui("query", query));
  load();
}

/** Check every sample file is still there and measure it, then reload what the check
 *  changed. Disabled while any scan runs (ADR-0038, ADR-0041). */
export const checkSamples = () => runScan("samples", reloadAfterCheck);

async function reloadAfterCheck() {
  await Promise.all([load(), loadChrome()]);
  if (untrack(() => sui.selected)) refetchDetail();
}

/** Hand off to the projects view, searching for the projects that use this sample. */
export const showProjectsUsing = (s: SampleRow) => showProjectsMatching(`sample:"${s.name.replace(/"/g, "")}"`);

export async function copyPath(s: SampleRow) {
  try { await navigator.clipboard.writeText(s.path); closePopups(); }
  catch (e) { setNotice(e instanceof Error ? e.message : String(e)); }
}
