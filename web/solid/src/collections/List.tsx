// The list in its two layouts: a grid of cover cards, or a details table. Rows are the
// store's own proxies, so `For` keeps a card's nodes across a sort or a reload.

import { For, Show } from "solid-js";
import { COLLECTION_COLUMNS, cardMeta } from "../../../shared/collections";
import type { CollectionRow } from "../../../shared/types";
import { Icon } from "../shell/parts";
import { setShell } from "../shell/shell";
import { Cover } from "./parts";
import { cui, collections, openCollection, renameCollection, select, setCui, sortByColumn } from "./state";

/** Select on a click, and open the row's menu where the pointer is. */
function menuAt(e: MouseEvent, id: string) {
  e.preventDefault();
  select(id);
  const win = (e.currentTarget as HTMLElement).closest(".win")!.getBoundingClientRect();
  setShell({ popover: null, menu: { id, x: e.clientX - win.left, y: e.clientY - win.top } });
}

function RenameInput(props: { c: CollectionRow }) {
  let done = false;   // Enter or Escape has settled it; the blur that follows must not commit again
  const finish = (commit: boolean, value: string) => {
    if (done) return;
    done = true;
    if (commit) renameCollection(props.c.id, value);
    else setCui("renaming", null);
  };
  return (
    <span class="namecell">
      <input value={props.c.name}
        ref={(el) => queueMicrotask(() => { el.focus(); el.select(); })}
        onClick={(e) => e.stopPropagation()}
        onKeyDown={(e) => {
          if (e.key === "Enter") finish(true, e.currentTarget.value);
          else if (e.key === "Escape") finish(false, "");
        }}
        onBlur={(e) => finish(true, e.currentTarget.value)} />
    </span>
  );
}

function Card(props: { c: CollectionRow }) {
  return (
    <div class="card" classList={{ sel: cui.selected === props.c.id }} data-card={props.c.id}
      onClick={() => select(props.c.id)} onDblClick={() => openCollection(props.c.id)}
      onContextMenu={(e) => menuAt(e, props.c.id)}>
      <div class="art"><Cover c={props.c} /></div>
      <Show when={cui.renaming === props.c.id} fallback={<div class="cname" title={props.c.name}>{props.c.name}</div>}>
        <RenameInput c={props.c} />
      </Show>
      <div class="cmeta">{cardMeta(props.c)}</div>
    </div>
  );
}

export function Grid() {
  return <div class="cards"><For each={collections}>{(c) => <Card c={c} />}</For></div>;
}

const cell = (n: number) => ({ width: `calc(var(--u) * ${n})` });

function Row(props: { c: CollectionRow }) {
  return (
    <tr data-card={props.c.id} classList={{ sel: cui.selected === props.c.id }}
      onClick={() => select(props.c.id)} onDblClick={() => openCollection(props.c.id)}
      onContextMenu={(e) => menuAt(e, props.c.id)}>
      <td class="lead"><Cover c={props.c} class="thumb" /></td>
      <For each={COLLECTION_COLUMNS}>{(col) => (
        <td classList={{ dim: col.dim, num: col.num }}>
          <Show when={col.id === "name"} fallback={col.text!(props.c)}>
            <Show when={cui.renaming === props.c.id} fallback={<span class="n">{props.c.name}</span>}>
              <RenameInput c={props.c} />
            </Show>
          </Show>
        </td>
      )}</For>
    </tr>
  );
}

export function Details() {
  // A search answers by relevance, so the headers show no sort and a click still sorts the list.
  const sorted = (key?: string) => !cui.query && !!key && cui.sort.col === key;
  return (
    <table class="grid fixed collections">
      <thead>
        <tr>
          <th class="lead" style={cell(12)} title="Cover" />
          <For each={COLLECTION_COLUMNS}>{(c) => (
            <th classList={{ num: c.num, sorted: sorted(c.key) }} style={cell(c.w)}
              onClick={() => { if (c.key) sortByColumn(c.key); }}>
              {c.label}
              <Show when={sorted(c.key)}><Icon name={cui.sort.desc ? "arrow_downward" : "arrow_upward"} /></Show>
              <span class="grip" />
            </th>
          )}</For>
        </tr>
      </thead>
      <tbody><For each={collections}>{(c) => <Row c={c} />}</For></tbody>
    </table>
  );
}
