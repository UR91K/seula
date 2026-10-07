// The projects table. Rows are keyed by project (`For`), so a sort or a page change moves
// the existing row nodes, and an edit to one project touches only its own cells. It takes
// its state as a prop (ADR-0055): the projects view and a collection's tracklist each pass
// their own. The tracklist also passes `tracks`, which adds the place column and the
// drag handles (ADR-0057).

import { For, Show, createMemo, createSignal, onCleanup } from "solid-js";
import { fileName } from "../../../shared/format";
import { COLUMNS, columnById, missingPlugins, missingSamples, nextSort, type Column } from "../../../shared/projects";
import type { Project } from "../../../shared/types";
import { Cb, Icon, TagChip } from "../shell/parts";
import { setShell, shell } from "../shell/shell";
import type { TableState } from "./tablestate";

/** What makes a table a tracklist. */
export interface Tracks {
  /** The new order of the whole collection, after a drag. */
  onReorder(ids: string[]): void;
}

const cell = (n: number) => ({ width: `calc(var(--u) * ${n})` });

function SortTh(props: { t: TableState; id: string; label: string; w: number; num?: boolean }) {
  const sorted = () => props.t.sort()?.col === props.id;
  return (
    <th classList={{ num: props.num, sorted: sorted() }} style={cell(props.w)}
      onClick={() => props.t.sortBy(nextSort(props.t.sort(), props.id))}>
      {props.label}
      <Show when={sorted()}><Icon name={props.t.sort()!.desc ? "arrow_downward" : "arrow_upward"} /></Show>
      <span class="grip" />
    </th>
  );
}

function NameCell(props: { t: TableState; p: Project }) {
  const file = () => fileName(props.p.path);
  let done = false;   // Enter or Escape has settled it; the blur that follows must not commit again
  const finish = (commit: boolean, value: string) => {
    if (done) return;
    done = true;
    if (commit) props.t.renameProject(props.p.id, value);
    else props.t.setRenaming(null);
  };
  return (
    <span class="namecell">
      <Show when={props.t.renaming() === props.p.id}
        fallback={<>
          <span class="n">{props.p.name}</span>
          <Show when={file() !== props.p.name}><span class="f">{file()}</span></Show>
          <span class="edit" title="Rename (F2)"
            onClick={(e) => { e.stopPropagation(); props.t.selectOnly(props.p.id); props.t.setRenaming(props.p.id); }}>
            <Icon name="edit" />
          </span>
        </>}>
        <input value={props.p.name}
          ref={(el) => queueMicrotask(() => { el.focus(); el.select(); })}
          onClick={(e) => e.stopPropagation()}
          onKeyDown={(e) => {
            if (e.key === "Enter") finish(true, e.currentTarget.value);
            else if (e.key === "Escape") finish(false, "");
          }}
          onBlur={(e) => finish(true, e.currentTarget.value)} />
      </Show>
    </span>
  );
}

function DataCell(props: { t: TableState; p: Project; c: Column }) {
  const missing = () => props.c.count === "plugins" ? missingPlugins(props.p) : props.c.count === "samples" ? missingSamples(props.p) : 0;
  const hot = () => props.t.hot()?.id === props.p.id && props.t.hot()?.kind === props.c.count;
  return (
    <td classList={{ dim: props.c.dim, num: props.c.num, count: !!props.c.count, hot: hot() }}
      data-hover={props.c.count}
      onMouseEnter={() => {
        if (props.c.count && props.c.text(props.p, shell.spelling)) { props.t.holdHover(); props.t.setHot({ id: props.p.id, kind: props.c.count }); }
      }}
      onMouseLeave={() => { if (props.c.count) props.t.releaseHover(); }}>
      {props.c.text(props.p, shell.spelling)}
      <Show when={missing()}><span class="miss" title={`${missing()} missing`}><Icon name="error" /></span></Show>
    </td>
  );
}

/** A tracklist's place column: the handle (in collection order) and the number. */
function PositionCell(props: { n: number; draggable: boolean; onGrab(e: PointerEvent): void }) {
  return (
    <td class="lead pos" onClick={(e) => { if ((e.target as HTMLElement).closest(".handle")) e.stopPropagation(); }}>
      <span class="posw">
        <Show when={props.draggable}>
          <span class="handle" title="Drag to reorder" onPointerDown={(e) => props.onGrab(e)}><Icon name="drag_indicator" /></span>
        </Show>
        <span class="n">{props.n}</span>
      </span>
    </td>
  );
}

