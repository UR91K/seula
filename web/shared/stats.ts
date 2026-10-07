// The stats view's logic with no framework in it: the labels, the marks each chart draws
// and how long each one is. The series come oldest first with empty periods as zero
// (ADR-0045), so a chart draws them as they come.

import { fmtKey } from "./format";
import { plural } from "./projects";
import type { KeySpelling, Statistics, StatsScope } from "./types";

export const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
export const MONTHS_LONG = ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];

export const pct = (x: number) => `${Math.round(x * 100)}%`;
export const dec1 = (x: number) => (Math.round(x * 10) / 10).toFixed(1);
export const num = (n: number) => n.toLocaleString("en-US");

export const SCOPES: Record<StatsScope, string> = { active: "Active projects", all: "All projects" };

/** A length of a bar or column, as a percentage of the largest. */
export const share = (n: number, max: number) => (100 * n) / Math.max(1, max);

// ---------------------------------------------------------------- overview

export interface TilePart {
  n: number; label: string;
  /** The colour class: is-accent, is-quiet, is-installed, is-absent or is-unscanned. */
  cls: string;
  /** The glyph beside the part: a swatch, or the icon that names it. */
  glyph: { swatch: true } | { icon: string; fill?: boolean; status?: "ok" | "missing" | "unknown" };
  /** A part the scope leaves out. */
  off?: boolean;
}

export interface Tile { label: string; total: number; note: string; parts: TilePart[] }

/** The six overview counts, each with its parts. The total is their sum (ADR-0045). */
export function overview(s: Statistics, scope: StatsScope): Tile[] {
  const sw = { swatch: true } as const;
  return [
    { label: "Projects", total: s.projects.total, note: "", parts: [
      { n: s.projects.active, label: "active", cls: "is-accent", glyph: sw },
      { n: s.projects.archived, label: "archived", cls: "is-quiet", glyph: { icon: "inventory_2" }, off: scope === "active" },
    ] },
    { label: "Plugins", total: s.plugins.total, note: "", parts: [
      { n: s.plugins.installed, label: "installed", cls: "is-installed", glyph: { icon: "check_circle", fill: true, status: "ok" } },
      { n: s.plugins.missing, label: "missing", cls: "is-absent", glyph: { icon: "error", fill: true, status: "missing" } },
      { n: s.plugins.not_scanned, label: "not scanned", cls: "is-unscanned", glyph: { icon: "help", status: "unknown" } },
    ] },
    { label: "Samples", total: s.samples.total, note: "", parts: [
      { n: s.samples.present, label: "present", cls: "is-installed", glyph: { icon: "check_circle", fill: true, status: "ok" } },
      { n: s.samples.missing, label: "missing", cls: "is-absent", glyph: { icon: "error", fill: true, status: "missing" } },
    ] },
    { label: "Collections", total: s.collections.total, note: "", parts: [
      { n: s.collections.with_projects, label: "with projects", cls: "is-accent", glyph: sw },
      { n: s.collections.empty, label: "empty", cls: "is-quiet", glyph: { icon: "check_box_outline_blank" } },
    ] },
    { label: "Tags", total: s.tags.total, note: "", parts: [
      { n: s.tags.in_use, label: "in use", cls: "is-accent", glyph: sw },
      { n: s.tags.unused, label: "unused", cls: "is-quiet", glyph: { icon: "label_off" } },
    ] },
    { label: "Tasks", total: s.tasks.total, note: s.tasks.total ? `${pct(s.tasks.completion_rate)} done` : "", parts: [
      { n: s.tasks.completed, label: "completed", cls: "is-installed", glyph: { icon: "task_alt" } },
      { n: s.tasks.pending, label: "pending", cls: "is-quiet", glyph: { icon: "radio_button_unchecked" } },
    ] },
  ];
}

// ---------------------------------------------------------------- charts

/** A hoverable mark: `hid` identifies it for the tooltip, `tip` is what the tooltip says. */
export interface Mark { hid: string; tip: string }

export interface Column extends Mark { n: number; label: string; sub?: string | number }

/** The tallest column: the only one that is labelled. */
export function peak(items: Column[]): Column | undefined {
  return items.reduce<Column | undefined>((best, i) => (!best || i.n > best.n ? i : best), undefined);
}

export interface Bar extends Mark { n: number; label: string; quiet?: boolean }

export function tempoColumns(s: Statistics): Column[] {
  return s.tempo_distribution.map((b) => ({
    n: b.count, label: `${b.tempo}`, hid: `tempo:${b.tempo}`,
    tip: `${b.tempo}–${b.tempo + 9} BPM · ${plural(b.count, "project")}`,
  }));
}

export const KEYS_SHOWN = 12;

