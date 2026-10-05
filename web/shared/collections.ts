// The collections view's logic with no framework in it: the sort choices, the details
// table's columns, and how a length and a facts line read. The server sorts this list
// (ADR-0044), so nothing here sorts rows.

import { fmtDate, fmtKey, fmtLength } from "./format";
import { plural } from "./projects";
import type {
  CollectionRow, CollectionSortKey, CollectionStats, CollectionTask, KeySpelling,
} from "./types";

export type CollectionLayout = "grid" | "details";

export interface CollectionSort { col: CollectionSortKey; desc: boolean }

export const DEFAULT_COLLECTION_SORT: CollectionSort = { col: "name", desc: false };

/** The grid's Sort ▾: what each choice asks the server for. */
export const COLLECTION_SORTS: { label: string; sort: CollectionSort }[] = [
  { label: "Name", sort: { col: "name", desc: false } },
  { label: "Most projects", sort: { col: "project_count", desc: true } },
  { label: "Longest", sort: { col: "total_duration", desc: true } },
  { label: "Newest", sort: { col: "created_at", desc: true } },
  { label: "Recently changed", sort: { col: "modified_at", desc: true } },
];

export const sameSort = (a: CollectionSort, b: CollectionSort) => a.col === b.col && a.desc === b.desc;

/** A click on a details header: the same column flips, a new one starts at its natural
 *  direction (biggest and newest first, names A to Z). */
export const nextCollectionSort = (current: CollectionSort, col: CollectionSortKey): CollectionSort =>
  current.col === col ? { col, desc: !current.desc } : { col, desc: col !== "name" };

export interface CollectionColumn {
  id: string;
  label: string;
  w: number;                    // default width, in --u
  /** The server's `sort_by`; a column with none (description) does not sort. */
  key?: CollectionSortKey;
  num?: boolean;
  dim?: boolean;
  text?(c: CollectionRow): string;
}

export const COLLECTION_COLUMNS: CollectionColumn[] = [
  { id: "name", key: "name", label: "Name", w: 110 },
  { id: "description", label: "Description", w: 150, dim: true, text: (c) => c.description ?? "" },
  { id: "projects", key: "project_count", label: "Projects", w: 30, num: true, text: (c) => String(c.project_count || "") },
  { id: "length", key: "total_duration", label: "Length", w: 32, num: true, text: (c) => fmtTotal(c.total_duration_seconds) },
  { id: "created", key: "created_at", label: "Date created", w: 56, dim: true, text: (c) => fmtDate(c.created_at) },
  { id: "modified", key: "modified_at", label: "Date modified", w: 56, dim: true, text: (c) => fmtDate(c.modified_at) },
];

/** A collection's length: minutes and seconds, or hours once it runs past one. */
export function fmtTotal(seconds: number | null): string {
  if (seconds == null) return "";
  const s = Math.round(seconds);
  if (s < 3600) return fmtLength(s);
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${Math.floor(s / 3600)}:${pad(Math.floor((s % 3600) / 60))}:${pad(s % 60)}`;
}

/** What a card says under its name. */
export const cardMeta = (c: CollectionRow) =>
  c.project_count ? `${plural(c.project_count, "project")} · ${fmtTotal(c.total_duration_seconds)}` : "Empty";

/** The facts line of an opened collection's header, leaving out what is absent. */
export function collectionFacts(
  stats: CollectionStats, tasks: { total_tasks: number; completed_tasks: number }, spelling: KeySpelling,
): string[] {
  return [
    plural(stats.project_count, "project"),
    fmtTotal(stats.total_duration_seconds),
    stats.average_tempo != null && `${Math.round(stats.average_tempo)} BPM average`,
    stats.most_common_key && fmtKey(stats.most_common_key, spelling),
    stats.most_common_time_signature,
    stats.total_plugins > 0 && plural(stats.total_plugins, "plugin"),
    stats.total_samples > 0 && plural(stats.total_samples, "sample"),
    tasks.total_tasks > 0 && `${tasks.completed_tasks} of ${plural(tasks.total_tasks, "task")} done`,
  ].filter((f): f is string => !!f);
}

export const tasksDone = (tasks: CollectionTask[]) => tasks.filter((t) => t.completed).length;
