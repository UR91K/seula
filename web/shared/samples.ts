// The samples view's logic with no framework in it: formats, the present filter, the
// columns, filtering, sorting and the counts.

import { splitPath } from "./format";
import type { SampleFormat, SampleRow, Sort } from "./types";

export type Presence = "present" | "missing";

export const PRESENCE: { id: Presence; label: string; value: boolean }[] = [
  { id: "present", label: "Present", value: true },
  { id: "missing", label: "Missing", value: false },
];

export const presenceLabel = (present: boolean) => (present ? "Present" : "Missing");

export interface SampleFilters {
  format: string | null;
  presence: Presence | null;
  sort: Sort | null;
}

const OTHER: SampleFormat = { format: "other", name: "Other", extensions: [], count: 0, present_count: 0, missing_count: 0, total_size_bytes: 0 };

/** The format a path is in, by the extensions the formats route lists for each. */
export function formatOf(path: string, formats: SampleFormat[]): SampleFormat {
  const ext = path.split(".").pop()?.toLowerCase() ?? "";
  return formats.find((f) => f.extensions.includes(ext)) ?? OTHER;
}

/** A path's folder, without its trailing separator. */
export const folderOf = (path: string) => splitPath(path).head.replace(/[\\/]$/, "");

export interface SampleColumn {
  id: string;
  label: string;
  w: number;       // default width, in --u
  num?: boolean;
  dim?: boolean;
  sort(s: SampleRow, formats: SampleFormat[]): string | number;
}

export const SAMPLE_COLUMNS: SampleColumn[] = [
  { id: "name", label: "Name", w: 120, sort: (s) => s.name.toLowerCase() },
  { id: "status", label: "Status", w: 40, sort: (s) => (s.is_present ? 0 : 1) },
  { id: "folder", label: "Folder", w: 176, sort: (s) => folderOf(s.path).toLowerCase() },
  { id: "format", label: "Format", w: 30, dim: true, sort: (s, f) => formatOf(s.path, f).name },
  { id: "size", label: "Size", w: 32, num: true, sort: (s) => s.size_bytes ?? -1 },
  { id: "projects", label: "Projects", w: 32, num: true, sort: (s) => s.project_count },
];

export const DEFAULT_SAMPLE_SORT: Sort = { col: "projects", desc: true };

/** A click on a header: the same column flips, a new one starts at its natural direction
 *  (most-used and largest first). */
export const nextSampleSort = (current: Sort | null, col: string): Sort =>
  current && current.col === col ? { col, desc: !current.desc } : { col, desc: col === "projects" || col === "size" };

const cmp = (x: string | number, y: string | number) => (x < y ? -1 : x > y ? 1 : 0);

/** The samples the list shows: the toolbar filters, then the sort. `list` is the whole
 *  library, or a search's hits. */
export function sampleRows(list: SampleRow[], f: SampleFilters, formats: SampleFormat[]): SampleRow[] {
  const rows = list.filter((s) => (!f.format || formatOf(s.path, formats).format === f.format)
    && (!f.presence || s.is_present === (f.presence === "present")));
  const col = SAMPLE_COLUMNS.find((c) => c.id === f.sort?.col) ?? SAMPLE_COLUMNS[0];
  const dir = f.sort?.desc ? -1 : 1;
  return rows.sort((a, b) => cmp(col.sort(a, formats), col.sort(b, formats)) * dir || cmp(a.name.toLowerCase(), b.name.toLowerCase()));
}

export interface SampleStats {
  total: number; present: number; missing: number;
  /** Present samples a check has measured, and their sizes added up. */
  measured: number; sizeBytes: number;
}

/** The status bar's counts for the rows the filters list. Only present samples add to the
 *  size, as the stats route counts them. */
export function sampleStats(rows: SampleRow[]): SampleStats {
  const present = rows.filter((s) => s.is_present);
  const sized = present.filter((s) => s.size_bytes != null);
  return {
    total: rows.length, present: present.length, missing: rows.length - present.length,
    measured: sized.length, sizeBytes: sized.reduce((n, s) => n + s.size_bytes!, 0),
  };
}
