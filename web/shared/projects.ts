// The projects screen's logic with no framework in it: columns, sorting, paging, the
// selection rules and the counts. Both comparison apps import this, so what differs
// between them is only how state is held and how the DOM is updated (ADR-0047).

import { fmtDate, fmtKey, fmtLength, fmtTempo, fmtVersion } from "./format";
import type { Collection, KeySpelling, Project, Sort } from "./types";

export interface Column {
  id: string;
  label: string;
  w: number;               // default width, in --u
  num?: boolean;           // right-aligned figure
  dim?: boolean;           // secondary text colour
  count?: "plugins" | "samples";   // a count cell with a hover list and a missing marker
  hidden?: boolean;        // unchecked in the column chooser to start with
  value(p: Project, spelling: KeySpelling): string | number;
  text(p: Project, spelling: KeySpelling): string;
}

const ver = (p: Project) =>
  [p.ableton_version.major, p.ableton_version.minor, p.ableton_version.patch].map((n) => String(n).padStart(3, "0")).join("");

export const COLUMNS: Column[] = [
  { id: "modified", dim: true, label: "Date modified", w: 56, value: (p) => p.modified_at, text: (p) => fmtDate(p.modified_at) },
  { id: "created", dim: true, label: "Date created", w: 56, hidden: true, value: (p) => p.created_at, text: (p) => fmtDate(p.created_at) },
  { id: "length", label: "Length", w: 25, num: true, value: (p) => p.duration_seconds ?? -1, text: (p) => fmtLength(p.duration_seconds) },
  { id: "tempo", label: "Tempo", w: 26, num: true, value: (p) => p.tempo, text: (p) => fmtTempo(p.tempo) },
  { id: "key", label: "Key", w: 58, value: (p, s) => fmtKey(p.key_signature, s), text: (p, s) => fmtKey(p.key_signature, s) },
  { id: "time", label: "Time", w: 20, num: true, value: (p) => p.time_signature.numerator / p.time_signature.denominator, text: (p) => `${p.time_signature.numerator}/${p.time_signature.denominator}` },
  { id: "plugins", label: "Plugins", w: 27, num: true, count: "plugins", value: (p) => p.plugins.length, text: (p) => (p.plugins.length ? String(p.plugins.length) : "") },
  { id: "samples", label: "Samples", w: 29, num: true, count: "samples", value: (p) => p.samples.length, text: (p) => (p.samples.length ? String(p.samples.length) : "") },
  { id: "live", dim: true, label: "Live", w: 32, value: (p) => ver(p), text: (p) => fmtVersion(p.ableton_version) },
];

export const DEFAULT_COLUMNS = COLUMNS.filter((c) => !c.hidden).map((c) => c.id);
export const columnById = (id: string) => COLUMNS.find((c) => c.id === id)!;

export const DEFAULT_SORT: Sort = { col: "modified", desc: true };
export const PAGE_SIZE = 100;

/** Rows in the chosen order. `name` and `tags` sort too, though they are not in COLUMNS. */
export function sortRows(rows: Project[], sort: Sort | null, spelling: KeySpelling): Project[] {
  if (!sort) return rows;
  const dir = sort.desc ? -1 : 1;
  const col = COLUMNS.find((c) => c.id === sort.col);
  const val = col ? (p: Project) => col.value(p, spelling)
    : sort.col === "tags" ? (p: Project) => p.tags.length
    : (p: Project) => p.name.toLowerCase();
  return [...rows].sort((a, b) => {
    const x = val(a), y = val(b);
    return (x < y ? -1 : x > y ? 1 : 0) * dir;
  });
}

/** The sort a click on a header gives: the same column flips, a new one starts at its
 *  natural direction (newest first for dates). */
export function nextSort(current: Sort | null, col: string): Sort {
  if (current && current.col === col) return { col, desc: !current.desc };
  return { col, desc: col === "modified" || col === "created" };
}

export const pageOf = <T,>(rows: T[], page: number, size = PAGE_SIZE) => rows.slice(page * size, (page + 1) * size);
export const lastPage = (total: number, size = PAGE_SIZE) => Math.max(0, Math.ceil(total / size) - 1);

// ---------------------------------------------------------------- selection

export interface Selection { selected: Set<string>; anchor: string | null }

/** A click on a row. Plain replaces the selection, Ctrl/Meta toggles, Shift extends from
 *  the anchor across the page's ids. Returns a new selection; never mutates the old. */
export function clickRow(
  cur: Selection, id: string, pageIds: string[], mods: { ctrl: boolean; shift: boolean },
): Selection {
  if (mods.ctrl) {
    const selected = new Set(cur.selected);
    selected.has(id) ? selected.delete(id) : selected.add(id);
    return { selected, anchor: id };
  }
  if (mods.shift && cur.anchor) {
    const [a, b] = [pageIds.indexOf(cur.anchor), pageIds.indexOf(id)].sort((x, y) => x - y);
    if (a >= 0) return { selected: new Set(pageIds.slice(a, b + 1)), anchor: cur.anchor };
  }
  return { selected: new Set([id]), anchor: id };
}

/** A row's checkbox toggles that row alone, like Ctrl-click. */
export const toggleRow = (cur: Selection, id: string): Selection => clickRow(cur, id, [], { ctrl: true, shift: false });

export function toggleAll(cur: Selection, pageIds: string[]): Selection {
  const all = pageIds.length > 0 && pageIds.every((id) => cur.selected.has(id));
  return { selected: new Set(all ? [] : pageIds), anchor: cur.anchor };
}

// ---------------------------------------------------------------- derived figures

export const missingPlugins = (p: Project) => p.plugins.filter((x) => x.installed === false).length;
export const missingSamples = (p: Project) => p.samples.filter((x) => !x.is_present).length;

/** Items common to every project in a list, by id. */
export function common<T extends { id: string }>(projects: Project[], pick: (p: Project) => T[]): T[] {
  if (!projects.length) return [];
  const [first, ...rest] = projects;
  return pick(first).filter((x) => rest.every((p) => pick(p).some((y) => y.id === x.id)));
}

export const collectionsOf = (p: Project, byId: Map<string, Collection>) =>
  p.collection_ids.map((id) => byId.get(id)).filter((c): c is Collection => !!c);

/** Collections that hold every one of the projects. */
export function commonCollections(projects: Project[], byId: Map<string, Collection>): Collection[] {
  if (!projects.length) return [];
  const [first, ...rest] = projects;
  return collectionsOf(first, byId).filter((c) => rest.every((p) => p.collection_ids.includes(c.id)));
}

export const plural = (n: number, one: string, many = `${one}s`) => `${n} ${n === 1 ? one : many}`;
