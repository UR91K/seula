// The daemon's HTTP client: plain fetch, as ADR-0048 says. No Tauri IPC for data.

import type {
  Collection, CollectionRow, CollectionSortKey, CollectionStats, CollectionTask, FormatRollup, PluginDetails, PluginRow,
  Project, SampleFile, SampleFormat, SampleRow, ScanProgress, Scope, Statistics, StatsScope, SystemInfo, VendorRollup,
} from "./types";

/** The mock daemon `mockup/data/generate.py` seeds (its HTTP_PORT). The real default is
 *  50052; override with VITE_SEULA_URL. */
export const DEFAULT_URL = "http://127.0.0.1:50152";

const BIG = "limit=10000";

export class Api {
  constructor(readonly base: string = DEFAULT_URL) {}

  private async json<T>(path: string, init?: RequestInit): Promise<T> {
    const res = await fetch(this.base + path, init);
    if (!res.ok) throw new Error(`${init?.method ?? "GET"} ${path}: ${res.status} ${await res.text()}`);
    return res.json() as Promise<T>;
  }

  /** A write whose answer carries nothing the screen needs. */
  private async send(path: string, init: RequestInit): Promise<void> {
    const res = await fetch(this.base + path, init);
    if (!res.ok) throw new Error(`${init.method} ${path}: ${res.status} ${await res.text()}`);
  }

  async projects(scope: Scope): Promise<Project[]> {
    const q = scope === "archived" ? "scope=deleted&" : "";
    return (await this.json<{ projects: Project[] }>(`/api/v1/projects?${q}${BIG}`)).projects;
  }

  async search(query: string): Promise<Project[]> {
    return (await this.json<{ projects: Project[] }>(`/api/v1/search?query=${encodeURIComponent(query)}&limit=200`)).projects;
  }

  async collections(): Promise<Collection[]> {
    return (await this.json<{ collections: Collection[] }>(`/api/v1/collections?${BIG}`)).collections;
  }

  systemInfo() { return this.json<SystemInfo>("/api/v1/system/info"); }

  /** The library's figures in a project scope (ADR-0045). */
  statistics(scope: StatsScope) { return this.json<Statistics>(`/api/v1/system/statistics?scope=${scope}`); }

  /** Where the same figures download as CSV. */
  statisticsExportUrl(scope: StatsScope) { return `${this.base}/api/v1/system/statistics/export?scope=${scope}`; }

  async setNotes(id: string, notes: string) {
    await this.send(`/api/v1/projects/${id}/notes`, put({ notes }));
  }

  async setName(id: string, name: string) {
    await this.send(`/api/v1/projects/${id}/name`, put({ name }));
  }

  /** The total the daemon reports for a list route, without fetching the list. */
  async count(route: "projects" | "plugins" | "samples" | "collections"): Promise<number> {
    return (await this.json<{ total_count: number }>(`/api/v1/${route}?limit=1`)).total_count;
  }

  // ---------------------------------------------------------------- collections

  /** The server sorts this list (unlike plugins and samples), counting in the active scope
   *  (ADR-0043). */
  async collectionList(sortBy: CollectionSortKey, desc: boolean): Promise<CollectionRow[]> {
    return (await this.json<{ collections: CollectionRow[] }>(
      `/api/v1/collections?sort_by=${sortBy}&sort_desc=${desc}&${BIG}`)).collections;
  }

  async searchCollections(query: string): Promise<CollectionRow[]> {
    return (await this.json<{ collections: CollectionRow[] }>(
      `/api/v1/collections/search?query=${encodeURIComponent(query)}&${BIG}`)).collections;
  }

  collectionStats(id: string) { return this.json<CollectionStats>(`/api/v1/collections/${id}/statistics`); }

  collectionTasks(id: string) {
    return this.json<{ tasks: CollectionTask[]; total_tasks: number; completed_tasks: number }>(`/api/v1/collections/${id}/tasks`);
  }

  /** The tracklist, in collection order. */
  async collectionProjects(id: string): Promise<Project[]> {
    return this.json<Project[]>(`/api/v1/collections/${id}/projects`);
  }

  createCollection(name: string, description: string | null) {
    return this.json<CollectionRow>("/api/v1/collections", post({ name, description }));
  }

  updateCollection(id: string, change: { name?: string; description?: string; notes?: string }) {
    return this.json<CollectionRow>(`/api/v1/collections/${id}`, put(change));
  }

  duplicateCollection(id: string, newName: string) {
    return this.json<CollectionRow>(`/api/v1/collections/${id}/duplicate`, post({ new_name: newName }));
  }

  async deleteCollection(id: string) { await this.send(`/api/v1/collections/${id}`, { method: "DELETE" }); }

  async reorderCollection(id: string, projectIds: string[]) {
    await this.send(`/api/v1/collections/${id}/reorder`, put({ project_ids: projectIds }));
  }

  async removeFromCollection(id: string, projectIds: string[]) {
    await this.send(`/api/v1/collections/${id}/batch-remove`, post({ project_ids: projectIds }));
  }

  /** Send an image's bytes and make it the collection's cover (ADR-0061). The daemon keeps
   *  its own copy. The media routes report a failure as a 200 with `success: false`. */
  async setCollectionCover(id: string, file: File): Promise<void> {
    const up = await this.json<{ media_file_id: string; success: boolean; error_message: string | null }>(
      `/api/v1/media/cover-art?collection_id=${id}&filename=${encodeURIComponent(file.name)}`,
      { method: "POST", headers: { "content-type": file.type || "application/octet-stream" }, body: file });
    if (!up.success) throw new Error(up.error_message ?? "The cover was not stored");
    const set = await this.json<{ success: boolean; error_message: string | null }>(
      `/api/v1/collections/${id}/cover-art`, put({ media_file_id: up.media_file_id }));
    if (!set.success) throw new Error(set.error_message ?? "The cover was not set");
  }