/** Keys, most common first, with "No key" last. `more` counts the keys left off. */
export function keyBars(s: Statistics, spelling: KeySpelling): { bars: Bar[]; more: number; keyed: number } {
  const keyed = s.key_distribution.filter((k) => k.key);
  const none = s.key_distribution.find((k) => !k.key);
  const bars: Bar[] = keyed.slice(0, KEYS_SHOWN).map((k) => ({
    label: fmtKey(k.key, spelling), n: k.count, hid: `key:${k.key!.tonic}-${k.key!.scale}`,
    tip: `${fmtKey(k.key, spelling)} · ${plural(k.count, "project")}`,
  }));
  if (none) bars.push({ label: "No key", n: none.count, quiet: true, hid: "key:none", tip: `No key detected · ${plural(none.count, "project")}` });
  return { bars, more: Math.max(0, keyed.length - KEYS_SHOWN), keyed: keyed.length };
}

export const timeSignatureBars = (s: Statistics): Bar[] => s.time_signature_distribution.map((t) => ({
  label: `${t.numerator}/${t.denominator}`, n: t.count, hid: `ts:${t.numerator}/${t.denominator}`,
  tip: `${t.numerator}/${t.denominator} · ${plural(t.count, "project")}`,
}));

export const monthColumns = (s: Statistics): Column[] => s.projects_per_month.map((m, i) => ({
  n: m.count, label: MONTHS[m.month - 1], sub: i === 0 || m.month === 1 ? m.year : "",
  hid: `month:${m.year}-${m.month}`, tip: `${MONTHS_LONG[m.month - 1]} ${m.year} · ${plural(m.count, "project")} created`,
}));

/** The mean of the months drawn, for the reference line. */
export const monthAverage = (s: Statistics) =>
  s.projects_per_month.length ? s.projects_per_month.reduce((a, m) => a + m.count, 0) / s.projects_per_month.length : 0;

export const yearColumns = (s: Statistics): Column[] => s.projects_per_year.map((y) => ({
  n: y.count, label: `${y.year}`, hid: `year:${y.year}`, tip: `${y.year} · ${plural(y.count, "project")} created`,
}));

export interface TaskColumn extends Column { completed: number; pending: number }

export const taskColumns = (s: Statistics): TaskColumn[] => s.task_completion_trends.map((t, i) => ({
  n: t.total_tasks, completed: t.completed_tasks, pending: t.total_tasks - t.completed_tasks,
  label: MONTHS[t.month - 1].slice(0, 1), sub: i === 0 || t.month === 1 ? `${t.year}`.slice(2) : "",
  hid: `task:${t.year}-${t.month}`,
  tip: t.total_tasks
    ? `${MONTHS_LONG[t.month - 1]} ${t.year} · ${t.completed_tasks} of ${t.total_tasks} completed (${pct(t.completion_rate)})`
    : `${MONTHS_LONG[t.month - 1]} ${t.year} · no tasks created`,
}));

export const VERSIONS_SHOWN = 8;

export const versionBars = (s: Statistics): { bars: Bar[]; more: number } => ({
  bars: s.ableton_versions.slice(0, VERSIONS_SHOWN).map((v) => ({
    label: v.version, n: v.count, hid: `ver:${v.version}`, tip: `Live ${v.version} · ${plural(v.count, "project")}`,
  })),
  more: Math.max(0, s.ableton_versions.length - VERSIONS_SHOWN),
});

export const tagBars = (s: Statistics): (Bar & { name: string })[] => s.top_tags.map((t) => ({
  name: t.name, label: t.name, n: t.usage_count, hid: `tag:${t.name}`, tip: `${t.name} · ${plural(t.usage_count, "project")}`,
}));

// ---------------------------------------------------------------- the last thirty days

export interface Day extends Mark { created: number; modified: number; tick: string }
export type DayKey = "created" | "modified";

const utc = (d: { year: number; month: number; day: number }) => new Date(Date.UTC(d.year, d.month - 1, d.day));

/** Each day with its tooltip, and a tick label on every seventh day back from the last. */
export function days(s: Statistics): Day[] {
  const list = s.recent_activity;
  return list.map((d, i) => {
    const when = utc(d).toLocaleDateString("en-GB", { weekday: "short", day: "numeric", month: "short", timeZone: "UTC" });
    return {
      created: d.projects_created, modified: d.projects_modified,
      hid: `${d.year}-${d.month}-${d.day}`,
      tip: `${when} · ${d.projects_created} created, ${d.projects_modified} modified`,
      tick: (list.length - 1 - i) % 7 === 0 ? `${d.day} ${MONTHS[d.month - 1]}` : "",
    };
  });
}

/** The tallest day of either strip, so the two share a scale. */
export const dayMax = (list: Day[]) => Math.max(1, ...list.map((d) => Math.max(d.created, d.modified)));

// ---------------------------------------------------------------- status bar

/** The status bar's segments for a scope: how many projects, and whether archived ones count. */
export function statusFor(s: Statistics, scope: StatsScope): { count: string; note: string; archived: boolean } {
  return scope === "all"
    ? { count: `${s.projects.total} projects`, note: "Counting archived projects", archived: true }
    : { count: `${s.projects.active} active projects`, note: `${s.projects.archived} archived, not counted`, archived: false };
}

/** Nothing has been scanned: every figure would be zero. */
export const isEmpty = (s: Statistics) => s.projects.total === 0;