function Row(props: {
  t: TableState; p: Project; cols: Column[]; place?: { n: number; draggable: boolean; dragging: boolean; dropAbove: boolean;
    onGrab(e: PointerEvent): void };
}) {
  return (
    <tr data-id={props.p.id}
      classList={{ sel: props.t.isSelected(props.p.id), dragging: props.place?.dragging, "drop-above": props.place?.dropAbove }}
      onClick={(e) => props.t.rowClick(props.p.id, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey })}
      onContextMenu={(e) => {
        e.preventDefault();
        props.t.selectOnly(props.p.id);
        const win = (e.currentTarget as HTMLElement).closest(".win")!.getBoundingClientRect();
        props.t.setHot(null);
        setShell({ popover: null, menu: { id: props.p.id, x: e.clientX - win.left, y: e.clientY - win.top } });
      }}>
      <Show when={props.place}>{(pl) => <PositionCell n={pl().n} draggable={pl().draggable} onGrab={pl().onGrab} />}</Show>
      <td class="lead check" onClick={(e) => { e.stopPropagation(); props.t.rowCheck(props.p.id); }}><Cb state={props.t.isSelected(props.p.id)} /></td>
      <td class="lead">
        <Show when={props.p.audio_file_id} fallback={<span class="addaudio" title="Add audition audio"><Icon name="add" /></span>}>
          <span class="play" title="Play audition audio"><Icon name="play_arrow" fill /></span>
        </Show>
      </td>
      <td><NameCell t={props.t} p={props.p} /></td>
      <td><For each={props.p.tags}>{(t) => <TagChip name={t.name} />}</For></td>
      <For each={props.cols}>{(c) => <DataCell t={props.t} p={props.p} c={c} />}</For>
    </tr>
  );
}

export function ProjectTable(props: { t: TableState; tracks?: Tracks }) {
  const t = () => props.t;
  const cols = createMemo(() => t().columns().map(columnById).sort((a, b) => COLUMNS.indexOf(a) - COLUMNS.indexOf(b)));
  const head = createMemo(() => {
    const ids = t().rowIds();
    const on = ids.filter(t().isSelected).length;
    return ids.length && on === ids.length ? true : on ? "mixed" : false;
  });

  // ---------------------------------------------------------------- reordering (ADR-0057)

  /** The collection's order, which is the order the rows arrived in. */
  const order = () => t().items.map((p) => p.id);
  const position = createMemo(() => new Map(order().map((id, i) => [id, i + 1])));
  const draggable = () => t().sort() === null;
  /** The row being dragged, and the row it would land above (null: after the last). */
  const [drag, setDrag] = createSignal<{ id: string; before: string | null } | null>(null);

  const grab = (id: string) => (e: PointerEvent) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    const handle = e.currentTarget as HTMLElement;
    handle.setPointerCapture(e.pointerId);
    setDrag({ id, before: id });
    const move = (ev: PointerEvent) => {
      // The row under the pointer; the pointer is captured, so ask the document.
      const row = document.elementFromPoint(ev.clientX, ev.clientY)?.closest<HTMLElement>("tbody tr[data-id]");
      if (!row) return;
      const box = row.getBoundingClientRect();
      const after = ev.clientY > box.top + box.height / 2;
      const ids = t().rowIds();
      const at = ids.indexOf(row.dataset.id!);
      const before = after ? (ids[at + 1] ?? null) : ids[at];
      setDrag({ id, before });
    };
    const end = (commit: boolean) => {
      handle.removeEventListener("pointermove", move);
      handle.removeEventListener("pointerup", up);
      handle.removeEventListener("pointercancel", cancel);
      const d = drag();
      setDrag(null);
      if (!commit || !d || d.before === d.id) return;
      const rest = order().filter((x) => x !== id);
      const to = d.before == null ? rest.length : rest.indexOf(d.before);
      const next = [...rest.slice(0, to), id, ...rest.slice(to)];
      if (next.some((x, i) => x !== order()[i])) props.tracks?.onReorder(next);
    };
    const up = () => end(true);
    const cancel = () => end(false);
    handle.addEventListener("pointermove", move);
    handle.addEventListener("pointerup", up);
    handle.addEventListener("pointercancel", cancel);
  };

  onCleanup(() => setDrag(null));

  return (
    <table class="grid fixed" classList={{ tracks: !!props.tracks }}>
      <thead>
        <tr>
          <Show when={props.tracks}>
            <th class="lead pos" classList={{ sorted: t().sort() === null }} style={cell(22)}
              title="Collection order" onClick={() => t().sortBy(null)}>#</th>
          </Show>
          <th class="lead check" style={cell(11)} title="Select all on this page" onClick={() => t().checkAll()}><Cb state={head()} /></th>
          <th class="lead" style={cell(10)} title="Audition audio" />
          <SortTh t={t()} id="name" label="Name" w={100} />
          <SortTh t={t()} id="tags" label="Tags" w={64} />
          <For each={cols()}>{(c) => <SortTh t={t()} id={c.id} label={c.label} w={c.w} num={c.num} />}</For>
        </tr>
      </thead>
      <tbody>
        <For each={t().rows()}>{(p) => (
          <Row t={t()} p={p} cols={cols()}
            place={props.tracks && {
              get n() { return position().get(p.id) ?? 0; },
              get draggable() { return draggable(); },
              get dragging() { return drag()?.id === p.id; },
              get dropAbove() { return drag() != null && drag()!.before === p.id && drag()!.id !== p.id; },
              onGrab: grab(p.id),
            }} />
        )}</For>
        <Show when={drag() && drag()!.before === null}><tr class="drop-end"><td colspan={99} /></tr></Show>
      </tbody>
    </table>
  );
}