  async removeCollectionCover(id: string): Promise<void> {
    const res = await this.json<{ success: boolean; error_message: string | null }>(
      `/api/v1/collections/${id}/cover-art`, { method: "DELETE" });
    if (!res.success) throw new Error(res.error_message ?? "The cover was not removed");
  }

  /** A cover's URL: media is served over HTTP (ADR-0033). */
  mediaUrl(id: string) { return `${this.base}/api/v1/media/${id}`; }

  // ---------------------------------------------------------------- plugins

  async plugins(): Promise<PluginRow[]> {
    return (await this.json<{ plugins: PluginRow[] }>(`/api/v1/plugins?${BIG}`)).plugins;
  }

  async searchPlugins(query: string): Promise<PluginRow[]> {
    return (await this.json<{ plugins: PluginRow[] }>(
      `/api/v1/plugins/search?query=${encodeURIComponent(query)}&${BIG}`)).plugins;
  }

  async vendors(): Promise<VendorRollup[]> {
    return (await this.json<{ vendors: VendorRollup[] }>(`/api/v1/plugins/vendors?${BIG}`)).vendors;
  }

  async formats(): Promise<FormatRollup[]> {
    return (await this.json<{ formats: FormatRollup[] }>("/api/v1/plugins/formats")).formats;
  }

  async pluginDetails(id: string): Promise<PluginDetails> {
    return (await this.json<{ details: PluginDetails }>(`/api/v1/plugins/${id}`)).details;
  }

  async pluginProjects(id: string): Promise<Project[]> {
    return (await this.json<{ projects: Project[] }>(`/api/v1/plugins/${id}/projects?${BIG}`)).projects;
  }

  // ---------------------------------------------------------------- samples

  async samples(): Promise<SampleRow[]> {
    return (await this.json<{ samples: SampleRow[] }>(`/api/v1/samples?${BIG}`)).samples;
  }

  async searchSamples(query: string): Promise<SampleRow[]> {
    return (await this.json<{ samples: SampleRow[] }>(
      `/api/v1/samples/search?query=${encodeURIComponent(query)}&${BIG}`)).samples;
  }

  async sampleFormats(): Promise<SampleFormat[]> {
    return (await this.json<{ formats: SampleFormat[] }>("/api/v1/samples/formats")).formats;
  }

  /** The file record the last check made; null when no check has found the file. */
  async sampleFile(id: string): Promise<SampleFile | null> {
    return (await this.json<{ file: SampleFile | null }>(`/api/v1/samples/${id}`)).file;
  }

  async sampleProjects(id: string): Promise<Project[]> {
    return (await this.json<{ projects: Project[] }>(`/api/v1/samples/${id}/projects?${BIG}`)).projects;
  }

  // ---------------------------------------------------------------- scans

  /** Start a scan and follow it. A scan's `POST` answers with the SSE stream itself, which
   *  `EventSource` cannot read (it only makes GET requests), so the body is read by hand
   *  (ADR-0056). A scan that is already running answers 409 and this throws. */
  scan(kind: "projects" | "plugins" | "samples", signal?: AbortSignal): AsyncGenerator<ScanProgress> {
    const route = { projects: "/api/v1/system/scan", plugins: "/api/v1/plugins/scan", samples: "/api/v1/samples/check" }[kind];
    return sse<ScanProgress>(this.base + route, { method: "POST", signal });
  }
}

const put = (body: unknown): RequestInit => ({
  method: "PUT", headers: { "content-type": "application/json" }, body: JSON.stringify(body),
});

const post = (body: unknown): RequestInit => ({
  method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify(body),
});

/** Parse a text/event-stream response body into its `data:` payloads. */
export async function* sse<T>(url: string, init: RequestInit): AsyncGenerator<T> {
  const res = await fetch(url, init);
  if (!res.ok) throw new Error(`${init.method ?? "GET"} ${url}: ${res.status} ${await res.text()}`);
  const reader = res.body!.pipeThrough(new TextDecoderStream()).getReader();
  let buf = "";
  try {
    for (;;) {
      const { value, done } = await reader.read();
      if (done) return;
      buf += value;
      let end: number;
      while ((end = buf.indexOf("\n\n")) >= 0) {
        const block = buf.slice(0, end);
        buf = buf.slice(end + 2);
        const data = block.split("\n").filter((l) => l.startsWith("data:")).map((l) => l.slice(5).trimStart()).join("\n");
        if (data) yield JSON.parse(data) as T;
      }
    }
  } finally {
    // The consumer may stop early; release the connection rather than leave it open.
    await reader.cancel().catch(() => {});
  }
}

/** A stand-in scan for judging how each framework takes fast pushes: `total` events at
 *  about `perSecond`, shaped like the real stream. Dev only; the real scan is `Api.scan`. */
export async function* simulatedScan(total = 600, perSecond = 40, signal?: AbortSignal): AsyncGenerator<ScanProgress> {
  for (let i = 1; i <= total; i++) {
    if (signal?.aborted) return;
    await new Promise((r) => setTimeout(r, 1000 / perSecond));
    const done = i === total;
    yield { completed: i, total, progress: i / total, status: done ? "completed" : "parsing",
      message: done ? `Scanned ${total} projects` : `Parsed simulated project ${i}.als (${i}/${total})` };
  }
}
