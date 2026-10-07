// The stats view's chart parts, as the mockup draws them: CSS bars, not SVG. Every mark
// carries `data-hid` and `data-tip`; the page's one pointer handler shows the tooltip.

import { For, Show, type JSX } from "solid-js";
import { dec1, num, peak, share, type Bar, type Column, type TaskColumn, type Tile } from "../../../shared/stats";
import { Icon } from "../shell/parts";

const tip = (m: { hid: string; tip: string }) => ({ "data-hid": m.hid, "data-tip": m.tip });

export function StatTile(props: { tile: Tile }) {
  const shown = () => props.tile.parts.filter((p) => p.n > 0);
  return (
    <div class="st-tile">
      <div class="st-k">{props.tile.label}</div>
      <div class="st-n">
        {num(props.tile.total)}
        <Show when={props.tile.note}> <span class="st-note">{props.tile.note}</span></Show>
      </div>
      <div class="st-split" classList={{ none: shown().length === 0 }}>
        <For each={shown()}>{(p) => (
          <i class={p.cls} classList={{ off: p.off }} style={{ flex: p.n }}
            {...tip({ hid: `${props.tile.label}:${p.label}`, tip: `${p.n} ${p.label}` })} />
        )}</For>
      </div>
      <ul class="st-parts">
        <For each={props.tile.parts}>{(p) => (
          <li classList={{ off: p.off }}>
            <span class={`st-glyph ${p.cls}`}>
              {"swatch" in p.glyph
                ? <i class="st-sw" />
                : <span class="status-dot" classList={{
                    "is-ok": p.glyph.status === "ok", "is-missing": p.glyph.status === "missing", "is-unknown": p.glyph.status === "unknown",
                  }}><Icon name={p.glyph.icon} fill={p.glyph.fill} /></span>}
            </span>
            <span class="n">{num(p.n)}</span> {p.label}
            <Show when={p.off}> <span class="faint">not counted</span></Show>
          </li>
        )}</For>
      </ul>
    </div>
  );
}

/**
 * Vertical columns from one baseline. Only the tallest is labelled; the rest say their
 * value on hover. `avg` draws a reference line with its value; `stacked` splits a column
 * into segments, bottom first.
 */
export function Columns<C extends Column>(props: {
  items: C[]; height?: number; avg?: number; narrow?: boolean; stacked?: (c: C) => { cls: string; n: number }[];
}) {
  const max = () => Math.max(1, ...props.items.map((i) => i.n));
  const top = () => peak(props.items);
  return (
    <div class="st-cols" classList={{ narrow: props.narrow }} style={{ "--plot": `calc(var(--u) * ${props.height ?? 40})` }}>
      <div class="st-plot">
        <Show when={props.avg != null && props.avg > 0}>
          <div class="st-avg" style={{ bottom: `${share(props.avg!, max())}%` }}><span>avg {dec1(props.avg!)}</span></div>
        </Show>
      </div>
      <For each={props.items}>{(i) => (
        <div class="st-col" classList={{ zero: !i.n }} {...tip(i)}>
          <div class="st-bar">
            <Show when={i === top() && i.n}>
              <span class="st-cap" style={{ bottom: `${share(i.n, max())}%` }}>{i.n}</span>
            </Show>
            <Show when={props.stacked} fallback={<i style={{ height: `${share(i.n, max())}%` }} />}>
              <For each={props.stacked!(i).filter((seg) => seg.n > 0)}>{(seg) => (
                <i class={seg.cls} style={{ height: `${share(seg.n, max())}%` }} />
              )}</For>
            </Show>
          </div>
          <div class="st-x">{i.label}<Show when={i.sub}><br /><span class="faint">{i.sub}</span></Show></div>
        </div>
      )}</For>
    </div>
  );
}

export const taskStack = (t: TaskColumn) => [{ cls: "is-quiet", n: t.pending }, { cls: "is-installed", n: t.completed }];

/** Horizontal bars in rows at table density, the value in its own column. A bar's length
 *  is relative to the largest. `label` lets a row draw its own label, a tag chip say. */
export function BarList(props: {
  items: Bar[]; more?: number; moreLabel?: string; labelW?: number; label?: (b: Bar) => JSX.Element;
}) {
  const max = () => Math.max(1, ...props.items.map((i) => i.n));
  return (
    <table class="st-bars" style={{ "--lw": `calc(var(--u) * ${props.labelW ?? 50})` }}>
      <tbody>
        <For each={props.items}>{(i) => (
          <tr classList={{ quiet: i.quiet }} {...tip(i)}>
            <td class="l">{props.label ? props.label(i) : i.label}</td>
            <td class="b"><i style={{ width: `${share(i.n, max())}%` }} /></td>
            <td class="num">{i.n}</td>
          </tr>
        )}</For>
        <Show when={props.more}>
          <tr class="more"><td class="l faint" colSpan={3}>and {props.more} {props.moreLabel ?? "more"}</td></tr>
        </Show>
      </tbody>
    </table>
  );
}

/** A count with a bar behind it, in a table's last column. */
export function InlineBar(props: { n: number; max: number }) {
  return (
    <span class="st-inline"><i style={{ width: `${(70 * props.n) / Math.max(1, props.max)}%` }} /><span>{props.n}</span></span>
  );
}

export function Panel(props: { title: string; span?: number; count?: string | number; children: JSX.Element }) {
  return (
    <section class="st-panel" style={{ "grid-column": `span ${props.span ?? 3}` }}>
      <h3>{props.title}<Show when={props.count !== undefined && props.count !== ""}> <span class="count">{props.count}</span></Show></h3>
      {props.children}
    </section>
  );
}

export function Band(props: { title: string; children: JSX.Element }) {
  return (
    <>
      <h2 class="st-band">{props.title}</h2>
      <div class="st-grid">{props.children}</div>
    </>
  );
}
