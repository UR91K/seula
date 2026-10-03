import type { AbletonVersion, Key, KeySpelling } from "./types";

const pad2 = (n: number) => String(n).padStart(2, "0");

/** 2026/09/21 02:50 -- the date style of the reference screenshots. */
export function fmtDate(epoch: number): string {
  const d = new Date(epoch * 1000);
  return `${d.getFullYear()}/${pad2(d.getMonth() + 1)}/${pad2(d.getDate())} ${pad2(d.getHours())}:${pad2(d.getMinutes())}`;
}

export function fmtLength(seconds: number | null): string {
  if (seconds == null) return "";
  const s = Math.round(seconds);
  return `${Math.floor(s / 60)}:${pad2(s % 60)}`;
}

export const fmtTempo = (t: number) => (Number.isInteger(t) ? `${t}` : t.toFixed(2));

export const fmtVersion = (v: AbletonVersion) => `${v.major}.${v.minor}.${v.patch}${v.beta ? " beta" : ""}`;

/** The API sends both spellings (ADR-0035); the sharp/flat switch picks one. */
export const fmtKey = (key: Key | null, spelling: KeySpelling) => (key ? key[spelling] : "");

/** File name of an .als path, without extension. */
export const fileStem = (path: string) => (path.split("\\").pop() ?? path).replace(/\.als$/i, "");

/** Split a path at its last backslash, for the middle-truncating path chip. */
export function splitPath(path: string): { head: string; tail: string } {
  const cut = path.lastIndexOf("\\");
  return { head: path.slice(0, cut + 1), tail: path.slice(cut + 1) };
}
